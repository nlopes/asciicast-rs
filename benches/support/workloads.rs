use std::{borrow::Cow, fmt::Write as _, io::Write as _};

use super::Recording;

fn timestamp(micros: u64, scientific: bool, index: usize) -> String {
    if scientific {
        match index % 3 {
            0 => format!("{micros}e-6"),
            1 => format!("{micros}E-06"),
            _ => format!("{micros}.0e-6"),
        }
    } else if micros.is_multiple_of(1_000_000) {
        (micros / 1_000_000).to_string()
    } else {
        format!("{}.{:06}", micros / 1_000_000, micros % 1_000_000)
            .trim_end_matches('0')
            .to_owned()
    }
}

fn timed_recording(
    version: u8,
    events: usize,
    scientific: bool,
    output: Option<&str>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = super::recording(version, 0, "mixed")?;
    let mut elapsed = 0;
    for index in 0..events {
        // Bursts, keystrokes, frame updates, and pauses. Integer microseconds
        // make both spellings deterministic without float formatting artifacts.
        let interval = match index % 8 {
            0 => 0,
            1 => 1,
            2 => 137,
            3 => 1000,
            4 => 16_667,
            5 => 123_456,
            6 => 1_000_000,
            _ => 12_500_000,
        };
        elapsed += interval;
        let time = if version == 2 { elapsed } else { interval };
        let (code, data) = output.map_or_else(
            || {
                let (code, data) = super::mixed_payload(index);
                (code, Cow::Owned(data))
            },
            |data| ("o", Cow::Borrowed(data)),
        );
        write!(&mut bytes, "[{},", timestamp(time, scientific, index))?;
        serde_json::to_writer(&mut bytes, code)?;
        bytes.push(b',');
        serde_json::to_writer(&mut bytes, &data)?;
        bytes.extend_from_slice(b"]\n");
    }
    Ok(bytes)
}

fn escape_unicode(bytes: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut escaped = String::new();
    for character in std::str::from_utf8(bytes)?.chars() {
        if character.is_ascii() {
            escaped.push(character);
        } else {
            // Non-ASCII only occurs inside JSON strings in these generated
            // inputs. Encode supplementary characters as UTF-16 surrogate pairs.
            for unit in character.encode_utf16(&mut [0; 2]) {
                write!(&mut escaped, "\\u{unit:04x}")?;
            }
        }
    }
    Ok(escaped.into_bytes())
}

pub fn additional_recordings() -> Result<Vec<Recording>, Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    for version in [2, 3] {
        for (style, scientific) in [("varied-time", false), ("exponent-time", true)] {
            cases.push(Recording {
                name: format!("v{version}-{style}"),
                version,
                bytes: timed_recording(version, 10_000, scientific, None)?,
                events: 10_000,
            });
        }
        for (size, events) in [(1, 10_000), (80, 10_000), (16_384, 512)] {
            cases.push(Recording {
                name: format!("v{version}-output-{size}"),
                version,
                bytes: timed_recording(version, events, false, Some(&"a".repeat(size)))?,
                events,
            });
        }
        cases.push(Recording {
            name: format!("v{version}-unicode-escaped"),
            version,
            bytes: escape_unicode(&super::recording(version, 10_000, "unicode")?)?,
            events: 10_000,
        });
        let data = format!("{}🚀", "x".repeat(4096));
        let bytes = timed_recording(version, 1024, false, Some(&data))?;
        cases.push(Recording {
            name: format!("v{version}-unicode-late-escaped"),
            version,
            bytes: escape_unicode(&bytes)?,
            events: 1024,
        });
        cases.push(Recording {
            name: format!("v{version}-unicode-late"),
            version,
            bytes,
            events: 1024,
        });
    }
    Ok(cases)
}
