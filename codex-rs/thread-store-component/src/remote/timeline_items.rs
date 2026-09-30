//! App-server item adapters preserving the native storage API's complete values.

use codex_app_server_protocol as api;
use codex_component_state_codec as codec;
use codex_protocol::items::ModelInvocationContext;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::ImageDetail;
use codex_protocol::models::MessagePhase;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::sandbox::SandboxType;
use codex_utils_path_uri::LegacyAppPathString;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(remote = "api::ThreadItem")]
enum ThreadItemWire {
    UserMessage {
        id: String,
        client_id: Option<String>,
        #[serde(with = "user_inputs")]
        content: Vec<api::UserInput>,
    },
    HookPrompt {
        id: String,
        fragments: Vec<api::HookPromptFragment>,
    },
    AgentMessage {
        id: String,
        text: String,
        phase: Option<MessagePhase>,
        memory_citation: Option<api::MemoryCitation>,
        delivery: Option<api::AgentMessageDelivery>,
        questions: Option<Vec<api::AsyncUserInputQuestion>>,
    },
    FunctionCallOutput {
        id: String,
        name: String,
        namespace: Option<String>,
        output: FunctionCallOutputBody,
    },
    Plan {
        id: String,
        text: String,
    },
    Reasoning {
        id: String,
        summary: Vec<String>,
        content: Vec<String>,
    },
    CommandExecution {
        sandbox_type: Option<SandboxType>,
        #[serde(with = "codec::model_context::option")]
        model_context: Option<ModelInvocationContext>,
        id: String,
        plugin_id: Option<String>,
        script_path: Option<String>,
        command: String,
        cwd: LegacyAppPathString,
        process_id: Option<String>,
        source: api::CommandExecutionSource,
        status: api::CommandExecutionStatus,
        command_actions: Vec<api::CommandAction>,
        aggregated_output: Option<String>,
        exit_code: Option<i32>,
        duration_ms: Option<i64>,
    },
    FileChange {
        id: String,
        #[serde(with = "file_changes")]
        changes: Vec<api::FileUpdateChange>,
        status: api::PatchApplyStatus,
    },
    McpToolCall {
        id: String,
        server: String,
        tool: String,
        status: api::McpToolCallStatus,
        arguments: Value,
        app_context: Option<api::McpToolCallAppContext>,
        mcp_app_resource_uri: Option<String>,
        mcp_app_ui: Option<api::McpAppUi>,
        plugin_id: Option<String>,
        read_only_hint: Option<bool>,
        #[serde(with = "mcp_result")]
        result: Option<Box<api::McpToolCallResult>>,
        error: Option<api::McpToolCallError>,
        duration_ms: Option<i64>,
    },
    DynamicToolCall {
        id: String,
        namespace: Option<String>,
        tool: String,
        arguments: Value,
        status: api::DynamicToolCallStatus,
        content_items: Option<Vec<api::DynamicToolCallOutputContentItem>>,
        success: Option<bool>,
        duration_ms: Option<i64>,
    },
    CollabAgentToolCall {
        id: String,
        tool: api::CollabAgentTool,
        status: api::CollabAgentToolCallStatus,
        sender_thread_id: String,
        receiver_thread_ids: Vec<String>,
        prompt: Option<String>,
        model: Option<String>,
        #[serde(with = "codec::reasoning_effort::option")]
        reasoning_effort: Option<ReasoningEffort>,
        agents_states: HashMap<String, api::CollabAgentState>,
    },
    SubAgentActivity {
        id: String,
        kind: api::SubAgentActivityKind,
        agent_thread_id: String,
        agent_path: String,
    },
    WebSearch(api::WebSearchItem),
    ImageView {
        id: String,
        path: LegacyAppPathString,
    },
    Sleep(api::SleepItem),
    ImageGeneration(#[serde(with = "codec::image_generation_item")] api::ImageGenerationItem),
    EnteredReviewMode {
        id: String,
        review: String,
    },
    ExitedReviewMode {
        id: String,
        review: String,
    },
    ContextCompaction {
        id: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "api::UserInput")]
enum UserInputWire {
    Text {
        text: String,
        text_elements: Vec<api::TextElement>,
    },
    Image {
        image: api::ImageReference,
        detail: Option<ImageDetail>,
    },
    LocalImage {
        detail: Option<ImageDetail>,
        #[serde(with = "codec::native_path")]
        path: PathBuf,
    },
    Audio {
        url: String,
    },
    LocalAudio {
        #[serde(with = "codec::native_path")]
        path: PathBuf,
    },
    Skill {
        name: String,
        #[serde(with = "codec::native_path")]
        path: PathBuf,
    },
    Mention {
        name: String,
        path: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "api::McpToolCallResult")]
struct McpToolCallResultWire {
    content: Vec<Value>,
    #[serde(with = "codec::optional_json")]
    structured_content: Option<Value>,
    #[serde(with = "codec::optional_json")]
    meta: Option<Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "api::FileUpdateChange")]
