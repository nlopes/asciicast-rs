use std::collections::BTreeMap;

#[path = "../benches/support/mod.rs"]
mod support;

type Event = (f64, String, serde_json::Value);

fn events(bytes: &[u8]) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    std::str::from_utf8(bytes)?
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

#[test]
fn original_benchmark_inputs_keep_their_hashes() -> Result<(), Box<dyn std::error::Error>> {
    for (version, style, hash) in [
        (
            2,
            "mixed",
            "476a83abd3eba395a9edc584f8968b3f42f225c95ea25d46c00e7cd0f43e49d9",
        ),
        (
            3,
            "mixed",
            "55cf775f54924052248c06ced45d666ae1adfd392a18f054d43daf2220e53c4c",
        ),
        (
            2,
            "unicode",
            "51231d709fee93149d356a2eb3aa3e26857644c3667744698dd133d60a4deb62",
        ),
        (
            3,
            "unicode",
            "a02036269475cb457cf56365bd87db444eec2ad33ae58bfb48d76bc1dee54b07",
        ),
    ] {
        assert_eq!(
            support::digest(&support::recording(version, 10_000, style)?),
            hash
        );
    }
    Ok(())
}

#[test]
fn additional_workloads_preserve_their_intended_semantics() -> Result<(), Box<dyn std::error::Error>>
{
    let cases: BTreeMap<_, _> = support::additional_recordings()?
        .into_iter()
        .map(|case| (case.name.clone(), case))
        .collect();
    assert_eq!(cases.len(), 16);
    let case = |name: &str| {
        cases
            .get(name)
            .ok_or_else(|| std::io::Error::other(format!("missing {name}")))
    };
    for version in [2, 3] {
        let decimal = events(&case(&format!("v{version}-varied-time"))?.bytes)?;
        let exponent = events(&case(&format!("v{version}-exponent-time"))?.bytes)?;
        assert_eq!(decimal, exponent);
        let times: Vec<_> = decimal.iter().map(|event| event.0).collect();
        assert!(times.contains(&0.0));
        assert!(times.contains(&0.000_001));
        if version == 2 {
            assert!(
                times
                    .windows(2)
                    .all(|pair| matches!(pair, [a, b] if a <= b))
            );
            assert!(times.last().is_some_and(|time| *time > 3600.0));
        } else {
            assert!(times.contains(&12.5));
            assert!(times.windows(2).any(|pair| matches!(pair, [a, b] if a > b)));
        }
        for size in [1, 80, 16_384] {
            let recording = case(&format!("v{version}-output-{size}"))?;
            for (_, code, data) in events(&recording.bytes)? {
                assert_eq!(code, "o");
                assert_eq!(data.as_str().map(str::len), Some(size));
            }
        }
        let escaped = case(&format!("v{version}-unicode-escaped"))?;
        assert!(escaped.bytes.is_ascii());
        assert!(std::str::from_utf8(&escaped.bytes)?.contains("\\ud83d\\ude80"));
        assert_eq!(
            events(&escaped.bytes)?,
            events(&support::recording(version, 10_000, "unicode")?)?
        );
        let late = case(&format!("v{version}-unicode-late"))?;
        let late_escaped = case(&format!("v{version}-unicode-late-escaped"))?;
        assert!(late_escaped.bytes.is_ascii());
        assert_eq!(events(&late.bytes)?, events(&late_escaped.bytes)?);
        let expected = format!("{}🚀", "x".repeat(4096));
        assert!(
            events(&late.bytes)?
                .iter()
                .all(|(_, _, data)| data.as_str() == Some(expected.as_str()))
        );
    }
    for case in cases.values() {
        assert_eq!(events(&case.bytes)?.len(), case.events);
        for api in ["eager", "auto", "stream"] {
            assert_eq!(
                support::parse(case.version, api, case.bytes.as_slice())?,
                case.events,
                "{} / {api}",
                case.name
            );
        }
    }
    Ok(())
}

#[test]
fn local_corpus_preserves_captured_output_and_timing() -> Result<(), Box<dyn std::error::Error>> {
    // Loading verifies every fixture's pinned hash and byte count.
    let cases = support::local_recordings()?;
    assert_eq!(cases.len(), 6);
    for case in &cases {
        let header = case
            .bytes
            .split(|byte| *byte == b'\n')
            .next()
            .ok_or_else(|| std::io::Error::other(format!("missing header in {}", case.name)))?;
        let header: serde_json::Value = serde_json::from_slice(header)?;
        assert_eq!(
            header.get("version").and_then(serde_json::Value::as_u64),
            Some(u64::from(case.version))
        );
        assert_eq!(events(&case.bytes)?.len(), case.events);
        for api in ["eager", "auto", "stream"] {
            assert_eq!(
                support::parse(case.version, api, case.bytes.as_slice())?,
                case.events
            );
            // Exercise split escape sequences and UTF-8 in the captured data.
            let reader = std::io::BufReader::with_capacity(7, case.bytes.as_slice());
            assert_eq!(support::parse(case.version, api, reader)?, case.events);
        }
    }
    for workload in ["typing", "logs", "dashboard"] {
        let find = |version| {
            cases
                .iter()
                .find(|case| case.name == format!("local-{workload}-v{version}.cast"))
                .ok_or_else(|| std::io::Error::other(format!("missing {workload} v{version}")))
        };
        let v2 = events(&find(2)?.bytes)?;
        let v3 = events(&find(3)?.bytes)?;
        assert!(!v2.is_empty());
        assert_eq!(v2.len(), v3.len());
        let mut absolute = 0.0;
        let mut output = String::new();
        for ((at, code, text), (interval, other_code, other_text)) in v2.iter().zip(&v3) {
            assert!(*interval >= 0.0);
            absolute += interval;
            assert!((at - absolute).abs() < 0.000_000_001);
            assert_eq!(code, "o");
            assert_eq!(code, other_code);
            assert_eq!(text, other_text);
            output.push_str(
                text.as_str()
                    .ok_or_else(|| std::io::Error::other("non-text output"))?,
            );
        }
        assert!(output.contains('\u{1b}'));
        assert!(output.contains("\r\n"));
        let ending = match workload {
            "typing" => {
                assert!(output.contains("café 東京 🚀"));
                assert!(output.contains("\x08 \x08"));
                "demo $ exit\r\n"
            }
            "logs" => "Build complete: 1024 modules\r\n",
            _ => {
                assert!(output.contains("\x1b[?1049h"));
                assert!(output.contains('█'));
                "\x1b[?25h\x1b[?1049lDashboard complete\r\n"
            }
        };
        assert!(output.ends_with(ending));
    }
    Ok(())
}
