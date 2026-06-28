//! Transparent zstd decompression of the input stream.
//!
//! A raw input entry point funnels its reader through [`Source::new`]: when the
//! `zstd` feature is enabled and the stream begins with the zstd magic number,
//! the bytes are decoded on the fly; otherwise the reader is used unchanged. A
//! caller that already holds a decoded stream uses [`Source::plain`] instead, so
//! detection is never run twice. The public API is therefore identical for plain
//! and zstd-compressed recordings.

use std::io::{BufRead, Read, Result as IoResult};

use crate::Error;

/// The zstd frame magic number (`0xFD2FB528`, little-endian on the wire).
///
/// Only the standard Zstandard frame magic is detected, not skippable-frame
/// magic (`0x184D2A50..=0x184D2A5F`). Tools that compress recordings emit a
/// standard frame first, and ruzstd's `StreamingDecoder` errors on a leading
/// skippable frame rather than skipping it, so detecting one would promise a
/// decode we cannot deliver.
#[cfg(feature = "zstd")]
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

/// The original reader with the peeked magic-number prefix replayed in front.
#[cfg(feature = "zstd")]
type Prefixed<R> = std::io::Chain<std::io::Cursor<Vec<u8>>, R>;

/// A buffered zstd decoder over the (prefix-replayed) reader.
#[cfg(feature = "zstd")]
type ZstdReader<R> = std::io::BufReader<
    ruzstd::decoding::StreamingDecoder<Prefixed<R>, ruzstd::decoding::FrameDecoder>,
>;

/// The reader held by [`Source::Plain`]: `R` directly, or `R` with its peeked
/// prefix replayed in front when zstd detection is compiled in.
#[cfg(feature = "zstd")]
type PlainReader<R> = Prefixed<R>;
#[cfg(not(feature = "zstd"))]
type PlainReader<R> = R;

/// A reader that is either the original input or a zstd decoder over it.
///
/// Implements [`Read`] and [`BufRead`] by delegating to whichever variant is
/// active, so callers treat compressed and plain input the same way.
pub(crate) enum Source<R: BufRead> {
    /// The input is consumed as-is (the prefix already replayed in front).
    Plain(PlainReader<R>),
    /// The input began with the zstd magic number and is decoded on the fly.
    #[cfg(feature = "zstd")]
    Zstd(Box<ZstdReader<R>>),
}

impl<R: BufRead> Source<R> {
    /// Wrap `reader`, transparently decoding it when the `zstd` feature is
    /// enabled and the stream begins with the zstd magic number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if reading the leading bytes fails, or
    /// `Error::Decompress` if the input begins with the zstd magic number but
    /// its frame header cannot be decoded.
    // Without the `zstd` feature this can never fail, but the fallible signature
    // is kept so callers are identical in both configurations.
    #[cfg_attr(not(feature = "zstd"), allow(clippy::unnecessary_wraps))]
    pub(crate) fn new(reader: R) -> Result<Self, Error> {
        #[cfg(feature = "zstd")]
        {
            let mut reader = reader;
            // Read the leading bytes explicitly rather than relying on
            // `fill_buf`, which may return fewer than the magic's 4 bytes for a
            // network- or pipe-backed reader. `take` + `read_to_end` coalesces
            // short reads and tolerates an early EOF.
            let mut prefix = Vec::with_capacity(ZSTD_MAGIC.len());
            (&mut reader)
                .take(ZSTD_MAGIC.len() as u64)
                .read_to_end(&mut prefix)?;
            let is_zstd = prefix.starts_with(&ZSTD_MAGIC);
            // Replay the consumed prefix ahead of the rest of the reader.
            let replayed = std::io::Cursor::new(prefix).chain(reader);
            if is_zstd {
                let decoder = ruzstd::decoding::StreamingDecoder::new(replayed)
                    .map_err(|err| Error::Decompress(Box::new(err)))?;
                Ok(Self::Zstd(Box::new(std::io::BufReader::new(decoder))))
            } else {
                Ok(Self::Plain(replayed))
            }
        }
        #[cfg(not(feature = "zstd"))]
        Ok(Self::Plain(reader))
    }

    /// Wrap an already-decoded reader, skipping zstd detection entirely.
    ///
    /// Used by the version-detecting path
    /// ([`AsciicastVersioned`](crate::AsciicastVersioned)), which decompresses
    /// the stream up front to read the version probe and then hands the decoded
    /// bytes here. Re-running detection on them would be wasted work and could
    /// only ever yield [`Source::Plain`], so this constructor builds it directly.
    pub(crate) fn plain(reader: R) -> Self {
        #[cfg(feature = "zstd")]
        {
            // The `Plain` variant carries a replayed-prefix slot (see
            // `PlainReader`); an already-decoded caller has nothing to replay, so
            // the prefix is empty.
            Self::Plain(std::io::Cursor::new(Vec::new()).chain(reader))
        }
        #[cfg(not(feature = "zstd"))]
        Self::Plain(reader)
    }
}

impl<R: BufRead> Read for Source<R> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        match self {
            Self::Plain(reader) => reader.read(buf),
            #[cfg(feature = "zstd")]
            Self::Zstd(reader) => reader.read(buf),
        }
    }
}

impl<R: BufRead> BufRead for Source<R> {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        match self {
            Self::Plain(reader) => reader.fill_buf(),
            #[cfg(feature = "zstd")]
            Self::Zstd(reader) => reader.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match self {
            Self::Plain(reader) => reader.consume(amt),
            #[cfg(feature = "zstd")]
            Self::Zstd(reader) => reader.consume(amt),
        }
    }
}
