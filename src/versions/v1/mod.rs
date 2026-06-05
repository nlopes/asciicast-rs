//! asciicast [v1](https://docs.asciinema.org/manual/asciicast/v1/).
//!
//! A v1 recording is a single JSON object whose `stdout` field holds the
//! `[delay, data]` output frames.

use std::io::BufRead;

use serde::Deserialize;

use crate::{
    Asciicast, Error,
    versions::{V1, Version, common::Env},
};

/// A convenient alias for a fully parsed v1 recording.
pub type Recording = Asciicast<V1>;

/// The metadata of a v1 recording (everything except the `stdout` frames).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Header {
    /// Format version; always `1`.
    pub version: u8,
    /// Terminal width in columns.
    pub width: u16,
    /// Terminal height in rows.
    pub height: u16,
    /// Total recording duration in seconds.
    pub duration: f64,
    /// The recorded command.
    #[serde(default)]
    pub command: Option<String>,
    /// The recording's title.
    #[serde(default)]
    pub title: Option<String>,
    /// Captured environment variables.
    #[serde(default)]
    pub env: Option<Env>,
}

/// The internal wire shape of a frame: `[delay, data]`.
#[derive(Deserialize)]
struct RawFrame(f64, String);

/// A single v1 output frame.
///
/// `delay` is the number of seconds elapsed since the previous frame; `data` is
/// the terminal output written at that point.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Seconds since the previous frame.
    pub delay: f64,
    /// The terminal output written.
    pub data: String,
}

impl From<RawFrame> for Frame {
    fn from(raw: RawFrame) -> Self {
        let RawFrame(delay, data) = raw;
        Self { delay, data }
    }
}

/// The whole v1 document: header metadata plus the `stdout` frames.
#[derive(Deserialize)]
struct Document {
    #[serde(flatten)]
    header: Header,
    stdout: Vec<RawFrame>,
}

/// Parse a v1 recording from a buffered reader.
pub(crate) fn parse<R: BufRead>(reader: R) -> Result<Asciicast<V1>, Error> {
    let document: Document = serde_json::from_reader(reader)?;
    if document.header.version != V1::NUMBER {
        return Err(Error::VersionMismatch {
            expected: V1::NUMBER,
            found: document.header.version,
        });
    }
    let events = document.stdout.into_iter().map(Frame::from).collect();
    Ok(Asciicast {
        header: document.header,
        events,
    })
}
