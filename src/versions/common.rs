//! Types shared across `asciicast` versions.

use std::collections::BTreeMap;
use std::io::BufRead;

use rgb::RGB8;
use serde::{
    Deserialize,
    de::{self, DeserializeOwned, Deserializer},
};

use crate::Error;

/// Environment variables captured in a recording's header.
///
/// The spec models this as a JSON object of string keys to string values
/// (commonly `TERM` and `SHELL`).
pub type Env = BTreeMap<String, String>;

/// The reason a CSS `#rrggbb` colour string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ColorError {
    /// The string did not start with `#`.
    #[error("colour {input:?} must start with '#'")]
    MissingHashPrefix {
        /// The offending input.
        input: String,
    },
    /// The string did not have exactly six hex digits.
    #[error("colour {input:?} must have six hex digits, found {found}")]
    WrongLength {
        /// The offending input.
        input: String,
        /// The number of digits found after the `#`.
        found: usize,
    },
    /// The string contained a non-hexadecimal digit.
    #[error("colour {input:?} contains a non-hexadecimal digit")]
    NonHexDigit {
        /// The offending input.
        input: String,
    },
}

/// The reason a theme `palette` string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PaletteError {
    /// A colour within the palette was invalid.
    #[error("palette colour at index {index}: {source}")]
    Color {
        /// The zero-based index of the offending colour.
        index: usize,
        /// The underlying colour error.
        #[source]
        source: ColorError,
    },
    /// The palette did not contain 8 or 16 colours.
    #[error("palette must contain 8 or 16 colours, found {found}")]
    WrongLength {
        /// The number of colours found.
        found: usize,
    },
}

/// An RGB colour parsed from a CSS `#rrggbb` string.
///
/// Construct one with [`Rgb::new`] and read the channels via [`Rgb::r`],
/// [`Rgb::g`], and [`Rgb::b`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(RGB8);

impl Rgb {
    /// Construct a colour from its red, green, and blue channels.
    #[must_use]
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self(RGB8::new(r, g, b))
    }

    /// The red channel.
    #[must_use]
    pub fn r(&self) -> u8 {
        self.0.r
    }

    /// The green channel.
    #[must_use]
    pub fn g(&self) -> u8 {
        self.0.g
    }

    /// The blue channel.
    #[must_use]
    pub fn b(&self) -> u8 {
        self.0.b
    }

    /// Parse a single `#rrggbb` colour, validating the `#` prefix and six hex
    /// digits.
    pub(crate) fn parse(s: &str) -> Result<Self, ColorError> {
        let hex = s
            .strip_prefix('#')
            .ok_or_else(|| ColorError::MissingHashPrefix {
                input: s.to_owned(),
            })?;
        if hex.len() != 6 {
            return Err(ColorError::WrongLength {
                input: s.to_owned(),
                found: hex.len(),
            });
        }
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ColorError::NonHexDigit {
                input: s.to_owned(),
            });
        }
        let value = u32::from_str_radix(hex, 16).map_err(|_| ColorError::NonHexDigit {
            input: s.to_owned(),
        })?;
        let [_, r, g, b] = value.to_be_bytes();
        Ok(Self(RGB8::new(r, g, b)))
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(de::Error::custom)
    }
}

/// Deserialize a palette: a single colon-delimited string of `#rrggbb` colours.
///
/// The asciicast spec requires either 8 or 16 colours.
pub(crate) fn deserialize_palette<'de, D>(deserializer: D) -> Result<Vec<Rgb>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let palette = s
        .split(':')
        .enumerate()
        .map(|(index, colour)| {
            Rgb::parse(colour).map_err(|source| PaletteError::Color { index, source })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(de::Error::custom)?;
    if palette.len() != 8 && palette.len() != 16 {
        return Err(de::Error::custom(PaletteError::WrongLength {
            found: palette.len(),
        }));
    }
    Ok(palette)
}

/// A terminal colour scheme.
///
/// Shared by v2 (top-level `theme`) and v3 (nested under `term.theme`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Theme {
    /// Foreground (normal text) colour.
    pub fg: Rgb,
    /// Background colour.
    pub bg: Rgb,
    /// Colour palette: 8 or 16 colours.
    #[serde(deserialize_with = "deserialize_palette")]
    pub palette: Vec<Rgb>,
}

