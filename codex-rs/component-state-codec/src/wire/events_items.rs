//! Trusted process transport DTOs. These deliberately bypass public rollout omissions.
//! External enum tags retain arbitrary-precision JSON values without Serde content buffering.

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::items::CommandExecutionItem")]
pub(crate) struct CommandExecutionItemWire {
    sandbox_type: Option<codex_protocol::sandbox::SandboxType>,
    #[serde(with = "super::events_scalars::optional_model_context")]
    model_context: Option<codex_protocol::items::ModelInvocationContext>,
    id: String,
    plugin_id: Option<String>,
    script_path: Option<String>,
    process_id: Option<String>,
    command: Vec<String>,
    cwd: codex_utils_path_uri::PathUri,
    #[serde(with = "super::events_paths::parsed_command::vec")]
    parsed_cmd: Vec<codex_protocol::parse_command::ParsedCommand>,
    source: codex_protocol::protocol::ExecCommandSource,
    interaction_input: Option<String>,
    status: codex_protocol::items::CommandExecutionStatus,
    stdout: Option<String>,
    stderr: Option<String>,
    aggregated_output: Option<String>,
    exit_code: Option<i32>,
    duration: Option<std::time::Duration>,
    formatted_output: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_extension_items::image_generation::ImageGenerationItem")]
pub(crate) struct ExtensionImageGenerationItemWire {
    id: String,
    status: String,
    revised_prompt: Option<String>,
    result: String,
    transparent_background: Option<bool>,
    failure: Option<codex_extension_items::image_generation::ImageGenerationFailure>,
    #[serde(with = "super::paths::absolute::option")]
    saved_path: Option<codex_utils_absolute_path::AbsolutePathBuf>,
    imagegen_request_id: Option<String>,
    generation_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_extension_items::ExtensionItem")]
pub(crate) enum ExtensionItemWire {
    ImageGeneration(
        #[serde(with = "ExtensionImageGenerationItemWire")]
        codex_extension_items::image_generation::ImageGenerationItem,
    ),
    Sleep(codex_extension_items::sleep::SleepItem),
    WebSearch(codex_extension_items::web_search::WebSearchItem),
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::items::TurnItem")]
pub(crate) enum TurnItemWire {
    UserMessage(
        #[serde(with = "super::events_paths::UserMessageItemWire")]
        codex_protocol::items::UserMessageItem,
    ),
    FunctionCallOutput(codex_protocol::items::FunctionCallOutputItem),
    HookPrompt(codex_protocol::items::HookPromptItem),
    AgentMessage(codex_protocol::items::AgentMessageItem),
    Plan(codex_protocol::items::PlanItem),
    Reasoning(codex_protocol::items::ReasoningItem),
    CommandExecution(
        #[serde(with = "CommandExecutionItemWire")] codex_protocol::items::CommandExecutionItem,
    ),
    DynamicToolCall(codex_protocol::items::DynamicToolCallItem),
    CollabAgentToolCall(
        #[serde(with = "CollabAgentToolCallItemWire")]
        codex_protocol::items::CollabAgentToolCallItem,
    ),
    SubAgentActivity(codex_protocol::items::SubAgentActivityItem),
    WebSearch(codex_protocol::items::WebSearchItem),
    ImageView(codex_protocol::items::ImageViewItem),
    Extension(#[serde(with = "ExtensionItemWire")] codex_extension_items::ExtensionItem),
    ImageGeneration(
        #[serde(with = "super::events_paths::ImageGenerationItemWire")]
        codex_protocol::items::ImageGenerationItem,
    ),
    EnteredReviewMode(codex_protocol::items::EnteredReviewModeItem),
    ExitedReviewMode(
        #[serde(with = "super::events_review::ExitedReviewModeItemWire")]
        codex_protocol::items::ExitedReviewModeItem,
    ),
    FileChange(
        #[serde(with = "super::events_file_changes::FileChangeItemWire")]
        codex_protocol::items::FileChangeItem,
    ),
    McpToolCall(
        #[serde(with = "super::events_json::McpToolCallItemWire")]
        codex_protocol::items::McpToolCallItem,
    ),
    ContextCompaction(codex_protocol::items::ContextCompactionItem),
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ItemStartedEvent")]
pub(crate) struct ItemStartedEventWire {
    thread_id: codex_protocol::ThreadId,
    turn_id: String,
    #[serde(with = "TurnItemWire")]
    item: codex_protocol::items::TurnItem,
    started_at_ms: i64,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ItemCompletedEvent")]
pub(crate) struct ItemCompletedEventWire {
    thread_id: codex_protocol::ThreadId,
    turn_id: String,
    #[serde(with = "TurnItemWire")]
    item: codex_protocol::items::TurnItem,
    started_at_ms: Option<i64>,
    completed_at_ms: i64,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::items::CollabAgentToolCallItem")]
pub(crate) struct CollabAgentToolCallItemWire {
    id: String,
    tool: codex_protocol::items::CollabAgentTool,
    status: codex_protocol::items::CollabAgentToolCallStatus,
    sender_thread_id: codex_protocol::ThreadId,
    receiver_thread_ids: Vec<codex_protocol::ThreadId>,
    receiver_agents: Vec<codex_protocol::protocol::CollabAgentRef>,
    prompt: Option<String>,
    model: Option<String>,
    #[serde(with = "super::events_scalars::optional_effort")]
    reasoning_effort: Option<codex_protocol::openai_models::ReasoningEffort>,
    agents_states:
        std::collections::HashMap<codex_protocol::ThreadId, codex_protocol::protocol::AgentStatus>,
}

remote_adapter!(
    image_generation_item,
    codex_extension_items::image_generation::ImageGenerationItem,
    ExtensionImageGenerationItemWire,
    "ExtensionImageGenerationItemWire"
);
