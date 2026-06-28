//! Transparent zstd decompression (the `zstd` feature).
//!
//! Compressed inputs are built with ruzstd's own encoder, except for the static
//! `v2.cast.zst` fixture which was produced by the reference `zstd` CLI so the
//! decoder is also exercised against real-world (libzstd) output.
#![cfg(feature = "zstd")]

use std::io::Read;

use asciicast_rs::{Asciicast, AsciicastVersioned, Error, V1, V2, V3, v2, v3};
use ruzstd::encoding::{CompressionLevel, compress_to_vec};

/// A reader that yields at most one byte per `read`, so that a wrapping
/// `BufReader::fill_buf` returns fewer than the 4 magic bytes at a time —
/// mimicking a slow network- or pipe-backed source.
struct Trickle<R: Read>(R);

impl<R: Read> Read for Trickle<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match buf.get_mut(..1) {
            Some(first) => self.0.read(first),
            None => Ok(0),
        }
    }
}

const V1_CAST: &str = include_str!("fixtures/v1.cast");
const V2_CAST: &str = include_str!("fixtures/v2.cast");
const V3_CAST: &str = include_str!("fixtures/v3.cast");

/// `tests/fixtures/v2.cast` compressed by the libzstd CLI at level 19.
const V2_CAST_ZST: &[u8] = include_bytes!("fixtures/v2.cast.zst");

fn compress(bytes: &[u8]) -> Vec<u8> {
    compress_to_vec(bytes, CompressionLevel::Fastest)
}

#[test]
fn from_slice_roundtrips_v1() -> Result<(), Error> {
    let from_zstd = Asciicast::<V1>::from_slice(&compress(V1_CAST.as_bytes()))?;
    let plain = Asciicast::<V1>::from_slice(V1_CAST.as_bytes())?;
    assert_eq!(from_zstd, plain);
    Ok(())
}

#[test]
fn from_slice_roundtrips_v2() -> Result<(), Error> {
    let from_zstd = Asciicast::<V2>::from_slice(&compress(V2_CAST.as_bytes()))?;
    let plain = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(from_zstd, plain);
    Ok(())
}

#[test]
fn from_slice_roundtrips_v3() -> Result<(), Error> {
    let from_zstd = Asciicast::<V3>::from_slice(&compress(V3_CAST.as_bytes()))?;
    let plain = Asciicast::<V3>::from_slice(V3_CAST.as_bytes())?;
    assert_eq!(from_zstd, plain);
    Ok(())
}

#[test]
fn from_reader_roundtrips_v2() -> Result<(), Error> {
    let compressed = compress(V2_CAST.as_bytes());
    let from_zstd = Asciicast::<V2>::from_reader(compressed.as_slice())?;
    let plain = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(from_zstd, plain);
    Ok(())
}

#[test]
fn from_path_reads_zstd_file() -> Result<(), Error> {
    let from_file = Asciicast::<V2>::from_path("tests/fixtures/v2.cast.zst")?;
    let plain = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(from_file, plain);
    Ok(())
}

#[test]
fn from_slice_decodes_real_libzstd_output() -> Result<(), Error> {
    let from_zstd = Asciicast::<V2>::from_slice(V2_CAST_ZST)?;
    let plain = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(from_zstd, plain);
    Ok(())
}

#[test]
fn versioned_detects_compressed_v1() -> Result<(), Error> {
    let detected = AsciicastVersioned::from_slice(&compress(V1_CAST.as_bytes()))?;
    let plain = AsciicastVersioned::from_slice(V1_CAST.as_bytes())?;
    assert_eq!(detected, plain);
    assert!(matches!(detected, AsciicastVersioned::V1(_)));
    Ok(())
}

#[test]
fn versioned_detects_compressed_v2() -> Result<(), Error> {
    let detected = AsciicastVersioned::from_slice(&compress(V2_CAST.as_bytes()))?;
    let plain = AsciicastVersioned::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(detected, plain);
    assert!(matches!(detected, AsciicastVersioned::V2(_)));
    Ok(())
}

#[test]
fn versioned_detects_compressed_v3() -> Result<(), Error> {
    let detected = AsciicastVersioned::from_slice(&compress(V3_CAST.as_bytes()))?;
    let plain = AsciicastVersioned::from_slice(V3_CAST.as_bytes())?;
    assert_eq!(detected, plain);
    assert!(matches!(detected, AsciicastVersioned::V3(_)));
    Ok(())
}

#[test]
fn stream_v2_into_recording_matches_eager() -> Result<(), Error> {
    let compressed = compress(V2_CAST.as_bytes());
    let streamed = v2::stream(compressed.as_slice())?.into_recording()?;
    let plain = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(streamed, plain);
    Ok(())
}

