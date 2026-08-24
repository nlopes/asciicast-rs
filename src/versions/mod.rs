//! Per-version data structures and parsing.
//!
//! Each supported version lives in its own module ([`v1`], [`v2`], [`v3`]) and is
//! tied to a zero-sized marker type ([`V1`], [`V2`], [`V3`]) implementing the
//! sealed [`Version`] trait. The marker is what parameterises [`crate::Asciicast`].

use std::fmt::Debug;
use std::io::BufRead;

use crate::{Asciicast, Error};

pub mod common;
pub mod v1;
pub mod v2;
pub mod v3;

mod private {
    pub trait Sealed {}

    impl Sealed for super::V1 {}
    impl Sealed for super::V2 {}
    impl Sealed for super::V3 {}
}

/// A supported `asciicast` version.
///
/// Sealed: only [`V1`], [`V2`], and [`V3`] implement it.
pub trait Version: private::Sealed + Sized {
    /// The numeric version as it appears in a file's `version` field.
    const NUMBER: u8;

    /// The header type for this version.
    type Header: Debug + Clone + PartialEq;

    /// The event type for this version.
    type Event: Debug + Clone + PartialEq;

    /// Whether each event's stored time is relative to the previous event (as in
    /// v1 and v3) rather than absolute since the recording start (as in v2).
    const RELATIVE_TIMING: bool;

    /// Parse a recording of this exact version from a buffered reader.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if reading fails, the input is not valid JSON, the
    /// declared version does not match, or an event is malformed.
    fn parse<R: BufRead>(reader: R) -> Result<Asciicast<Self>, Error>;

    /// The raw stored time of an event: its delay/interval for relative-timed
    /// versions, or its absolute time for v2.
    fn event_time(event: &Self::Event) -> f64;
}

/// A version whose events are newline-delimited and can therefore be streamed
/// one line at a time via [`crate::Reader`].
///
/// Sealed (it requires the sealed [`Version`]) and implemented only by [`V2`] and [`V3`].
/// v1 is a single-document format, so it deliberately does **not** implement
/// [`Streamable`] which makes `Reader<V1, _>` a compile error.
pub trait Streamable: Version {
    /// Whether `#`-prefixed comment lines are skipped in the event stream.
    const SKIP_COMMENTS: bool;

    /// Read the `version` field from a parsed header.
    fn header_version(header: &Self::Header) -> u8;

    /// Parse a single event line into an event.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the line is not a valid event for this version.
    fn parse_event(line: &str) -> Result<Self::Event, Error>;
}

/// Marker type for asciicast v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V1;

/// Marker type for asciicast v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V2;

/// Marker type for asciicast v3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V3;

impl Version for V1 {
    const NUMBER: u8 = 1;
    type Header = v1::Header;
    type Event = v1::Frame;
    const RELATIVE_TIMING: bool = true;

    fn parse<R: BufRead>(reader: R) -> Result<Asciicast<Self>, Error> {
        v1::parse(reader)
    }

    fn event_time(event: &Self::Event) -> f64 {
        event.delay
    }
}

impl Version for V2 {
    const NUMBER: u8 = 2;
    type Header = v2::Header;
    type Event = v2::Event;
    const RELATIVE_TIMING: bool = false;

    fn parse<R: BufRead>(reader: R) -> Result<Asciicast<Self>, Error> {
        v2::parse(reader)
    }

    fn event_time(event: &Self::Event) -> f64 {
        event.time
    }
}

impl Version for V3 {
    const NUMBER: u8 = 3;
    type Header = v3::Header;
    type Event = v3::Event;
    const RELATIVE_TIMING: bool = true;

    fn parse<R: BufRead>(reader: R) -> Result<Asciicast<Self>, Error> {
        v3::parse(reader)
    }

    fn event_time(event: &Self::Event) -> f64 {
        event.interval
    }
}
