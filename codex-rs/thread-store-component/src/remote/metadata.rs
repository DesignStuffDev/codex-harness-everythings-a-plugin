//! Storage-specific lossless mirrors; native serde stays unchanged.

use chrono::DateTime;
use chrono::Utc;
use codex_protocol::ThreadId;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::ThreadMemoryMode as MemoryMode;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::protocol::TokenUsage;
use codex_thread_store::ClearableField;
use codex_thread_store::GitInfoPatch;
use codex_thread_store::ThreadMetadataPatch;
use codex_thread_store::UpdateThreadMetadataParams;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(remote = "ThreadMetadataPatch", deny_unknown_fields)]
struct ThreadMetadataPatchWire {
    #[serde(with = "super::clearable")]
    name: ClearableField<String>,
    #[serde(with = "codex_component_state_codec::native_path::option")]
    rollout_path: Option<PathBuf>,
    preview: Option<String>,
    title: Option<String>,
    model_provider: Option<String>,
    model: Option<String>,
    #[serde(with = "super::clearable::reasoning_effort")]
    reasoning_effort: ClearableField<ReasoningEffort>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    advance_recency_at: Option<DateTime<Utc>>,
    source: Option<SessionSource>,
    creator_user_id: Option<String>,
    creator_account_id: Option<String>,
    originator: Option<String>,
    #[serde(with = "super::clearable::thread_source")]
    thread_source: ClearableField<ThreadSource>,
    #[serde(with = "super::clearable")]
    agent_nickname: ClearableField<String>,
    #[serde(with = "super::clearable")]
    agent_role: ClearableField<String>,
    #[serde(with = "super::clearable")]
    agent_path: ClearableField<String>,
    #[serde(with = "codex_component_state_codec::native_path::option")]
    cwd: Option<PathBuf>,
    cli_version: Option<String>,
    approval_mode: Option<AskForApproval>,
    #[serde(with = "codex_component_state_codec::permission_profile::option")]
    permission_profile: Option<PermissionProfile>,
    #[serde(with = "codex_component_state_codec::token_usage::option")]
    token_usage: Option<TokenUsage>,
    first_user_message: Option<String>,
    git_info: Option<GitInfoPatch>,
    memory_mode: Option<MemoryMode>,
    #[serde(with = "super::clearable")]
    project_id: ClearableField<String>,
    daybreak_enabled: Option<bool>,
}
remote_adapter!(
    thread_metadata_patch,
    ThreadMetadataPatch,
    ThreadMetadataPatchWire,
    "ThreadMetadataPatchWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "UpdateThreadMetadataParams", deny_unknown_fields)]
struct UpdateThreadMetadataParamsWire {
    thread_id: ThreadId,
    #[serde(with = "super::thread_metadata_patch")]
    patch: ThreadMetadataPatch,
    include_archived: bool,
}
remote_adapter!(
    update_thread_metadata_params,
    UpdateThreadMetadataParams,
    UpdateThreadMetadataParamsWire,
    "UpdateThreadMetadataParamsWire"
);
