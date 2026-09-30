//! Storage-specific lossless mirrors; native serde stays unchanged.

use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::capabilities::SelectedCapabilityRoot;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::models::BaseInstructions;
use codex_protocol::protocol::HistoryPosition;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::ThreadMemoryMode as MemoryMode;
use codex_protocol::protocol::ThreadSource;
use codex_rollout::RolloutItem;
use codex_thread_store::AppendThreadItemsParams;
use codex_thread_store::CreateThreadParams;
use codex_thread_store::ExtraConfig;
use codex_thread_store::ResumeThreadParams;
use codex_thread_store::StoredModelContext;
use codex_thread_store::StoredThreadHistory;
use codex_thread_store::ThreadPersistenceMetadata;
use codex_utils_absolute_path::AbsolutePathBuf;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
#[serde(remote = "ThreadPersistenceMetadata", deny_unknown_fields)]
struct ThreadPersistenceMetadataWire {
    #[serde(with = "codex_component_state_codec::native_path::option")]
    cwd: Option<PathBuf>,
    model_provider: String,
    memory_mode: MemoryMode,
}
remote_adapter!(
    thread_persistence_metadata,
    ThreadPersistenceMetadata,
    ThreadPersistenceMetadataWire,
    "ThreadPersistenceMetadataWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "CreateThreadParams", deny_unknown_fields)]
struct CreateThreadParamsWire {
    creator_user_id: Option<String>,
    creator_account_id: Option<String>,
    session_id: SessionId,
    thread_id: ThreadId,
    extra_config: Option<ExtraConfig>,
    forked_from_id: Option<ThreadId>,
    parent_thread_id: Option<ThreadId>,
    source: SessionSource,
    #[serde(with = "codex_component_state_codec::thread_source::option")]
    thread_source: Option<ThreadSource>,
    originator: String,
    base_instructions: BaseInstructions,
    #[serde(with = "codex_component_state_codec::dynamic_tool::vec")]
    dynamic_tools: Vec<DynamicToolSpec>,
    selected_capability_roots: Vec<SelectedCapabilityRoot>,
    multi_agent_version: Option<MultiAgentVersion>,
    history_mode: ThreadHistoryMode,
    history_base: Option<HistoryPosition>,
    subagent_history_start_ordinal: Option<u64>,
    initial_window_id: String,
    #[serde(with = "codex_component_state_codec::absolute_path::option_vec")]
    runtime_workspace_roots: Option<Vec<AbsolutePathBuf>>,
    #[serde(with = "super::thread_persistence_metadata")]
    metadata: ThreadPersistenceMetadata,
}
remote_adapter!(
    create_thread_params,
    CreateThreadParams,
    CreateThreadParamsWire,
    "CreateThreadParamsWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "ResumeThreadParams", deny_unknown_fields)]
struct ResumeThreadParamsWire {
    thread_id: ThreadId,
    #[serde(with = "codex_component_state_codec::native_path::option")]
    rollout_path: Option<PathBuf>,
    #[serde(with = "super::history_arc")]
    history: Option<Arc<Vec<RolloutItem>>>,
    include_archived: bool,
    #[serde(with = "super::thread_persistence_metadata")]
    metadata: ThreadPersistenceMetadata,
}
remote_adapter!(
    resume_thread_params,
    ResumeThreadParams,
    ResumeThreadParamsWire,
    "ResumeThreadParamsWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "AppendThreadItemsParams", deny_unknown_fields)]
struct AppendThreadItemsParamsWire {
    thread_id: ThreadId,
    #[serde(with = "codex_component_state_codec::rollout_item::vec")]
    items: Vec<RolloutItem>,
}
remote_adapter!(
    append_thread_items_params,
    AppendThreadItemsParams,
    AppendThreadItemsParamsWire,
    "AppendThreadItemsParamsWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "StoredThreadHistory", deny_unknown_fields)]
struct StoredThreadHistoryWire {
    thread_id: ThreadId,
    #[serde(with = "codex_component_state_codec::rollout_item::vec")]
    items: Vec<RolloutItem>,
}
remote_adapter!(
    stored_thread_history,
    StoredThreadHistory,
    StoredThreadHistoryWire,
    "StoredThreadHistoryWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "StoredModelContext", deny_unknown_fields)]
struct StoredModelContextWire {
    thread_id: ThreadId,
    #[serde(with = "codex_component_state_codec::rollout_item::vec")]
    items: Vec<RolloutItem>,
}
remote_adapter!(
    stored_model_context,
    StoredModelContext,
    StoredModelContextWire,
    "StoredModelContextWire"
);
