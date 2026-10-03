#![allow(dead_code)]

use std::{hint::black_box, io::BufRead};

use asciicast_rs::{Asciicast, AsciicastVersioned, Error, Reader, Streamable, V1, V2, V3, v2, v3};
use serde::de::DeserializeOwned;
use sha2::Digest;

#[derive(serde::Deserialize)]
struct CorpusEntry {
    name: String,
    version: u8,
    events: usize,
    origin: String,
    sha256: String,
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", sha2::Sha256::digest(bytes))
}

fn corpus_recordings() -> Result<Vec<Recording>, Box<dyn std::error::Error>> {
    let Some(directory) = std::env::var_os("ASCIICAST_BENCH_CORPUS") else {
        return Ok(Vec::new());
    };
    let directory = std::path::Path::new(&directory);
    let manifest: Vec<CorpusEntry> =
        serde_json::from_slice(&std::fs::read(directory.join("corpus.json"))?)?;
    let mut cases = Vec::new();
    for row in manifest {
        if row.origin != "downloaded" {
            continue;
        }
        let bytes = std::fs::read(directory.join(&row.name))?;
        if digest(&bytes) != row.sha256 {
            return Err(format!("corpus hash mismatch: {}", row.name).into());
        }
        cases.push(Recording {
            name: format!("real-{}", row.name),
            version: row.version,
            bytes,
            events: row.events,
        });
    }
    Ok(cases)
}

pub struct Recording {
    pub name: String,
    pub version: u8,
    pub bytes: Vec<u8>,
    pub events: usize,
}

pub fn recording(version: u8, events: usize, style: &str) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    let header = if version == 3 {
        serde_json::json!({"version":3,"term":{"cols":120,"rows":40}})
    } else {
        serde_json::json!({"version":version,"width":120,"height":40})
    };
    let mut frames = Vec::new();
    if version != 1 {
        serde_json::to_writer(&mut bytes, &header)?;
        bytes.push(b'\n');
    }
    for index in 0..events {
        let (code, data) = match style {
            "unicode" => ("o", "構築 — café Ελληνικά 🚀\r\n".repeat(4)),
            "structured" if index % 2 == 0 => ("r", "120x40".to_owned()),
            "structured" => ("x", "0".to_owned()),
            _ => match index % 100 {
                0..80 => (
                    "o",
                    format!("\x1b[32m[{index:07}]\x1b[0m build module: completed\r\n"),
                ),
                80..92 => ("i", "abcdef\r\t".to_owned()),
                92..96 => ("m", "chapter".to_owned()),
                96..99 => ("r", "120x40".to_owned()),
                _ => ("future", "extension".to_owned()),
            },
        };
        if version == 1 {
            frames.push((0.001, data));
        } else if style == "unknown" {
            serde_json::to_writer(
                &mut bytes,
                &serde_json::json!([
                    0.001, "future", {"text":data,"values":[null,true,index]}
                ]),
            )?;
            bytes.push(b'\n');
        } else {
            serde_json::to_writer(&mut bytes, &(0.001, code, data))?;
            bytes.push(b'\n');
        }
    }
    if version == 1 {
        let mut document = header;
        if let Some(object) = document.as_object_mut() {
            object.insert("stdout".to_owned(), serde_json::to_value(frames)?);
        }
        serde_json::to_writer(&mut bytes, &document)?;
    }
    Ok(bytes)
}

pub fn long_recording() -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = recording(3, 0, "mixed")?;
    serde_json::to_writer(&mut bytes, &(0.001, "o", "x".repeat(8 * 1024 * 1024)))?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(feature = "zstd")]
pub fn compress(bytes: &[u8]) -> Vec<u8> {
    ruzstd::encoding::compress_to_vec(bytes, ruzstd::encoding::CompressionLevel::Fastest)
}

