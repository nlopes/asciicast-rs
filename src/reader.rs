//! A streaming reader for newline-delimited (v2/v3) recordings.

use std::io::BufRead;

use serde::de::DeserializeOwned;

use crate::{Asciicast, Error, source::Source, versions::Streamable};

// Larger lines are accepted, but their storage is not retained between events.
const MAX_RETAINED_LINE_CAPACITY: usize = 64 * 1024;

pub(crate) fn read_header_line(reader: &mut impl BufRead) -> Result<String, Error> {
    let mut line = String::new();
    loop {
        if reader.read_line(&mut line)? == 0 {
            return Err(Error::MissingHeader);
        }
        if !line.trim().is_empty() {
            return Ok(line);
        }
        line.clear();
    }
}

/// A lazy reader over a newline-delimited recording.
///
/// Create one with [`Reader::open`] (or one of the wrappers [`crate::v2::stream`] /
/// [`crate::v3::stream`]), which reads and validates the header. The reader then yields
/// one `Result<Event, Error>` per event line as an [`Iterator`], so a long recording is
/// never fully buffered. Call [`Reader::into_recording`] to drain it into an eager
/// [`Asciicast`].
///
/// Only the newline-delimited versions implement [`Streamable`], so a `Reader` cannot be
/// constructed for v1. What is nice here is that it is enforced at compile time.
///
/// ```
/// use asciicast_rs::{Reader, V2};
///
/// let bytes: &[u8] = b"{\"version\":2,\"width\":80,\"height\":24}\n[0.5,\"o\",\"hi\"]\n";
/// let reader = Reader::<V2, _>::open(bytes).expect("valid header");
/// assert_eq!(reader.header().width, 80);
/// for event in reader {
///     let event = event.expect("valid event");
///     if let Some(text) = event.as_output() {
///         assert_eq!(text, "hi");
///     }
/// }
/// ```
pub struct Reader<V: Streamable, R: BufRead> {
    header: V::Header,
    source: Source<R>,
    line: Vec<u8>,
}

impl<V: Streamable, R: BufRead> Reader<V, R>
where
    V::Header: DeserializeOwned,
{
    /// Read and validate the header, returning a reader positioned at the first
    /// event.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if reading fails, the header is not valid JSON, or
    /// the declared version does not match `V`.
    pub fn open(reader: R) -> Result<Self, Error> {
        // Transparently decode zstd input (a no-op without the `zstd` feature).
        Self::from_source(Source::new(reader)?)
    }

    /// Read and validate the header over an already-built [`Source`].
    ///
    /// Shared by [`Reader::open`] and the version-detecting path
    /// ([`AsciicastVersioned`](crate::AsciicastVersioned)): the latter has
    /// already decompressed the stream to read the version probe, so it passes a
    /// [`Source::plain`] to skip a second, redundant zstd detection.
    pub(crate) fn from_source(mut source: Source<R>) -> Result<Self, Error> {
        let header_line = read_header_line(&mut source)?;
        let header: V::Header = serde_json::from_str(&header_line)?;
        let found = V::header_version(&header);
        if found != V::NUMBER {
            return Err(Error::VersionMismatch {
                expected: V::NUMBER,
                found,
            });
        }

        Ok(Self {
            header,
            source,
            line: if header_line.capacity() > MAX_RETAINED_LINE_CAPACITY {
                Vec::new()
            } else {
                let mut line = header_line.into_bytes();
                line.clear();
                line
            },
        })
    }

    /// The parsed header.
    #[must_use]
    pub fn header(&self) -> &V::Header {
        &self.header
    }

    /// Drain the remaining events and materialise an eager [`Asciicast`], bundling them
    /// with the parsed header.
    ///
    /// If you only need the events (not the header), the `Reader` is an [`Iterator`], so
    /// collect them directly instead: `reader.collect::<Result<Vec<_>, _>>()`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any event line is malformed.
    pub fn into_recording(mut self) -> Result<Asciicast<V>, Error> {
        let events = (&mut self).collect::<Result<Vec<_>, _>>()?;
        Ok(Asciicast {
            header: self.header,
            events,
        })
    }

    /// Stream the events paired with their absolute time, in seconds since the
    /// start of the recording.
    ///
    /// The streaming counterpart of [`Asciicast::absolute_times`]: each item is
    /// `Result<(f64, V::Event), Error>`, with relative timings (v3) accumulated
    /// as the stream is consumed. Consumes the reader.
    pub fn absolute_times(self) -> impl Iterator<Item = Result<(f64, V::Event), Error>> {
        self.scan(0.0_f64, |elapsed, event| {
            Some(event.map(|event| {
                let raw = V::event_time(&event);
                let absolute = if V::RELATIVE_TIMING {
                    *elapsed += raw;
                    *elapsed
                } else {
                    raw
                };
                (absolute, event)
            }))
        })
    }
}

impl<V: Streamable, R: BufRead> Iterator for Reader<V, R> {
    type Item = Result<V::Event, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let buffer = match self.source.fill_buf() {
                Ok(buffer) => buffer,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    self.line = Vec::new();
                    return Some(Err(error.into()));
                }
            };
            if buffer.is_empty() {
                let event = if self.line.is_empty() {
                    None
                } else {
                    parse_line::<V>(&self.line)
                };
                self.line = Vec::new();
                return event;
            }
            let Some(end) = memchr::memchr(b'\n', buffer) else {
                self.line.extend_from_slice(buffer);
                let consumed = buffer.len();
                self.source.consume(consumed);
                continue;
            };
            let consumed = end + 1;
            let (line, _) = buffer.split_at(consumed);
            // Most events fit in the input buffer. Parse them in place, copying
            // only lines that cross a buffer boundary into the spill buffer.
            let event = if self.line.is_empty() {
                parse_line::<V>(line)
            } else {
                self.line.extend_from_slice(line);
                let event = parse_line::<V>(&self.line);
                if self.line.capacity() > MAX_RETAINED_LINE_CAPACITY {
                    self.line = Vec::new();
                } else {
                    self.line.clear();
                }
                event
            };
            self.source.consume(consumed);
            if event.is_some() {
                return event;
            }
        }
    }
}

fn parse_line<V: Streamable>(mut bytes: &[u8]) -> Option<Result<V::Event, Error>> {
    let Ok(line) = simdutf8::basic::from_utf8(bytes) else {
        // Preserve read_line's I/O error details for invalid UTF8, including
        // invalid bytes in comments and whitespace-only lines.
        let mut line = String::new();
        return Some(
            bytes
                .read_line(&mut line)
                .map_err(Error::from)
                .and_then(|_| V::parse_event(&line)),
        );
    };
    let line = if let Some(line) = line.strip_suffix('\n') {
        line.strip_suffix('\r').unwrap_or(line)
    } else {
        line
    };
    let trimmed = line.trim();
    if trimmed.is_empty() || (V::SKIP_COMMENTS && trimmed.starts_with('#')) {
        None
    } else {
        Some(V::parse_event(line))
    }
}
