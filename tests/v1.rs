use asciicast_rs::v1::Frame;
use asciicast_rs::{Asciicast, Error, V1};

const V1_PRETTY: &str = include_str!("fixtures/v1.cast");
const V1_MINIFIED: &str = include_str!("fixtures/v1_minified.cast");
const V2_CAST: &str = include_str!("fixtures/v2.cast");

#[test]
fn parses_v1_header_and_frames() -> Result<(), Error> {
    let cast = Asciicast::<V1>::from_slice(V1_PRETTY.as_bytes())?;

    assert_eq!(cast.header.version, 1);
    assert_eq!(cast.header.width, 80);
    assert_eq!(cast.header.height, 24);
    assert_eq!(cast.header.duration.to_bits(), 5.5_f64.to_bits());
    assert_eq!(cast.header.command.as_deref(), Some("/bin/bash"));
    assert_eq!(cast.header.title.as_deref(), Some("v1 demo"));
    assert_eq!(
        cast.header
            .env
            .as_ref()
            .and_then(|env| env.get("TERM"))
            .map(String::as_str),
        Some("xterm-256color")
    );

    assert_eq!(
        cast.events,
        vec![
            Frame {
                delay: 1.0,
                data: "hello".to_owned(),
            },
            Frame {
                delay: 0.5,
                data: "world\n".to_owned(),
            },
        ]
    );
    Ok(())
}

#[test]
fn minified_equals_pretty() -> Result<(), Error> {
    let pretty = Asciicast::<V1>::from_slice(V1_PRETTY.as_bytes())?;
    let minified = Asciicast::<V1>::from_slice(V1_MINIFIED.as_bytes())?;
    assert_eq!(pretty, minified);
    Ok(())
}

#[test]
fn wrong_version_is_rejected() {
    assert!(Asciicast::<V1>::from_slice(V2_CAST.as_bytes()).is_err());
}
