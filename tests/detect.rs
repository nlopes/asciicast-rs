use asciicast_rs::{Asciicast, AsciicastVersioned, Error, V1, V2, V3};

const V1_PRETTY: &str = include_str!("fixtures/v1.cast");
const V1_MINIFIED: &str = include_str!("fixtures/v1_minified.cast");
const V2_CAST: &str = include_str!("fixtures/v2.cast");
const V3_CAST: &str = include_str!("fixtures/v3.cast");

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
