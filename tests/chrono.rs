#![cfg(feature = "chrono")]

use asciicast_rs::{Asciicast, Error, V2, V3};

const V2_CAST: &str = include_str!("fixtures/v2.cast");
const V3_CAST: &str = include_str!("fixtures/v3.cast");

#[test]
fn v2_timestamp_datetime() -> Result<(), Error> {
    let cast = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(
        cast.header.timestamp_datetime().map(|dt| dt.timestamp()),
        Some(1_504_467_315)
    );
    Ok(())
}

#[test]
fn v3_timestamp_datetime() -> Result<(), Error> {
    let cast = Asciicast::<V3>::from_slice(V3_CAST.as_bytes())?;
    assert_eq!(
        cast.header.timestamp_datetime().map(|dt| dt.timestamp()),
        Some(1_700_000_000)
    );
    Ok(())
}
