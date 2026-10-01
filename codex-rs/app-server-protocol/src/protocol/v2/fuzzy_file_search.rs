use crate::JsonSchema;
use crate::TS;
use serde::Deserialize;
use serde::Serialize;

/// An operation failure in a fuzzy file-search session.
///
/// This notification does not confirm that session cleanup has completed.
/// The session stop response and server shutdown own their cleanup results.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub struct FuzzyFileSearchSessionFailedNotification {
    /// Caller-supplied session ID, limited to 256 UTF-8 bytes at admission.
    pub session_id: String,
    /// Query associated with the failure, bounded by the effective search policy.
    pub query: String,
    pub error: FuzzyFileSearchSessionError,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub struct FuzzyFileSearchSessionError {
    pub kind: FuzzyFileSearchSessionErrorKind,
    /// Diagnostic bounded by the emitting adapter to 2048 UTF-8 bytes.
    pub message: String,
}

/// Stable wire categories independent of the selected search implementation.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum FuzzyFileSearchSessionErrorKind {
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

#[cfg(test)]
#[path = "fuzzy_file_search_tests.rs"]
mod tests;