/// The new terminal dimensions carried by a resize (`r`) event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resize {
    /// Terminal width in columns.
    pub cols: u16,
    /// Terminal height in rows.
    pub rows: u16,
}

impl Resize {
    /// Parse a resize payload of the form `"COLSxROWS"` (e.g. `"80x24"`).
    pub(crate) fn parse(s: &str) -> Result<Self, Error> {
        let (cols, rows) = s
            .split_once('x')
            .ok_or_else(|| Error::MalformedEvent(format!("invalid resize payload: {s:?}")))?;
        Ok(Self {
            cols: cols
                .parse()
                .map_err(|_| Error::MalformedEvent(format!("invalid resize columns: {s:?}")))?,
            rows: rows
                .parse()
                .map_err(|_| Error::MalformedEvent(format!("invalid resize rows: {s:?}")))?,
        })
    }
}

/// The status carried by an exit (`x`) event.
///
/// An opaque newtype over the raw status so the representation can grow (e.g. to
/// distinguish a signal from an exit code) without breaking callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitStatus(i32);

impl ExitStatus {
    /// Parse an exit payload (a numeric string such as `"0"`).
    pub(crate) fn parse(s: &str) -> Result<Self, Error> {
        s.trim()
            .parse()
            .map(Self)
            .map_err(|_| Error::MalformedEvent(format!("invalid exit status: {s:?}")))
    }

    /// The numeric exit status.
    #[must_use]
    pub fn code(&self) -> i32 {
        self.0
    }
}

/// Parse a newline-delimited recording (the shared shape of v2 and v3): a header
/// object on the first non-blank line followed by one event array per line.
///
/// The caller supplies the per-version specifics: the expected version, whether
/// `#`-prefixed comment lines are skipped, how to read the `version` field from
/// the parsed header, and how to parse a single event line.
pub(crate) fn parse_ndjson<H, E, R, V, P>(
    reader: R,
    expected_version: u8,
    skip_comments: bool,
    header_version: V,
    parse_event: P,
) -> Result<(H, Vec<E>), Error>
where
    H: DeserializeOwned,
    R: BufRead,
    V: Fn(&H) -> u8,
    P: Fn(&str) -> Result<E, Error>,
{
    let mut lines = reader.lines();

    let header_line = loop {
        match lines.next() {
            Some(line) => {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                break line;
            }
            None => return Err(Error::MissingHeader),
        }
    };

    let header: H = serde_json::from_str(&header_line)?;
    let found = header_version(&header);
    if found != expected_version {
        return Err(Error::VersionMismatch {
            expected: expected_version,
            found,
        });
    }

    let mut events = Vec::new();
    for line in lines {
        let line = line?;
        let trimmed = line.trim();
        // Skip blank lines and, where the format allows them, comment lines.
        if trimmed.is_empty() || (skip_comments && trimmed.starts_with('#')) {
            continue;
        }
        events.push(parse_event(&line)?);
    }

    Ok((header, events))
}

#[cfg(test)]
mod tests {
    use super::{ColorError, Rgb};

    #[test]
    fn rgb_parse_returns_typed_errors() {
        assert_eq!(
            Rgb::parse("ffffff"),
            Err(ColorError::MissingHashPrefix {
                input: "ffffff".to_owned()
            })
        );
        assert_eq!(
            Rgb::parse("#fff"),
            Err(ColorError::WrongLength {
                input: "#fff".to_owned(),
                found: 3
            })
        );
        assert_eq!(
            Rgb::parse("#zzzzzz"),
            Err(ColorError::NonHexDigit {
                input: "#zzzzzz".to_owned()
            })
        );
        assert_eq!(Rgb::parse("#1a2b3c"), Ok(Rgb::new(0x1a, 0x2b, 0x3c)));
    }
}
