use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use serde::Deserialize;
use serde::Serialize;

/// Explicit portable result shape; this does not expose Rust's Result encoding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationOutcome {
    Ok,
    Error { error: SearchError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireCloseOutcome {
    pub operation: OperationOutcome,
    pub cleanup: CloseCleanup,
}

impl From<SearchCloseOutcome> for WireCloseOutcome {
    fn from(value: SearchCloseOutcome) -> Self {
        Self {
            operation: match value.operation {
                Ok(()) => OperationOutcome::Ok,
                Err(error) => OperationOutcome::Error { error },
            },
            cleanup: value.cleanup,
        }
    }
}

impl From<WireCloseOutcome> for SearchCloseOutcome {
    fn from(value: WireCloseOutcome) -> Self {
        Self {
            operation: match value.operation {
                OperationOutcome::Ok => Ok(()),
                OperationOutcome::Error { error } => Err(error),
            },
            cleanup: value.cleanup,
        }
    }
}
