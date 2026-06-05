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

    /// Parse a recording of this exact version from a buffered reader.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if reading fails, the input is not valid JSON, the
    /// declared version does not match, or an event payload is malformed.
    fn parse<R: BufRead>(reader: R) -> Result<Asciicast<Self>, Error>;
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

    fn parse<R: BufRead>(reader: R) -> Result<Asciicast<Self>, Error> {
        v1::parse(reader)
    }
}

impl Version for V2 {
    const NUMBER: u8 = 2;
    type Header = v2::Header;
    type Event = v2::Event;

    fn parse<R: BufRead>(reader: R) -> Result<Asciicast<Self>, Error> {
        v2::parse(reader)
    }
}

impl Version for V3 {
    const NUMBER: u8 = 3;
    type Header = v3::Header;
    type Event = v3::Event;

    fn parse<R: BufRead>(reader: R) -> Result<Asciicast<Self>, Error> {
        v3::parse(reader)
    }
}