#[test]
fn stream_v3_over_zstd_reads_header_and_events() -> Result<(), Error> {
    let compressed = compress(V3_CAST.as_bytes());
    let mut reader = v3::stream(compressed.as_slice())?;
    assert_eq!(reader.header().term.cols, 80);
    let events = (&mut reader).collect::<Result<Vec<_>, _>>()?;
    let plain = Asciicast::<V3>::from_slice(V3_CAST.as_bytes())?;
    assert_eq!(events, plain.events);
    Ok(())
}

#[test]
fn stream_absolute_times_over_zstd_matches_plain() -> Result<(), Error> {
    let compressed = compress(V3_CAST.as_bytes());
    let from_zstd = v3::stream(compressed.as_slice())?
        .absolute_times()
        .map(|item| item.map(|(at, _)| at))
        .collect::<Result<Vec<_>, _>>()?;
    let plain = v3::stream(V3_CAST.as_bytes())?
        .absolute_times()
        .map(|item| item.map(|(at, _)| at))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(from_zstd, plain);
    Ok(())
}

#[test]
fn decodes_multiple_compression_levels() -> Result<(), Error> {
    let plain = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    for level in [CompressionLevel::Uncompressed, CompressionLevel::Fastest] {
        let compressed = compress_to_vec(V2_CAST.as_bytes(), level);
        assert_eq!(Asciicast::<V2>::from_slice(&compressed)?, plain);
    }
    Ok(())
}

#[test]
fn detects_zstd_when_fill_buf_returns_partial_magic() -> Result<(), Error> {
    // The reader dribbles one byte at a time, so detection must not rely on a
    // single `fill_buf` returning the whole 4-byte magic number.
    let compressed = compress(V2_CAST.as_bytes());
    let reader = std::io::BufReader::new(Trickle(compressed.as_slice()));
    let from_zstd = Asciicast::<V2>::from_reader(reader)?;
    let plain = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(from_zstd, plain);
    Ok(())
}

#[test]
fn plain_input_still_parses_with_feature_on() -> Result<(), Error> {
    // Auto-detection must not disturb ordinary, uncompressed recordings.
    let parsed = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(parsed.header.width, 80);
    Ok(())
}

#[test]
fn truncated_zstd_stream_is_decompress_error() {
    let mut compressed = compress(V2_CAST.as_bytes());
    compressed.truncate(compressed.len() / 2);
    assert!(matches!(
        Asciicast::<V2>::from_slice(&compressed),
        Err(Error::Decompress(_))
    ));
}

#[test]
fn truncated_compressed_v1_is_decompress_error() {
    // v1 reads its single document to completion, so corruption mid-stream must
    // still be reported as a decompression error, not a JSON error.
    let mut compressed = compress(V1_CAST.as_bytes());
    compressed.truncate(compressed.len() / 2);
    assert!(matches!(
        Asciicast::<V1>::from_slice(&compressed),
        Err(Error::Decompress(_))
    ));
}

#[test]
fn versioned_truncated_compressed_v1_is_decompress_error() {
    // The version-detecting path reads v1 to completion after decompressing up
    // front, so mid-stream corruption must still surface as a decompression
    // error rather than serde mislabelling the decoded bytes as JSON.
    let mut compressed = compress(V1_CAST.as_bytes());
    compressed.truncate(compressed.len() / 2);
    assert!(matches!(
        AsciicastVersioned::from_slice(&compressed),
        Err(Error::Decompress(_))
    ));
}

#[test]
fn zstd_magic_followed_by_garbage_is_decompress_error() {
    let mut data = vec![0x28, 0xb5, 0x2f, 0xfd];
    data.extend_from_slice(&[0xff; 16]);
    assert!(matches!(
        Asciicast::<V2>::from_slice(&data),
        Err(Error::Decompress(_))
    ));
}

#[test]
fn zstd_decoding_to_non_json_is_json_error() {
    // The frame decodes cleanly, so the failure is a genuine JSON error rather
    // than a decompression one.
    let compressed = compress(b"this is not an asciicast recording\n");
    assert!(matches!(
        Asciicast::<V2>::from_slice(&compressed),
        Err(Error::Json(_))
    ));
}

#[test]
fn short_inputs_do_not_panic() {
    // Inputs shorter than the 4-byte magic must not match and must not panic.
    for prefix_len in 0..4_usize {
        let bytes = vec![0x28_u8; prefix_len];
        let _ = Asciicast::<V2>::from_slice(&bytes);
    }
    // Exactly the magic with no frame body must error, not panic.
    assert!(Asciicast::<V2>::from_slice(&[0x28, 0xb5, 0x2f, 0xfd]).is_err());
}
