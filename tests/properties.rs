use std::{
    fmt::Write as _,
    io::{BufReader, Cursor},
};

use asciicast_rs::{Asciicast, AsciicastVersioned, Error, Streamable, V1, V2, V3, v1, v2, v3};
use proptest::{
    prelude::*,
    test_runner::{FileFailurePersistence, RngSeed},
};
use serde_json::Value;

fn text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..80).prop_map(|chars| chars.into_iter().collect())
}

fn json_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::from),
        (-1000_i64..1000).prop_map(Value::from),
        text().prop_map(Value::from)
    ]
    .prop_recursive(3, 32, 8, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..8).prop_map(Value::Array),
            prop::collection::btree_map("[a-z]{1,8}", inner, 0..8)
                .prop_map(|map| Value::Object(map.into_iter().collect()))
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        rng_seed: RngSeed::Fixed(0x2026_1003),
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct("tests/proptest-regressions.txt"))),
        ..ProptestConfig::default()
    })]

    #[test]
    fn v1_matches_generated_frames(frames in prop::collection::vec((0_u32..10_000, text()), 0..30),
        capacity in 1_usize..128, pretty in any::<bool>()) {
        let expected: Vec<_> = frames.into_iter().map(|(ticks, data)| v1::Frame { delay:f64::from(ticks)/1024.0, data }).collect();
        let stdout: Vec<_> = expected.iter().map(|frame| (frame.delay, &frame.data)).collect();
        let document = serde_json::json!({"version":1,"width":80,"height":24,"stdout":stdout});
        let bytes = if pretty { serde_json::to_vec_pretty(&document)? } else { serde_json::to_vec(&document)? };
        let cast = Asciicast::<V1>::from_reader(BufReader::with_capacity(capacity, Cursor::new(&bytes)))?;
        prop_assert_eq!(&cast.events, &expected);
        let detected = AsciicastVersioned::from_slice(&bytes)?;
        prop_assert!(matches!(detected, AsciicastVersioned::V1(ref cast) if cast.events == expected));
        let actual: Vec<_> = cast.absolute_times().map(|(time, _)| time).collect();
        let mut elapsed = 0.0;
        let times: Vec<_> = expected.iter().map(|frame| { elapsed += frame.delay; elapsed }).collect();
        prop_assert_eq!(actual, times);
    }

    #[test]
    fn v2_matches_generated_output(events in prop::collection::vec((0_u32..10_000, text()), 0..30),
        capacity in 1_usize..128, crlf in any::<bool>(), final_newline in any::<bool>()) {
        let expected: Vec<_> = events.into_iter().map(|(ticks, data)| v2::Event {
            time:f64::from(ticks)/1024.0, payload:v2::EventPayload::Output(data) }).collect();
        let ending = if crlf { "\r\n" } else { "\n" };
        let mut lines = vec![r#"{"version":2,"width":80,"height":24}"#.to_owned()];
        for event in &expected { lines.push(serde_json::to_string(&(event.time, "o", event.as_output()))?); }
        let mut input = lines.join(ending);
        if final_newline { input.push_str(ending); }
        let cast = Asciicast::<V2>::from_reader(BufReader::with_capacity(capacity, input.as_bytes()))?;
        prop_assert_eq!(&cast.events, &expected);
        let stream = v2::stream(BufReader::with_capacity(capacity, input.as_bytes()))?;
        prop_assert_eq!(stream.collect::<Result<Vec<_>,_>>()?, expected);
        prop_assert!(matches!(AsciicastVersioned::from_slice(input.as_bytes())?, AsciicastVersioned::V2(ref value) if value.events == cast.events));
    }

    #[test]
    fn v3_matches_generated_output(events in prop::collection::vec((0_u32..10_000, text()), 0..30),
        capacity in 1_usize..128, comments in any::<bool>(), final_newline in any::<bool>()) {
        let expected: Vec<_> = events.into_iter().map(|(ticks, data)| v3::Event {
            interval:f64::from(ticks)/1024.0, payload:v3::EventPayload::Output(data) }).collect();
        let mut lines = vec![r#"{"version":3,"term":{"cols":80,"rows":24}}"#.to_owned()];
        for event in &expected {
            if comments { lines.extend(["# ignored".to_owned(), " \t".to_owned()]); }
            lines.push(serde_json::to_string(&(event.interval, "o", event.as_output()))?);
        }
        let mut input = lines.join("\r\n");
        if final_newline { input.push_str("\r\n"); }
        let cast = Asciicast::<V3>::from_reader(BufReader::with_capacity(capacity, input.as_bytes()))?;
        prop_assert_eq!(&cast.events, &expected);
        prop_assert_eq!(v3::stream(input.as_bytes())?.collect::<Result<Vec<_>,_>>()?, expected);
        let mut elapsed = 0.0;
        let times: Vec<_> = cast.events.iter().map(|event| { elapsed += event.interval; elapsed }).collect();
        let actual = v3::stream(input.as_bytes())?.absolute_times().map(|event| event.map(|(time, _)| time)).collect::<Result<Vec<_>,_>>()?;
        prop_assert_eq!(actual, times);
        prop_assert!(matches!(AsciicastVersioned::from_slice(input.as_bytes())?, AsciicastVersioned::V3(ref value) if value.events == cast.events));
    }

    #[test]
    fn unknown_v2_preserves_json(data in json_value(), ticks in 0_u32..10_000) {
        let time = f64::from(ticks)/1024.0;
        let line = serde_json::to_string(&(time, "future", &data))?;
        let event = V2::parse_event(&line)?;
        prop_assert_eq!(event.time.to_bits(), time.to_bits());
        prop_assert!(matches!(event.payload, v2::EventPayload::Unknown { ref code, data: ref actual } if code == "future" && actual == &data), "unknown JSON payload changed");
    }

    #[test]
    fn escaped_codes_and_payloads_preserve_values(data in text(), code in prop::sample::select(vec!["o","i","m","future"])) {
        let mut escaped = String::new();
        for character in code.chars() { write!(&mut escaped, "\\u{:04x}", u32::from(character))?; }
        let line = format!("[0.125,\"{escaped}\",{}]", serde_json::to_string(&data)?);
        let v2 = V2::parse_event(&line)?;
        let v3 = V3::parse_event(&line)?;
        match code {
            "o" => { prop_assert_eq!(v2.as_output(), Some(data.as_str())); prop_assert_eq!(v3.as_output(), Some(data.as_str())); }
            "i" => { prop_assert_eq!(v2.as_input(), Some(data.as_str())); prop_assert_eq!(v3.as_input(), Some(data.as_str())); }
            "m" => { prop_assert_eq!(v2.as_marker(), Some(data.as_str())); prop_assert_eq!(v3.as_marker(), Some(data.as_str())); }
            _ => {
                prop_assert!(matches!(v2.payload, v2::EventPayload::Unknown { code: ref actual_code, data: ref actual } if actual_code == code && actual == &Value::String(data.clone())), "unknown v2 event changed");
                prop_assert!(matches!(v3.payload, v3::EventPayload::Unknown { code: ref actual_code, data: ref actual } if actual_code == code && actual == &data), "unknown v3 event changed");
            }
        }
    }

    #[test]
    fn structured_v3_payloads_preserve_values(cols in 1_u16..=u16::MAX, rows in 1_u16..=u16::MAX, status in any::<i32>()) {
        let resize = V3::parse_event(&format!("[0.125,\"r\",\"{cols}x{rows}\"]"))?;
        prop_assert_eq!(resize.as_resize(), Some(asciicast_rs::common::Resize { cols, rows }));
        let exit = V3::parse_event(&format!("[0.125,\"x\",\"{status}\"]"))?;
        prop_assert_eq!(exit.as_exit().map(|value| value.code()), Some(status));
    }

    #[test]
    fn recovers_after_invalid_utf8(tail in prop::collection::vec(0_u8..10, 0..40), capacity in 1_usize..128) {
        let mut bytes = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n".to_vec();
        bytes.push(0xff);
        bytes.extend(tail);
        bytes.extend_from_slice(b"\n[0.125,\"o\",\"next\"]\n");
        let mut reader = v3::stream(BufReader::with_capacity(capacity, bytes.as_slice()))?;
        prop_assert!(matches!(reader.next(), Some(Err(Error::Io(_)))));
        let next = reader.next().transpose()?;
        prop_assert_eq!(next.as_ref().and_then(|event| event.as_output()), Some("next"));
        prop_assert!(reader.next().is_none());
    }

    #[test]
    fn arbitrary_input_does_not_panic(bytes in prop::collection::vec(any::<u8>(), 0..512), capacity in 1_usize..128) {
        let reader = || BufReader::with_capacity(capacity, bytes.as_slice());
        let _ = AsciicastVersioned::from_reader(reader());
        let _ = Asciicast::<V1>::from_reader(reader());
        let _ = Asciicast::<V2>::from_reader(reader());
        let _ = Asciicast::<V3>::from_reader(reader());
    }

    #[test]
    fn output_events_match_reference_json(time in prop::sample::select(vec![
        "0", "0.001", "-0", "-1", "1e-323", "-1e-999", "9007199254740993",
        "1.234567890123456789", "1e999", "01", "+1", "null", "\"0\""
    ]), data in text(), suffix in prop::sample::select(vec!["", "\r\n", " trailing"]),
        malformed in prop::sample::select(vec![None, Some("\"\\ud800\""), Some("\"\\udfff\""), Some("null"), Some("42"), Some("\"\\q\"")])) {
        let payload = malformed.map_or_else(|| serde_json::to_string(&data), |value| Ok(value.to_owned()))?;
        let line = format!("[{time},\"o\",{payload}]{suffix}");
        let reference: Result<(f64, String, String),_> = serde_json::from_str(&line);
        let expected = reference.as_ref().ok().filter(|(time, _, _)| !time.is_sign_negative())
            .map(|(time, _, data)| (time.to_bits(), data.as_str()));
        let v2 = V2::parse_event(&line);
        let v3 = V3::parse_event(&line);
        prop_assert_eq!(v2.as_ref().ok().and_then(|event| event.as_output().map(|data| (event.time.to_bits(), data))), expected);
        prop_assert_eq!(v3.as_ref().ok().and_then(|event| event.as_output().map(|data| (event.interval.to_bits(), data))), expected);
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn compressed_and_concatenated_frames_preserve_events(data in text(), capacity in 1_usize..128) {
        let input = format!("{{\"version\":3,\"term\":{{\"cols\":80,\"rows\":24}}}}\n[0.125,\"o\",{}]\n", serde_json::to_string(&data)?);
        let middle = input.len()/2;
        let (left, right) = input.as_bytes().split_at(middle);
        let mut compressed = vec![0x50,0x2a,0x4d,0x18,0,0,0,0];
        compressed.extend(ruzstd::encoding::compress_to_vec(left, ruzstd::encoding::CompressionLevel::Fastest));
        compressed.extend(ruzstd::encoding::compress_to_vec(right, ruzstd::encoding::CompressionLevel::Fastest));
        let cast = Asciicast::<V3>::from_reader(BufReader::with_capacity(capacity, compressed.as_slice()))?;
        prop_assert_eq!(cast.events.len(), 1);
        prop_assert_eq!(cast.events.first().and_then(|event| event.as_output()), Some(data.as_str()));
    }
}

#[test]
fn all_single_utf16_escapes_match_unicode_scalars() -> Result<(), Box<dyn std::error::Error>> {
    for scalar in 0..=u32::from(u16::MAX) {
        let line = format!("[0.125,\"o\",\"\\u{scalar:04x}\"]");
        if let Some(character) = char::from_u32(scalar) {
            let expected = character.to_string();
            assert_eq!(V2::parse_event(&line)?.as_output(), Some(expected.as_str()));
            assert_eq!(V3::parse_event(&line)?.as_output(), Some(expected.as_str()));
        } else {
            assert!(V2::parse_event(&line).is_err());
            assert!(V3::parse_event(&line).is_err());
        }
    }
    Ok(())
}
