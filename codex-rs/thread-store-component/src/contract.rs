use std::path::PathBuf;

use codex_protocol::ThreadId;
use codex_protocol::protocol::HistoryPosition;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_thread_store::PersistContext;
use codex_thread_store::PrepareForkParams;
use codex_thread_store::ThreadMetadataPatch;
use codex_thread_store::ThreadStore;
use serde::Deserialize;
use serde::Serialize;

pub use codex_component_host::THREAD_STORE_CONTRACT_VERSION;
pub const OPEN_METHOD: &str = "thread_store/open";
pub const CALL_METHOD: &str = "thread_store/call";
pub const RELEASE_FORK_METHOD: &str = "thread_store/release_fork";

/// Filesystem roots shared with host-owned memories, goals, queue and agent graph.
/// Contract v2 preserves this explicit compatibility dependency; it does not
/// imply that these auxiliary services have moved into the storage process.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalStoragePaths {
    #[serde(with = "codex_component_state_codec::native_path")]
    pub codex_home: PathBuf,
    #[serde(with = "codex_component_state_codec::native_path")]
    pub sqlite_home: PathBuf,
}

/// Initialization for a local-compatible implementation. The host supplies
/// resolved paths, never Config or live database pointers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageInitialization {
    pub contract_version: u32,
    pub paths: LocalStoragePaths,
    pub default_model_provider_id: String,
    pub state_db_enabled: bool,
    pub startup_migration: bool,
    pub startup_compression: bool,
}

/// Negotiated features are available synchronously through the native trait.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageCapabilities {
    pub contract_version: u32,
    pub default_history_mode: ThreadHistoryMode,
    pub thread_sections: bool,
    pub thread_attachments: bool,
    pub projects: bool,
    pub paginated_history_lists: bool,
    pub rollout_maintenance: bool,
    pub rollout_path_reads: bool,
    pub shared_local_sqlite: LocalStoragePaths,
}

impl StorageCapabilities {
    pub fn for_store(store: &dyn ThreadStore, paths: LocalStoragePaths) -> Self {
        Self {
            contract_version: THREAD_STORE_CONTRACT_VERSION,
            default_history_mode: store.default_history_mode(),
            thread_sections: store.supports_thread_sections(),
            thread_attachments: store.supports_thread_attachments(),
            projects: store.supports_projects(),
            paginated_history_lists: store.supports_paginated_history_lists(),
            rollout_maintenance: store.supports_rollout_maintenance(),
            rollout_path_reads: store.supports_rollout_path_reads(),
            shared_local_sqlite: paths,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingMetadataParams {
    pub thread_id: ThreadId,
    #[serde(with = "crate::remote::thread_metadata_patch")]
    pub patch: ThreadMetadataPatch,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistParams {
    pub thread_id: ThreadId,
    pub context: PersistContext,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CopyAttachmentsParams {
    pub source_thread_id: ThreadId,
    pub destination_thread_id: ThreadId,
}

/// The client allocates the lease ID before sending prepare, so cancellation
/// can release a reservation even before its native preparation completes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepareForkRequest {
    pub lease_id: String,
    pub params: PrepareForkParams,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseForkRequest {
    pub lease_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedForkResponse {
    pub source_thread_id: ThreadId,
    pub history_base: Option<HistoryPosition>,
    // Use the existing DTO for its typed rollout items, avoiding a dependency on
    // LocalThreadStore implementation details in the external contract.
    #[serde(with = "crate::remote::stored_model_context")]
    pub model_context: codex_thread_store::StoredModelContext,
}
