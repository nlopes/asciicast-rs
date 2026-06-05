//! A library to parse `asciicast` files.
//!
//! It supports versions v1, v2, and v3.
//!
//! # Overview
//!
//! [`Asciicast<V>`] is the core type, parameterised by a version marker
//! ([`V1`], [`V2`], [`V3`]). Use it directly when you know the version:
//!
//! ```no_run
//! use asciicast_rs::{Asciicast, V2};
//!
//! let cast = Asciicast::<V2>::from_path("recording.cast")?;
//! # Ok::<(), asciicast_rs::Error>(())
//! ```
//!
//! When the version is not known ahead of time, [`AsciicastVersioned`]
//! auto-detects it from the content and yields the matching variant.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

mod error;
mod versions;

pub use error::Error;
pub use versions::{V1, V2, V3, Version, common, v1, v2, v3};

/// A parsed `asciicast` recording of a known version `V`.
#[derive(Debug, Clone, PartialEq)]
pub struct Asciicast<V: Version> {
    /// The recording's header metadata.
    pub header: V::Header,
    /// The recording's events, in order.
    pub events: Vec<V::Event>,
}

impl<V: Version> Asciicast<V> {
    /// Parse a recording from a buffered reader.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if reading fails, the input is not valid JSON for
    /// version `V`, the declared version does not match, or an event payload is
    /// malformed.
    pub fn from_reader<R: BufRead>(reader: R) -> Result<Self, Error> {
        V::parse(reader)
    }

    /// Parse a recording from a byte slice.
    ///
    /// # Errors
    ///
    /// See [`Asciicast::from_reader`].
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        V::parse(bytes)
    }

    /// Parse a recording from a file path.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the file cannot be opened, or for any reason
    /// described by [`Asciicast::from_reader`].
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        V::parse(BufReader::new(File::open(path)?))
    }
}

/// A parsed `asciicast` recording whose version was detected at runtime.
///
/// Returned by the auto-detecting constructors; `match` on it to recover the
/// fully typed [`Asciicast<V>`].
#[derive(Debug, Clone, PartialEq)]
pub enum AsciicastVersioned {
    /// A v1 recording.
    V1(Asciicast<V1>),
    /// A v2 recording.
    V2(Asciicast<V2>),
    /// A v3 recording.
    V3(Asciicast<V3>),
}

/// Commonly used types, re-exported for glob import.
pub mod prelude {
    pub use crate::versions::{V1, V2, V3, Version};
    pub use crate::{Asciicast, AsciicastVersioned, Error};
}