struct FileUpdateChangeWire {
    path: String,
    #[serde(with = "PatchChangeKindWire")]
    kind: api::PatchChangeKind,
    diff: String,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "api::PatchChangeKind")]
enum PatchChangeKindWire {
    Add,
    Delete,
    Update {
        #[serde(with = "codec::native_path::option")]
        move_path: Option<PathBuf>,
    },
}

mod file_changes {
    use super::*;

    #[derive(Serialize)]
    #[serde(transparent)]
    struct Borrowed<'a>(#[serde(with = "FileUpdateChangeWire")] &'a api::FileUpdateChange);

    #[derive(Deserialize)]
    #[serde(transparent)]
    struct Owned(#[serde(with = "FileUpdateChangeWire")] api::FileUpdateChange);

    pub(super) fn serialize<S: serde::Serializer>(
        value: &[api::FileUpdateChange],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(value.iter().map(Borrowed))
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<api::FileUpdateChange>, D::Error> {
        Vec::<Owned>::deserialize(deserializer)
            .map(|items| items.into_iter().map(|item| item.0).collect())
    }
}

mod mcp_result {
    use super::*;

    #[derive(Serialize)]
    #[serde(transparent)]
    struct Borrowed<'a>(#[serde(with = "McpToolCallResultWire")] &'a api::McpToolCallResult);

    #[derive(Deserialize)]
    #[serde(transparent)]
    struct Owned(#[serde(with = "McpToolCallResultWire")] api::McpToolCallResult);

    pub(super) fn serialize<S: serde::Serializer>(
        value: &Option<Box<api::McpToolCallResult>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.as_deref().map(Borrowed).serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Box<api::McpToolCallResult>>, D::Error> {
        Option::<Owned>::deserialize(deserializer).map(|value| value.map(|value| Box::new(value.0)))
    }
}

pub(super) mod boxed {
    use super::*;

    pub(crate) fn serialize<S: serde::Serializer>(
        item: &api::ThreadItem,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        ThreadItemWire::serialize(item, serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Box<api::ThreadItem>, D::Error> {
        ThreadItemWire::deserialize(deserializer).map(Box::new)
    }
}

mod user_inputs {
    use super::*;

    #[derive(Serialize)]
    #[serde(transparent)]
    struct Borrowed<'a>(#[serde(with = "UserInputWire")] &'a api::UserInput);

    #[derive(Deserialize)]
    #[serde(transparent)]
    struct Owned(#[serde(with = "UserInputWire")] api::UserInput);

    pub(super) fn serialize<S: serde::Serializer>(
        value: &[api::UserInput],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(value.iter().map(Borrowed))
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<api::UserInput>, D::Error> {
        Vec::<Owned>::deserialize(deserializer)
            .map(|items| items.into_iter().map(|item| item.0).collect())
    }
}
