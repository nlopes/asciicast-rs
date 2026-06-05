//! asciicast [v3](https://docs.asciinema.org/manual/asciicast/v3/).
//!
//! A v3 recording is newline-delimited JSON: a header object on the first line
//! followed by one `[interval, code, data]` event array per line. Comment lines
//! beginning with `#` are ignored.

use std::io::BufRead;

use serde::Deserialize;

use crate::{
    Asciicast, Error,
    versions::{
        V3, Version,
        common::{self, Env, ExitStatus, Resize, Theme},
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
}

/// The internal wire shape of an event line: `[interval, code, data]`.
#[derive(Deserialize)]
struct RawEvent(f64, EventCode, String);

/// A typed v3 event payload.
#[derive(Debug, Clone, PartialEq)]
pub enum EventPayload {
    /// Output written to the terminal.
    Output(String),
    /// User keyboard input.
    Input(String),
    /// A marker with an (possibly empty) label.
    Marker(String),
    /// A terminal resize to new dimensions.
    Resize(Resize),
    /// Session exit with a status.
    Exit(ExitStatus),
}

/// A single v3 event.
///
/// `interval` is the number of seconds elapsed since the previous event.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Seconds since the previous event.
    pub interval: f64,
    /// The typed payload for this event.
    pub payload: EventPayload,
}

impl TryFrom<RawEvent> for Event {
    type Error = Error;

    fn try_from(raw: RawEvent) -> Result<Self, Self::Error> {
        let RawEvent(interval, code, data) = raw;
        let payload = match code {
            EventCode::Output => EventPayload::Output(data),
            EventCode::Input => EventPayload::Input(data),
            EventCode::Marker => EventPayload::Marker(data),
            EventCode::Resize => EventPayload::Resize(Resize::parse(&data)?),
            EventCode::Exit => EventPayload::Exit(ExitStatus::parse(&data)?),
        };
        Ok(Self { interval, payload })
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
            EventPayload::Exit(_) => EventCode::Exit,
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
            | EventPayload::Exit(_) => None,
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
            | EventPayload::Exit(_) => None,
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
            | EventPayload::Exit(_) => None,
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
            | EventPayload::Exit(_) => None,
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
            | EventPayload::Resize(_) => None,
        }
    }
}

/// Parse a v3 recording from a buffered reader.
pub(crate) fn parse<R: BufRead>(reader: R) -> Result<Asciicast<V3>, Error> {
    let (header, events) = common::parse_ndjson(
        reader,
        V3::NUMBER,
        true,
        |header: &Header| header.version,
        |line| Event::try_from(serde_json::from_str::<RawEvent>(line)?),
    )?;
    Ok(Asciicast { header, events })
}
