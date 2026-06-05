#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::{
    fs::File,
    io::{BufRead, BufReader, Cursor, Read},
    path::Path,
};

use serde::Deserialize;

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

impl AsciicastVersioned {
    /// Parse a recording from a buffered reader, detecting its version from the
    /// content.
    ///
    /// The version is read from the `version` field of the first line. A v1
    /// recording may be pretty-printed across multiple lines, in which case the
    /// first line is not a complete JSON object; that case is treated as v1.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownVersion`] if the declared version is not 1, 2, or
    /// 3, or any error from the matching [`Asciicast::from_reader`].
    pub fn from_reader<R: BufRead>(mut reader: R) -> Result<Self, Error> {
        #[derive(Deserialize)]
        struct VersionProbe {
            version: u8,
        }

        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;

        // A complete first line carries the version; a parse failure means a
        // pretty-printed (multi-line) v1 document whose first line is just `{`.
        let version = serde_json::from_str::<VersionProbe>(&first_line)
            .ok()
            .map(|probe| probe.version);

        // Re-feed the consumed first line ahead of the rest of the reader.
        let combined = BufReader::new(Cursor::new(first_line.into_bytes()).chain(reader));

        match version {
            None | Some(1) => Asciicast::<V1>::from_reader(combined).map(Self::V1),
            Some(2) => Asciicast::<V2>::from_reader(combined).map(Self::V2),
            Some(3) => Asciicast::<V3>::from_reader(combined).map(Self::V3),
            Some(other) => Err(Error::UnknownVersion(other)),
        }
    }

    /// Parse a recording from a byte slice, detecting its version.
    ///
    /// # Errors
    ///
    /// See [`AsciicastVersioned::from_reader`].
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        Self::from_reader(bytes)
    }

    /// Parse a recording from a file path, detecting its version.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the file cannot be opened, or for any reason
    /// described by [`AsciicastVersioned::from_reader`].
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::from_reader(BufReader::new(File::open(path)?))
    }
}

/// Commonly used types, re-exported for glob import.
pub mod prelude {
    pub use crate::versions::{V1, V2, V3, Version};
    pub use crate::{Asciicast, AsciicastVersioned, Error};
}
