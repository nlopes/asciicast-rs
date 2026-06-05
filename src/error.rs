//! Error type for parsing `asciicast` files.

/// Errors that can occur while parsing an `asciicast` recording.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// An I/O error occurred while reading the input.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The input was not valid JSON for the expected shape.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// The input did not contain a header line.
    #[error("missing header")]
    MissingHeader,

    /// The `version` field held a value this crate does not support.
    #[error("unknown asciicast version: {0}")]
    UnknownVersion(u8),

    /// A specific version was requested but the input declared another.
    #[error("version mismatch: expected v{expected}, found v{found}")]
    VersionMismatch {
        /// The version that was requested.
        expected: u8,
        /// The version found in the input.
        found: u8,
    },

    /// An event's payload could not be parsed (e.g. a malformed resize or exit).
    #[error("malformed event payload: {0}")]
    MalformedEvent(String),
}
