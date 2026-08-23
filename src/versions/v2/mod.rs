//! asciicast [v2](https://docs.asciinema.org/manual/asciicast/v2/).
//!
//! A v2 recording is newline-delimited JSON: a header object on the first line
//! followed by one `[time, code, data]` event array per line.

use std::io::BufRead;

use serde::Deserialize;

use crate::{
    Asciicast, Error, Reader,
    versions::{
        Streamable, V2,
        common::{Env, Resize, Theme, deserialize_env},
    },
};

/// A convenient alias for a fully parsed v2 recording.
pub type Recording = Asciicast<V2>;

/// The header (first line) of a v2 recording.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Header {
    /// Format version; always `2`.
    pub version: u8,
    /// Terminal width in columns.
    pub width: u16,
    /// Terminal height in rows.
    pub height: u16,
    /// Unix timestamp (seconds) of the recording's start.
    #[serde(default)]
    pub timestamp: Option<i64>,
    /// Total recording duration in seconds.
    #[serde(default)]
    pub duration: Option<f64>,
    /// Maximum inactivity duration a player should honour, in seconds.
    #[serde(default)]
    pub idle_time_limit: Option<f64>,
    /// The recorded command.
    #[serde(default)]
    pub command: Option<String>,
    /// The recording's title.
    #[serde(default)]
    pub title: Option<String>,
    /// Captured environment variables.
    #[serde(default, deserialize_with = "deserialize_env")]
    pub env: Option<Env>,
    /// Terminal colour scheme.
    #[serde(default)]
    pub theme: Option<Theme>,
}

#[cfg(feature = "chrono")]
impl Header {
    /// The recording's start time as a UTC datetime, if a `timestamp` is present.
    #[must_use]
    pub fn timestamp_datetime(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.timestamp
            .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
    }
}

/// The event type identifier for a v2 event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum EventCode {
    /// Output written to the terminal (`o`).
    #[serde(rename = "o")]
    Output,
    /// User keyboard input (`i`).
    #[serde(rename = "i")]
    Input,
    /// A marker / navigation point (`m`).
    #[serde(rename = "m")]
    Marker,
    /// A terminal resize (`r`).
    #[serde(rename = "r")]
    Resize,
}

/// The internal wire shape of an event line: `[time, code, data]`.
#[derive(Deserialize)]
struct RawEvent(f64, EventCode, String);

/// A typed v2 event payload.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EventPayload {
    /// Output written to the terminal.
    Output(String),
    /// User keyboard input.
    Input(String),
    /// A marker with an (possibly empty) label.
    Marker(String),
    /// A terminal resize to new dimensions.
    Resize(Resize),
}

/// A single v2 event.
///
/// `time` is the number of seconds elapsed since the start of the recording.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Seconds since the start of the recording.
    pub time: f64,
    /// The typed payload for this event.
    pub payload: EventPayload,
}

impl TryFrom<RawEvent> for Event {
    type Error = Error;

    fn try_from(raw: RawEvent) -> Result<Self, Self::Error> {
        let RawEvent(time, code, data) = raw;
        let payload = match code {
            EventCode::Output => EventPayload::Output(data),
            EventCode::Input => EventPayload::Input(data),
            EventCode::Marker => EventPayload::Marker(data),
            EventCode::Resize => EventPayload::Resize(Resize::parse(&data)?),
        };
        Ok(Self { time, payload })
    }
}

impl Event {
    /// The event type identifier.
    #[must_use]
    pub fn code(&self) -> EventCode {
        match self.payload {
            EventPayload::Output(_) => EventCode::Output,
            EventPayload::Input(_) => EventCode::Input,
            EventPayload::Marker(_) => EventCode::Marker,
            EventPayload::Resize(_) => EventCode::Resize,
        }
    }

    /// The output text, if this is an output event.
    #[must_use]
    pub fn as_output(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Output(s) => Some(s),
            EventPayload::Input(_) | EventPayload::Marker(_) | EventPayload::Resize(_) => None,
        }
    }

    /// The input text, if this is an input event.
    #[must_use]
    pub fn as_input(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Input(s) => Some(s),
            EventPayload::Output(_) | EventPayload::Marker(_) | EventPayload::Resize(_) => None,
        }
    }

    /// The marker label, if this is a marker event.
    #[must_use]
    pub fn as_marker(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Marker(s) => Some(s),
            EventPayload::Output(_) | EventPayload::Input(_) | EventPayload::Resize(_) => None,
        }
    }

    /// The new dimensions, if this is a resize event.
    #[must_use]
    pub fn as_resize(&self) -> Option<Resize> {
        match &self.payload {
            EventPayload::Resize(r) => Some(*r),
            EventPayload::Output(_) | EventPayload::Input(_) | EventPayload::Marker(_) => None,
        }
    }
}

impl Streamable for V2 {
    const SKIP_COMMENTS: bool = false;

    fn header_version(header: &Header) -> u8 {
        header.version
    }

    fn parse_event(line: &str) -> Result<Event, Error> {
        Event::try_from(serde_json::from_str::<RawEvent>(line)?)
    }
}

/// Parse the header of a v2 recording and return a [`Reader`] that streams its
/// events lazily.
///
/// A convenience wrapper over [`Reader::open`] that infers the version, so you
/// can write `v2::stream(reader)` instead of `Reader::<V2, _>::open(reader)`.
///
/// # Errors
///
/// Returns an [`Error`] if reading the header fails, it is not valid JSON, or
/// the declared version is not 2.
pub fn stream<R: BufRead>(reader: R) -> Result<Reader<V2, R>, Error> {
    Reader::open(reader)
}

/// Parse a v2 recording from a buffered reader.
pub(crate) fn parse<R: BufRead>(reader: R) -> Result<Asciicast<V2>, Error> {
    Reader::<V2, R>::open(reader)?.into_recording()
}
