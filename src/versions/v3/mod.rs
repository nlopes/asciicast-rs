//! asciicast [v3](https://docs.asciinema.org/manual/asciicast/v3/).
//!
//! A v3 recording is newline-delimited JSON: a header object on the first line
//! followed by one `[interval, code, data]` event array per line. Comment lines
//! beginning with `#` are ignored.

use std::io::BufRead;

use crate::{Asciicast, Error, versions::V3};

/// The header (first line) of a v3 recording.
//
// Placeholder — fleshed out in the v3 slice.
#[derive(Debug, Clone, PartialEq)]
pub struct Header;

/// A single v3 event.
//
// Placeholder — fleshed out in the v3 slice.
#[derive(Debug, Clone, PartialEq)]
pub struct Event;

/// Parse a v3 recording from a buffered reader.
// Temporary stub; the real implementation lands in the v3 slice.
pub(crate) fn parse<R: BufRead>(_reader: R) -> Result<Asciicast<V3>, Error> {
    todo!("v3 parsing to be implemented later")
}
