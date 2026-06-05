//! asciicast [v1](https://docs.asciinema.org/manual/asciicast/v1/).
//!
//! A v1 recording is a single JSON object whose `stdout` field holds the
//! `[delay, data]` output frames.

use std::io::BufRead;

use crate::{Asciicast, Error, versions::V1};

/// The metadata of a v1 recording (everything except the `stdout` frames).
//
// Placeholder — fleshed out in the v1 slice.
#[derive(Debug, Clone, PartialEq)]
pub struct Header;

/// A single v1 output frame.
//
// Placeholder — fleshed out in the v1 slice.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame;

/// Parse a v1 recording from a buffered reader.
// Temporary stub; the real implementation lands in the v1 slice.
pub(crate) fn parse<R: BufRead>(_reader: R) -> Result<Asciicast<V1>, Error> {
    todo!("v1 parsing to be implemented later")
}
