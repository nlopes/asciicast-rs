use asciicast_rs::{Asciicast, AsciicastVersioned, Error, V1, V2, V3};

const V1_PRETTY: &str = include_str!("fixtures/v1.cast");
const V1_MINIFIED: &str = include_str!("fixtures/v1_minified.cast");
const V2_CAST: &str = include_str!("fixtures/v2.cast");
const V3_CAST: &str = include_str!("fixtures/v3.cast");

#[test]
fn detects_versions_with_leading_blank_lines() -> Result<(), Error> {
    for recording in [V1_PRETTY, V1_MINIFIED, V2_CAST, V3_CAST] {
        let prefixed = format!("\n \t\r\n{recording}");
        assert_eq!(
            AsciicastVersioned::from_slice(prefixed.as_bytes())?,
            AsciicastVersioned::from_slice(recording.as_bytes())?
        );
    }
    Ok(())
}

#[test]
fn detects_v1_pretty() -> Result<(), Error> {
    let detected = AsciicastVersioned::from_slice(V1_PRETTY.as_bytes())?;
    let typed = Asciicast::<V1>::from_slice(V1_PRETTY.as_bytes())?;
    assert_eq!(detected, AsciicastVersioned::V1(typed));
    Ok(())
}

#[test]
fn detects_v1_minified() -> Result<(), Error> {
    let detected = AsciicastVersioned::from_slice(V1_MINIFIED.as_bytes())?;
    assert!(matches!(detected, AsciicastVersioned::V1(_)));
    Ok(())
}

#[test]
fn detected_v1_rejects_content_after_the_first_line() {
    for trailing in ["\n[]", "\n{}", "\nnot json"] {
        let input = format!("{V1_MINIFIED}{trailing}");
        assert!(matches!(
            AsciicastVersioned::from_slice(input.as_bytes()),
            Err(Error::Json(_))
        ));
    }
}

#[test]
fn detects_v2() -> Result<(), Error> {
    let detected = AsciicastVersioned::from_slice(V2_CAST.as_bytes())?;
    let typed = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(detected, AsciicastVersioned::V2(typed));
    Ok(())
}

#[test]
fn detects_v3() -> Result<(), Error> {
    let detected = AsciicastVersioned::from_slice(V3_CAST.as_bytes())?;
    let typed = Asciicast::<V3>::from_slice(V3_CAST.as_bytes())?;
    assert_eq!(detected, AsciicastVersioned::V3(typed));
    Ok(())
}

#[test]
fn from_path_detects_version() -> Result<(), Error> {
    let detected = AsciicastVersioned::from_path("tests/fixtures/v3.cast")?;
    assert!(matches!(detected, AsciicastVersioned::V3(_)));
    Ok(())
}

#[test]
fn unknown_version_is_rejected() {
    let bad = r#"{"version": 9, "width": 80, "height": 24}"#;
    assert!(matches!(
        AsciicastVersioned::from_slice(bad.as_bytes()),
        Err(Error::UnknownVersion(9))
    ));
}

#[test]
fn detects_versions_with_unknown_events() -> Result<(), Error> {
    let v2_data =
        b"{\"version\":2,\"width\":80,\"height\":24}\n[1.0,\"overlay\",{\"text\":\"hello\"}]\n";
    let v2 = Asciicast::<V2>::from_slice(v2_data)?;
    assert_eq!(
        AsciicastVersioned::from_slice(v2_data)?,
        AsciicastVersioned::V2(v2)
    );

    let v3_data =
        b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n[0.5,\"subtitle\",\"hello\"]\n";
    let v3 = Asciicast::<V3>::from_slice(v3_data)?;
    assert_eq!(
        AsciicastVersioned::from_slice(v3_data)?,
        AsciicastVersioned::V3(v3)
    );
    Ok(())
}
