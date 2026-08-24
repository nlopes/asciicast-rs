//! asciicast [v3](https://docs.asciinema.org/manual/asciicast/v3/).
//!
//! A v3 recording is newline-delimited JSON: a header object on the first line
//! followed by one `[interval, code, data]` event array per line. Comment lines
//! beginning with `#` are ignored.

use std::io::BufRead;

use serde::Deserialize;

use crate::{
    Asciicast, Error, Reader,
    versions::{
        Streamable, V3,
        common::{Env, ExitStatus, Resize, Theme, deserialize_non_negative_f64},
    },
};

/// A convenient alias for a fully parsed v3 recording.
pub type Recording = Asciicast<V3>;

/// Terminal information carried by a v3 header.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Term {
    /// Terminal width in columns.
    pub cols: u16,
    /// Terminal height in rows.
    pub rows: u16,
    /// Terminal type, e.g. `xterm-256color`.
    #[serde(default)]
    pub r#type: Option<String>,
    /// Terminal version, as reported by an `XTVERSION` query.
    #[serde(default)]
    pub version: Option<String>,
    /// Terminal colour scheme.
    #[serde(default)]
    pub theme: Option<Theme>,
}

/// The header (first line) of a v3 recording.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Header {
    /// Format version; always `3`.
    pub version: u8,
    /// Terminal information (dimensions, type, theme).
    pub term: Term,
    /// Unix timestamp (seconds) of the recording's start.
    #[serde(default)]
    pub timestamp: Option<i64>,
    /// Maximum inactivity duration a player should honour, in seconds.
    ///
    /// This should be used by an asciicast player to reduce all terminal inactivity
    /// (delays between frames) to a maximum of `idle_time_limit` value.
    #[serde(default)]
    pub idle_time_limit: Option<f64>,
    /// The recorded command.
    #[serde(default)]
    pub command: Option<String>,
    /// The recording's title.
    #[serde(default)]
    pub title: Option<String>,
    /// Captured environment variables.
    #[serde(default)]
    pub env: Option<Env>,
    /// Categorical tags for the recording.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
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

/// The event type identifier for a v3 event.
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
    /// Session exit (`x`).
    #[serde(rename = "x")]
    Exit,
    /// An event code this crate does not interpret.
    #[serde(other)]
    Unknown,
}

/// The internal wire shape of an event line: `[interval, code, data]`.
#[derive(Deserialize)]
struct RawEvent(
    #[serde(deserialize_with = "deserialize_non_negative_f64")] f64,
    String,
    String,
);

/// A typed v3 event payload.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EventPayload {
    /// Output written to the terminal.
    Output(String),
    /// User keyboard input.
    Input(String),
    /// A marker with a (possibly empty) label.
    Marker(String),
    /// A terminal resize to new dimensions.
    Resize(Resize),
    /// Session exit with a status.
    Exit(ExitStatus),
    /// An event whose code is not known to this crate.
    Unknown {
        /// The complete event code.
        code: String,
        /// The event data.
        data: String,
    },
}

/// A single v3 event.
///
/// `interval` is the number of seconds elapsed since the previous event.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Non-negative seconds since the previous event.
    pub interval: f64,
    /// The typed payload for this event.
    pub payload: EventPayload,
}

impl TryFrom<RawEvent> for Event {
    type Error = Error;

    fn try_from(raw: RawEvent) -> Result<Self, Self::Error> {
        let RawEvent(interval, code, data) = raw;
        let payload = match code.as_str() {
            "o" => EventPayload::Output(data),
            "i" => EventPayload::Input(data),
            "m" => EventPayload::Marker(data),
            "r" => EventPayload::Resize(Resize::parse(&data)?),
            "x" => EventPayload::Exit(ExitStatus::parse(&data)?),
            "" => return Err(Error::MalformedEvent("missing event code".to_owned())),
            _ => EventPayload::Unknown { code, data },
        };
        Ok(Self { interval, payload })
    }
}

impl Event {
    /// The event type identifier.
    ///
    /// Unknown events return [`EventCode::Unknown`]; their complete code is stored in
    /// [`EventPayload::Unknown`].
    #[must_use]
    pub fn code(&self) -> EventCode {
        match self.payload {
            EventPayload::Output(_) => EventCode::Output,
            EventPayload::Input(_) => EventCode::Input,
            EventPayload::Marker(_) => EventCode::Marker,
            EventPayload::Resize(_) => EventCode::Resize,
            EventPayload::Exit(_) => EventCode::Exit,
            EventPayload::Unknown { .. } => EventCode::Unknown,
        }
    }

    /// The output text, if this is an output event.
    #[must_use]
    pub fn as_output(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Output(s) => Some(s),
            EventPayload::Input(_)
            | EventPayload::Marker(_)
            | EventPayload::Resize(_)
            | EventPayload::Exit(_)
            | EventPayload::Unknown { .. } => None,
        }
    }

    /// The input text, if this is an input event.
    #[must_use]
    pub fn as_input(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Input(s) => Some(s),
            EventPayload::Output(_)
            | EventPayload::Marker(_)
            | EventPayload::Resize(_)
            | EventPayload::Exit(_)
            | EventPayload::Unknown { .. } => None,
        }
    }

    /// The marker label, if this is a marker event.
    #[must_use]
    pub fn as_marker(&self) -> Option<&str> {
        match &self.payload {
            EventPayload::Marker(s) => Some(s),
            EventPayload::Output(_)
            | EventPayload::Input(_)
            | EventPayload::Resize(_)
            | EventPayload::Exit(_)
            | EventPayload::Unknown { .. } => None,
        }
    }

    /// The new dimensions, if this is a resize event.
    #[must_use]
    pub fn as_resize(&self) -> Option<Resize> {
        match &self.payload {
            EventPayload::Resize(r) => Some(*r),
            EventPayload::Output(_)
            | EventPayload::Input(_)
            | EventPayload::Marker(_)
            | EventPayload::Exit(_)
            | EventPayload::Unknown { .. } => None,
        }
    }

    /// The exit status, if this is an exit event.
    #[must_use]
    pub fn as_exit(&self) -> Option<ExitStatus> {
        match &self.payload {
            EventPayload::Exit(status) => Some(*status),
            EventPayload::Output(_)
            | EventPayload::Input(_)
            | EventPayload::Marker(_)
            | EventPayload::Resize(_)
            | EventPayload::Unknown { .. } => None,
        }
    }
}

impl Streamable for V3 {
    const SKIP_COMMENTS: bool = true;

    fn header_version(header: &Header) -> u8 {
        header.version
    }

    fn parse_event(line: &str) -> Result<Event, Error> {
        Event::try_from(serde_json::from_str::<RawEvent>(line)?)
    }
}

/// Parse the header of a v3 recording and return a [`Reader`] that streams its
/// events lazily (skipping `#` comment lines).
///
/// A convenience wrapper over [`Reader::open`] that infers the version, so you
/// can write `v3::stream(reader)` instead of `Reader::<V3, _>::open(reader)`.
///
/// # Errors
///
/// Returns an [`Error`] if reading the header fails, it is not valid JSON, or
/// the declared version is not 3.
pub fn stream<R: BufRead>(reader: R) -> Result<Reader<V3, R>, Error> {
    Reader::open(reader)
}

/// Parse a v3 recording from a buffered reader.
pub(crate) fn parse<R: BufRead>(reader: R) -> Result<Asciicast<V3>, Error> {
    Reader::<V3, R>::open(reader)?.into_recording()
}
