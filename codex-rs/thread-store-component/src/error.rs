use codex_protocol::ThreadId;
use codex_thread_store::ThreadStoreError;
use serde::Deserialize;
use serde::Serialize;

/// Stable application errors, distinct from transport failures with unknown
/// completion. Error messages and operation names remain owned wire values.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum StorageError {
    ThreadNotFound { thread_id: ThreadId },
    InvalidRequest { message: String },
    Conflict { message: String },
    Unsupported { operation: String },
    Internal { message: String },
}

impl From<ThreadStoreError> for StorageError {
    fn from(error: ThreadStoreError) -> Self {
        match error {
            ThreadStoreError::ThreadNotFound { thread_id } => Self::ThreadNotFound { thread_id },
            ThreadStoreError::InvalidRequest { message } => Self::InvalidRequest { message },
            ThreadStoreError::Conflict { message } => Self::Conflict { message },
            ThreadStoreError::Unsupported { operation } => Self::Unsupported {
                operation: operation.to_owned(),
            },
            ThreadStoreError::Internal { message } => Self::Internal { message },
        }
    }
}

impl StorageError {
    pub(crate) fn into_native(self) -> ThreadStoreError {
        match self {
            Self::ThreadNotFound { thread_id } => ThreadStoreError::ThreadNotFound { thread_id },
            Self::InvalidRequest { message } => ThreadStoreError::InvalidRequest { message },
            Self::Conflict { message } => ThreadStoreError::Conflict { message },
            Self::Internal { message } => ThreadStoreError::Internal { message },
            Self::Unsupported { operation } => match native_operation(&operation) {
                Some(operation) => ThreadStoreError::Unsupported { operation },
                None => ThreadStoreError::Internal {
                    message: format!(
                        "storage component reported unknown unsupported operation: {operation}"
                    ),
                },
            },
        }
    }
}

// The native API uses &'static str. Recognize the actual contract vocabulary
// without leaking arbitrary plugin-owned strings into permanent allocations.
fn native_operation(operation: &str) -> Option<&'static str> {
    const OPERATIONS: &[&str] = &[
        "paginated_threads",
        "paginated_history",
        "projects",
        "prepare_fork",
        "stage_pending_thread_metadata",
        "remove_pending_thread_metadata",
        "load_latest_model_context",
        "revert_thread",
        "copy_thread_attachments",
        "list_turns",
        "list_items",
        "thread/search",
        "thread/searchOccurrences",
        "thread/timeline/list",
        "thread/section/move",
        "thread/attachment/add",
        "thread/attachment/list",
        "thread/attachment/remove",
        "threadSection/list",
        "threadSection/create",
        "threadSection/update",
        "threadSection/delete",
        "project/list",
        "project/read",
        "project/create",
        "project/update",
        "project/move",
        "project/delete",
        "rollout_maintenance",
    ];
    OPERATIONS.iter().copied().find(|known| *known == operation)
}

pub(crate) fn transport_error(error: impl std::fmt::Display) -> ThreadStoreError {
    ThreadStoreError::Internal {
        message: format!(
            "storage component transport failed; operation completion may be unknown; no retry was attempted: {error}"
        ),
    }
}
