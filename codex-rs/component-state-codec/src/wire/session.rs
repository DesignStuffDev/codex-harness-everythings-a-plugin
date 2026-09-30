//! Session records use the trusted shape, without legacy conversion or flattening.
use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::capabilities::SelectedCapabilityRoot;
use codex_protocol::config_types::ApprovalsReviewer;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::MultiAgentMode;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
use codex_protocol::dynamic_tools::DynamicToolFunctionSpec;
use codex_protocol::dynamic_tools::DynamicToolNamespaceSpec;
use codex_protocol::dynamic_tools::DynamicToolNamespaceTool;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::GitInfo;
use codex_protocol::protocol::HistoryPosition;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::RawFileSystemSandboxPolicy;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::protocol::SessionContextWindow;
use codex_protocol::protocol::SessionMeta;
use codex_protocol::protocol::SessionMetaLine;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::protocol::TurnContextItem;
use codex_protocol::protocol::TurnContextNetworkItem;
use codex_protocol::turn_input::CyberAccessProgram;
use codex_utils_absolute_path::AbsolutePathBuf;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(remote = "SessionMeta", deny_unknown_fields)]
pub(crate) struct SessionMetaWire {
    creator_user_id: Option<String>,
    creator_account_id: Option<String>,
    session_id: SessionId,
    id: ThreadId,
    forked_from_id: Option<ThreadId>,
    forked_from_ordinal_exclusive: Option<u64>,
    parent_thread_id: Option<ThreadId>,
    timestamp: String,
    #[serde(with = "super::paths::native")]
    cwd: PathBuf,
    #[serde(with = "super::paths::native::option_vec")]
    runtime_workspace_roots: Option<Vec<PathBuf>>,
    originator: String,
    cli_version: String,
    source: SessionSource,
    #[serde(with = "super::events_scalars::optional_thread_source")]
    thread_source: Option<ThreadSource>,
    agent_nickname: Option<String>,
    agent_role: Option<String>,
    agent_path: Option<String>,
    model_provider: Option<String>,
    base_instructions: Option<BaseInstructions>,
    #[serde(with = "dynamic_tool::option_vec")]
    dynamic_tools: Option<Vec<DynamicToolSpec>>,
    selected_capability_roots: Vec<SelectedCapabilityRoot>,
    memory_mode: Option<String>,
    history_mode: ThreadHistoryMode,
    history_base: Option<HistoryPosition>,
    subagent_history_start_ordinal: Option<u64>,
    multi_agent_version: Option<MultiAgentVersion>,
    context_window: Option<SessionContextWindow>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "TurnContextItem", deny_unknown_fields)]
pub(crate) struct TurnContextItemWire {
    turn_id: Option<String>,
    root_turn_id: Option<String>,
    disabled_plugin_ids: Option<Vec<String>>,
    #[serde(with = "super::paths::absolute")]
    cwd: AbsolutePathBuf,
    #[serde(with = "super::paths::absolute::option_vec")]
    workspace_roots: Option<Vec<AbsolutePathBuf>>,
    current_date: Option<String>,
    timezone: Option<String>,
    approval_policy: AskForApproval,
    approvals_reviewer: Option<ApprovalsReviewer>,
    #[serde(with = "super::events_permissions::SandboxPolicyWire")]
    sandbox_policy: SandboxPolicy,
    #[serde(with = "super::events_permissions::optional_permission_profile")]
    permission_profile: Option<PermissionProfile>,
    active_permission_profile: Option<ActivePermissionProfile>,
    network: Option<TurnContextNetworkItem>,
    file_system_sandbox_policy: Option<RawFileSystemSandboxPolicy>,
    model: String,
    comp_hash: Option<String>,
    personality: Option<Personality>,
    #[serde(with = "super::events_scalars::optional_collaboration")]
    collaboration_mode: Option<CollaborationMode>,
    multi_agent_version: Option<MultiAgentVersion>,
    multi_agent_mode: Option<MultiAgentMode>,
    realtime_active: Option<bool>,
    cyber_access_program: Option<CyberAccessProgram>,
    #[serde(with = "super::events_scalars::optional_effort")]
    effort: Option<ReasoningEffortConfig>,
    summary: ReasoningSummaryConfig,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "SessionMetaLine", deny_unknown_fields)]
pub(crate) struct SessionMetaLineWire {
    #[serde(with = "SessionMetaWire")]
    meta: SessionMeta,
    git: Option<GitInfo>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "DynamicToolSpec")]
enum DynamicToolWire {
    Function(DynamicToolFunctionSpec),
    Namespace(#[serde(with = "DynamicNamespaceWire")] DynamicToolNamespaceSpec),
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "DynamicToolNamespaceSpec", deny_unknown_fields)]
struct DynamicNamespaceWire {
    name: String,
    description: String,
    #[serde(with = "namespace_tool::vec")]
    tools: Vec<DynamicToolNamespaceTool>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "DynamicToolNamespaceTool")]
enum DynamicNamespaceToolWire {
    Function(DynamicToolFunctionSpec),
}

remote_adapter!(
    dynamic_tool,
    DynamicToolSpec,
    DynamicToolWire,
    "DynamicToolWire"
);
remote_adapter!(
    namespace_tool,
    DynamicToolNamespaceTool,
    DynamicNamespaceToolWire,
    "DynamicNamespaceToolWire"
);
remote_adapter!(
    turn_context,
    TurnContextItem,
    TurnContextItemWire,
    "TurnContextItemWire"
);
