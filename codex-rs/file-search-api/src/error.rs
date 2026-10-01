use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::fmt;

/// Stable failure categories; operation failure is separate from cleanup proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchErrorKind {
    InvalidInput,
    UnsupportedVersion,
    UnsupportedOption,
    UnknownLease,
    ClosedLease,
    StaleEpoch,
    ResourceExhausted,
    SearchFailed,
    TransportLost,
    ForcedShutdown,
}

/// A backend-neutral failure with bounded, non-sensitive diagnostic text.
///
/// The adapter must supply safe diagnostics, not credentials, full request
/// payloads or serialized native error chains. Bounding a diagnostic is not a
/// substitute for bounding logical messages before transport deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DecodedSearchError")]
pub struct SearchError {
    kind: SearchErrorKind,
    message: String,
}

impl SearchError {
    /// Maximum retained diagnostic length, measured in UTF-8 bytes.
    pub const MAX_MESSAGE_BYTES: usize = 2_048;

    /// Retains a UTF-8-safe prefix of a diagnostic up to the documented bound.
    pub fn new(kind: SearchErrorKind, message: impl AsRef<str>) -> Self {
        let message = message.as_ref();
        let mut end = message.len().min(Self::MAX_MESSAGE_BYTES);
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        Self {
            kind,
            message: message[..end].to_owned(),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> SearchErrorKind {
        self.kind
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Deserialize)]
struct DecodedSearchError {
    kind: SearchErrorKind,
    message: String,
}

impl TryFrom<DecodedSearchError> for SearchError {
    type Error = &'static str;

    fn try_from(value: DecodedSearchError) -> Result<Self, Self::Error> {
        if value.message.len() > Self::MAX_MESSAGE_BYTES {
            return Err("file-search diagnostic exceeds the byte limit");
        }
        Ok(Self {
            kind: value.kind,
            message: value.message,
        })
    }
}

impl fmt::Display for SearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl Error for SearchError {}

/// Cleanup evidence for an unsuccessful open, not a retry or replay policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "error", rename_all = "snake_case")]
pub enum StartCleanup {
    /// No backend operation was accepted; no lease requires cleanup.
    NotAdmitted,
    /// Accepted work was observed to reach its joined cleanup boundary.
    Confirmed,
    /// Cleanup failed or its receipt was lost; capacity must remain accounted.
    Unconfirmed(SearchError),
}

/// Keeps startup's operation error distinct from actual cleanup evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchStartError {
    pub operation: SearchError,
    pub cleanup: StartCleanup,
}

impl fmt::Display for SearchStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.operation.fmt(formatter)?;
        match &self.cleanup {
            StartCleanup::NotAdmitted => formatter.write_str(" (not admitted)"),
            StartCleanup::Confirmed => formatter.write_str(" (cleanup confirmed)"),
            StartCleanup::Unconfirmed(error) => {
                write!(formatter, " (cleanup unconfirmed: {error})")
            }
        }
    }
}

impl Error for SearchStartError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.operation)
    }
}
