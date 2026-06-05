//! A streaming reader for newline-delimited (v2/v3) recordings.

use std::io::{BufRead, Lines};

use serde::de::DeserializeOwned;

use crate::{Asciicast, Error, versions::Streamable};

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
    lines: Lines<R>,
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
        let mut lines = reader.lines();

        let header_line = loop {
            match lines.next() {
                Some(line) => {
                    let line = line?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    break line;
                }
                None => return Err(Error::MissingHeader),
            }
        };

        let header: V::Header = serde_json::from_str(&header_line)?;
        let found = V::header_version(&header);
        if found != V::NUMBER {
            return Err(Error::VersionMismatch {
                expected: V::NUMBER,
                found,
            });
        }

        Ok(Self { header, lines })
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
        // `find_map` advances `lines` until the closure yields `Some`, so blank
        // (and, for v3, comment) lines are skipped without a hand-written loop.
        self.lines.find_map(|line| {
            let line = match line {
                Ok(line) => line,
                Err(error) => return Some(Err(error.into())),
            };
            let trimmed = line.trim();
            if trimmed.is_empty() || (V::SKIP_COMMENTS && trimmed.starts_with('#')) {
                None
            } else {
                Some(V::parse_event(&line))
            }
        })
    }
}
