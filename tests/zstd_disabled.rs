//! Without the `zstd` feature, compressed input must not be silently decoded.
//!
//! Run by the `--no-default-features` CI job; compiles to nothing otherwise.
#![cfg(not(feature = "zstd"))]

use asciicast_rs::{Asciicast, AsciicastVersioned, V2};
use ruzstd::encoding::{CompressionLevel, compress_to_vec};

const V2_CAST: &str = include_str!("fixtures/v2.cast");

#[test]
fn compressed_input_is_not_decoded_without_feature() {
    let compressed = compress_to_vec(V2_CAST.as_bytes(), CompressionLevel::Fastest);
    // The zstd bytes are treated as raw input and fail to parse as JSON, rather
    // than being decoded behind the caller's back.
    assert!(Asciicast::<V2>::from_slice(&compressed).is_err());
    assert!(AsciicastVersioned::from_slice(&compressed).is_err());
}
