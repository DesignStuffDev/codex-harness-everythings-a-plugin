//! Trusted process transport DTOs. These deliberately bypass public rollout omissions.
//! External enum tags retain arbitrary-precision JSON values without Serde content buffering.

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ErrorEvent")]
pub(crate) struct ErrorEventWire {
    message: String,
    codex_error_info: Option<codex_protocol::protocol::CodexErrorInfo>,
    misalignment: Option<codex_protocol::protocol::MisalignmentErrorDetails>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::TurnCompleteEvent")]
pub(crate) struct TurnCompleteEventWire {
    turn_id: String,
    last_agent_message: Option<String>,
    #[serde(with = "errors::option")]
    error: Option<codex_protocol::protocol::ErrorEvent>,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    duration_ms: Option<i64>,
    time_to_first_token_ms: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::TurnAbortedEvent")]
pub(crate) struct TurnAbortedEventWire {
    turn_id: Option<String>,
    reason: codex_protocol::protocol::TurnAbortReason,
    #[serde(with = "errors::option")]
    error: Option<codex_protocol::protocol::ErrorEvent>,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    duration_ms: Option<i64>,
}

remote_adapter!(
    errors,
    codex_protocol::protocol::ErrorEvent,
    ErrorEventWire,
    "ErrorEventWire"
);

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::CollabAgentSpawnBeginEvent")]
pub(crate) struct CollabAgentSpawnBeginEventWire {
    call_id: String,
    started_at_ms: i64,
    sender_thread_id: codex_protocol::ThreadId,
    prompt: String,
    model: String,
    #[serde(with = "super::events_scalars::ReasoningEffortWire")]
    reasoning_effort: codex_protocol::openai_models::ReasoningEffort,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::CollabAgentSpawnEndEvent")]
pub(crate) struct CollabAgentSpawnEndEventWire {
    call_id: String,
    completed_at_ms: i64,
    sender_thread_id: codex_protocol::ThreadId,
    new_thread_id: Option<codex_protocol::ThreadId>,
    new_agent_nickname: Option<String>,
    new_agent_role: Option<String>,
    prompt: String,
    model: String,
    #[serde(with = "super::events_scalars::ReasoningEffortWire")]
    reasoning_effort: codex_protocol::openai_models::ReasoningEffort,
    status: codex_protocol::protocol::AgentStatus,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::HookRunSummary")]
pub(crate) struct HookRunSummaryWire {
    builtin: bool,
    id: String,
    event_name: codex_protocol::protocol::HookEventName,
    handler_type: codex_protocol::protocol::HookHandlerType,
    execution_mode: codex_protocol::protocol::HookExecutionMode,
    scope: codex_protocol::protocol::HookScope,
    #[serde(with = "super::paths::absolute")]
    source_path: codex_utils_absolute_path::AbsolutePathBuf,
    source: codex_protocol::protocol::HookSource,
    display_order: i64,
    status: codex_protocol::protocol::HookRunStatus,
    status_message: Option<String>,
    started_at: i64,
    completed_at: Option<i64>,
    duration_ms: Option<i64>,
    entries: Vec<codex_protocol::protocol::HookOutputEntry>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::HookStartedEvent")]
pub(crate) struct HookStartedEventWire {
    turn_id: Option<String>,
    #[serde(with = "HookRunSummaryWire")]
    run: codex_protocol::protocol::HookRunSummary,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::HookCompletedEvent")]
pub(crate) struct HookCompletedEventWire {
    turn_id: Option<String>,
    #[serde(with = "HookRunSummaryWire")]
    run: codex_protocol::protocol::HookRunSummary,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::SessionConfiguredEvent")]
pub(crate) struct SessionConfiguredEventWire {
    session_id: codex_protocol::SessionId,
    thread_id: codex_protocol::ThreadId,
    forked_from_id: Option<codex_protocol::ThreadId>,
    parent_thread_id: Option<codex_protocol::ThreadId>,
    #[serde(with = "super::events_scalars::optional_thread_source")]
    thread_source: Option<codex_protocol::protocol::ThreadSource>,
    thread_name: Option<String>,
    model: String,
    model_provider_id: String,
    service_tier: Option<String>,
    approval_policy: codex_protocol::protocol::AskForApproval,
    approvals_reviewer: codex_protocol::config_types::ApprovalsReviewer,
    #[serde(with = "super::events_permissions::PermissionProfileWire")]
    permission_profile: codex_protocol::models::PermissionProfile,
    active_permission_profile: Option<codex_protocol::models::ActivePermissionProfile>,
    #[serde(with = "super::paths::absolute")]
    cwd: codex_utils_absolute_path::AbsolutePathBuf,
    #[serde(with = "super::events_scalars::optional_effort")]
    reasoning_effort: Option<codex_protocol::openai_models::ReasoningEffort>,
    #[serde(with = "optional_events")]
    initial_messages: Option<Vec<codex_protocol::protocol::EventMsg>>,
    network_proxy: Option<codex_protocol::protocol::SessionNetworkProxyRuntime>,
    #[serde(with = "super::paths::native::option")]
    rollout_path: Option<std::path::PathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ThreadSettingsSnapshot")]
pub(crate) struct ThreadSettingsSnapshotWire {
    model: String,
    model_provider_id: String,
    service_tier: Option<String>,
    approval_policy: codex_protocol::protocol::AskForApproval,
    approvals_reviewer: codex_protocol::config_types::ApprovalsReviewer,
    #[serde(with = "super::events_permissions::PermissionProfileWire")]
    permission_profile: codex_protocol::models::PermissionProfile,
    active_permission_profile: Option<codex_protocol::models::ActivePermissionProfile>,
    #[serde(with = "super::paths::absolute")]
    cwd: codex_utils_absolute_path::AbsolutePathBuf,
    #[serde(with = "super::paths::absolute::option_vec")]
    runtime_workspace_roots: Option<Vec<codex_utils_absolute_path::AbsolutePathBuf>>,
    #[serde(with = "super::events_scalars::optional_effort")]
    reasoning_effort: Option<codex_protocol::openai_models::ReasoningEffort>,
    reasoning_summary: Option<codex_protocol::config_types::ReasoningSummary>,
    personality: Option<codex_protocol::config_types::Personality>,
    #[serde(with = "super::events_scalars::CollaborationModeWire")]
    collaboration_mode: codex_protocol::config_types::CollaborationMode,
    disabled_plugin_ids: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ThreadSettingsAppliedEvent")]
pub(crate) struct ThreadSettingsAppliedEventWire {
    thread_id: Option<codex_protocol::ThreadId>,
    #[serde(with = "ThreadSettingsSnapshotWire")]
    thread_settings: codex_protocol::protocol::ThreadSettingsSnapshot,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::RawResponseItemEvent")]
pub(crate) struct RawResponseItemEventWire {
    #[serde(with = "super::response::TrustedResponseItem")]
    item: codex_protocol::models::ResponseItem,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::RawResponseCompletedEvent")]
pub(crate) struct RawResponseCompletedEventWire {
    response_id: String,
    #[serde(with = "optional_usage")]
    token_usage: Option<codex_protocol::protocol::TokenUsage>,
    #[serde(with = "super::events_json::response_usage::option")]
    usage_metadata: Option<codex_protocol::ResponseUsageMetadata>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::TokenCountEvent")]
pub(crate) struct TokenCountEventWire {
    #[serde(with = "optional_usage_info")]
    info: Option<codex_protocol::protocol::TokenUsageInfo>,
    #[serde(with = "super::events_review::optional_rate_limits")]
    rate_limits: Option<codex_protocol::protocol::RateLimitSnapshot>,
}

pub(crate) mod optional_usage {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::super::usage::TokenUsageWire")]
        &'a codex_protocol::protocol::TokenUsage,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::super::usage::TokenUsageWire")] codex_protocol::protocol::TokenUsage,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::protocol::TokenUsage>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::protocol::TokenUsage>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod optional_usage_info {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::super::usage::TokenUsageInfoWire")]
        &'a codex_protocol::protocol::TokenUsageInfo,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::super::usage::TokenUsageInfoWire")]
        codex_protocol::protocol::TokenUsageInfo,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::protocol::TokenUsageInfo>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::protocol::TokenUsageInfo>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod optional_events {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::super::events::EventMsgWire")]
        &'a codex_protocol::protocol::EventMsg,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::super::events::EventMsgWire")] codex_protocol::protocol::EventMsg,
    );
    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<Vec<codex_protocol::protocol::EventMsg>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let owned = value
            .as_ref()
            .map(|items| items.iter().map(Borrowed).collect::<Vec<_>>());
        serde::Serialize::serialize(&owned, serializer)
    }
    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<codex_protocol::protocol::EventMsg>>, D::Error> {
        let owned = <Option<Vec<Owned>> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(owned.map(|items| items.into_iter().map(|item| item.0).collect()))
    }
}