pub fn recordings() -> Result<Vec<Recording>, Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    for version in 1..=3 {
        let large = if version == 3 { 1_000_000 } else { 100_000 };
        for (label, events) in [
            ("empty", 0),
            ("tiny", 1),
            ("medium", 10_000),
            ("large", large),
        ] {
            cases.push(Recording {
                name: format!("v{version}-{label}"),
                version,
                bytes: recording(version, events, "mixed")?,
                events,
            });
        }
    }
    for (version, style) in [
        (2, "unicode"),
        (3, "unicode"),
        (2, "unknown"),
        (3, "structured"),
    ] {
        cases.push(Recording {
            name: format!("v{version}-{style}"),
            version,
            bytes: recording(version, 10_000, style)?,
            events: 10_000,
        });
    }
    cases.push(Recording {
        name: "v3-long".to_owned(),
        version: 3,
        bytes: long_recording()?,
        events: 1,
    });
    let document: serde_json::Value = serde_json::from_slice(&recording(1, 10_000, "mixed")?)?;
    cases.push(Recording {
        name: "v1-pretty".to_owned(),
        version: 1,
        bytes: serde_json::to_vec_pretty(&document)?,
        events: 10_000,
    });
    let comments = String::from_utf8(recording(3, 10_000, "mixed")?)?
        .replace('\n', "\r\n# comment\r\n \t\r\n");
    cases.push(Recording {
        name: "v3-comments".to_owned(),
        version: 3,
        bytes: format!("\n \t\r\n{comments}").into_bytes(),
        events: 10_000,
    });
    #[cfg(feature = "zstd")]
    {
        let mut compressed = Vec::new();
        for case in &cases {
            if case.events != 0
                && !case.name.ends_with("pretty")
                && !case.name.ends_with("comments")
            {
                compressed.push(Recording {
                    name: format!("{}-zstd", case.name),
                    version: case.version,
                    bytes: compress(&case.bytes),
                    events: case.events,
                });
            }
        }
        cases.extend(compressed);
        let bytes = recording(3, 10_000, "mixed")?;
        let (first, rest) = bytes.split_at(bytes.len() / 2);
        let frames = [compress(first), compress(rest)].concat();
        cases.push(Recording {
            name: "v3-concatenated-zstd".to_owned(),
            version: 3,
            bytes: frames.clone(),
            events: 10_000,
        });
        let skip = [0x50, 0x2a, 0x4d, 0x18, 0, 0, 0, 0];
        cases.push(Recording {
            name: "v3-skippable-zstd".to_owned(),
            version: 3,
            bytes: [skip.as_slice(), frames.as_slice(), skip.as_slice()].concat(),
            events: 10_000,
        });
    }
    cases.extend(corpus_recordings()?);
    Ok(cases)
}

fn stream<V: Streamable, R: BufRead>(mut reader: Reader<V, R>, limit: usize) -> Result<usize, Error>
where
    V::Header: DeserializeOwned,
{
    let mut count = 0;
    while count < limit {
        let Some(event) = reader.next() else { break };
        black_box(event?);
        count += 1;
    }
    Ok(count)
}

pub fn parse<R: BufRead>(version: u8, api: &str, reader: R) -> Result<usize, Error> {
    match (version, api) {
        (1, "eager") => Ok(black_box(Asciicast::<V1>::from_reader(reader)?)
            .events
            .len()),
        (2, "eager") => Ok(black_box(Asciicast::<V2>::from_reader(reader)?)
            .events
            .len()),
        (3, "eager") => Ok(black_box(Asciicast::<V3>::from_reader(reader)?)
            .events
            .len()),
        (_, "auto") => Ok(match black_box(AsciicastVersioned::from_reader(reader)?) {
            AsciicastVersioned::V1(cast) => cast.events.len(),
            AsciicastVersioned::V2(cast) => cast.events.len(),
            AsciicastVersioned::V3(cast) => cast.events.len(),
        }),
        (2, "stream") => stream(v2::stream(reader)?, usize::MAX),
        (3, "stream") => stream(v3::stream(reader)?, usize::MAX),
        (2, "header") => stream(v2::stream(reader)?, 0),
        (3, "header") => stream(v3::stream(reader)?, 0),
        (2, "first") => stream(v2::stream(reader)?, 1),
        (3, "first") => stream(v3::stream(reader)?, 1),
        (3, "times") => {
            let mut count = 0;
            for event in v3::stream(reader)?.absolute_times() {
                black_box(event?);
                count += 1;
            }
            Ok(count)
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "unsupported benchmark API",
        )
        .into()),
    }
}
