use asciicast_rs::{Asciicast, AsciicastVersioned, Error, Streamable, V2, V3};

#[test]
fn empty_input_is_missing_header() {
    assert!(matches!(
        Asciicast::<V2>::from_slice(b""),
        Err(Error::MissingHeader)
    ));
}

#[test]
fn empty_input_detection_errors() {
    assert!(AsciicastVersioned::from_slice(b"").is_err());
}

#[test]
fn header_only_recording_has_no_events() -> Result<(), Error> {
    let cast = Asciicast::<V2>::from_slice(b"{\"version\":2,\"width\":80,\"height\":24}\n")?;
    assert_eq!(cast.events.len(), 0);
    Ok(())
}

#[test]
fn trailing_blank_lines_are_skipped() -> Result<(), Error> {
    let data = b"{\"version\":2,\"width\":80,\"height\":24}\n[0.0,\"o\",\"hi\"]\n\n\n";
    let cast = Asciicast::<V2>::from_slice(data)?;
    assert_eq!(cast.events.len(), 1);
    Ok(())
}

#[test]
fn crlf_line_endings_are_handled() -> Result<(), Error> {
    let data = b"{\"version\":2,\"width\":80,\"height\":24}\r\n[0.0,\"o\",\"hi\"]\r\n";
    let cast = Asciicast::<V2>::from_slice(data)?;
    assert_eq!(cast.header.width, 80);
    assert_eq!(cast.events.len(), 1);
    assert_eq!(
        cast.events.first().and_then(|event| event.as_output()),
        Some("hi")
    );
    Ok(())
}

#[test]
fn detection_handles_crlf() -> Result<(), Error> {
    let data = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\r\n";
    let detected = AsciicastVersioned::from_slice(data)?;
    assert!(matches!(detected, AsciicastVersioned::V3(_)));
    Ok(())
}

#[test]
fn unknown_header_fields_are_ignored() -> Result<(), Error> {
    // The v3 spec states tools MUST ignore header attributes they don't understand.
    let data = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24},\"future_field\":42}\n";
    let cast = Asciicast::<V3>::from_slice(data)?;
    assert_eq!(cast.header.term.cols, 80);
    Ok(())
}

#[test]
fn malformed_event_line_errors() {
    // Two elements instead of the expected [interval, code, data].
    let data = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n[0.0,\"o\"]\n";
    assert!(Asciicast::<V3>::from_slice(data).is_err());
}

#[test]
fn known_v2_events_still_require_string_data() {
    let data = b"{\"version\":2,\"width\":80,\"height\":24}\n[0.0,\"o\",{\"text\":\"hello\"}]\n";
    assert!(Asciicast::<V2>::from_slice(data).is_err());
}

#[test]
fn empty_event_codes_are_rejected() {
    let v2 = b"{\"version\":2,\"width\":80,\"height\":24}\n[0.0,\"\",\"hello\"]\n";
    assert!(Asciicast::<V2>::from_slice(v2).is_err());

    let v3 = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n[0.0,\"\",\"hello\"]\n";
    assert!(Asciicast::<V3>::from_slice(v3).is_err());
}

#[test]
fn comments_are_only_a_v3_feature() {
    // A '#' line is a comment in v3 but invalid JSON in the v2 event stream.
    let v3 = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n# a v3 comment\n";
    assert!(Asciicast::<V3>::from_slice(v3).is_ok());

    let v2 = b"{\"version\":2,\"width\":80,\"height\":24}\n# not valid in v2\n";
    assert!(Asciicast::<V2>::from_slice(v2).is_err());
}

#[test]
fn text_events_reject_invalid_json_string_syntax() {
    for payload in [
        r#""\u00+1""#,
        r#""\u00-1""#,
        r#""\u00xz""#,
        r#""\u00""#,
        r#""trailing\""#,
        r#""interior"quote""#,
        r#""text","extra""#,
        r#""\uD800""#,
        r#""\uDC00""#,
        r#""\q""#,
    ] {
        let line = format!("[0.125,\"o\",{payload}]");
        assert!(V2::parse_event(&line).is_err(), "{line}");
        assert!(V3::parse_event(&line).is_err(), "{line}");
    }
    // Check each control byte at both ends and across vector-sized boundaries.
    for control in '\0'..='\x1f' {
        for offset in [0, 1, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128] {
            let line = format!("[0.125,\"o\",\"{}{control}tail\"]", "x".repeat(offset));
            assert!(V2::parse_event(&line).is_err(), "{line:?}");
            assert!(V3::parse_event(&line).is_err(), "{line:?}");
        }
    }
}

#[test]
fn text_events_accept_json_whitespace_and_escapes() -> Result<(), Error> {
    let line = " \r\n[\t1.25e-1 \n, \"o\"\t, \"\\\"\\\\\\/\\b\\f\\n\\r\\t\\u001B\\u00e9🚀\"\r ]\n ";
    let expected = "\"\\/\x08\x0c\n\r\t\x1bé🚀";
    assert_eq!(V2::parse_event(line)?.as_output(), Some(expected));
    assert_eq!(V3::parse_event(line)?.as_output(), Some(expected));
    Ok(())
}
