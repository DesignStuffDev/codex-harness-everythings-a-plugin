//! Explicit response-item DTO; no provider deserialization policy is reused.
use codex_protocol::ResponseItemId;
use codex_protocol::models::AgentMessageInputContent;
use codex_protocol::models::ConfigurationReasoning;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::InternalChatMessageMetadataPassthrough;
use codex_protocol::models::LocalShellAction;
use codex_protocol::models::LocalShellStatus;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ReasoningItemReasoningSummary;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::WebSearchAction;
use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
#[serde(remote = "ResponseItem")]
pub(crate) enum TrustedResponseItem {
    AdditionalTools {
        id: Option<ResponseItemId>,
        role: String,
        tools: Vec<serde_json::Value>,
    },
    Message {
        id: Option<ResponseItemId>,
        role: String,
        content: Vec<ContentItem>,
        phase: Option<MessagePhase>,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    AgentMessage {
        id: Option<ResponseItemId>,
        author: String,
        recipient: String,
        content: Vec<AgentMessageInputContent>,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    Reasoning {
        id: Option<ResponseItemId>,
        summary: Vec<ReasoningItemReasoningSummary>,
        content: Option<Vec<ReasoningItemContent>>,
        encrypted_content: Option<String>,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    LocalShellCall {
        id: Option<ResponseItemId>,
        call_id: Option<String>,
        status: LocalShellStatus,
        action: LocalShellAction,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    FunctionCall {
        id: Option<ResponseItemId>,
        name: String,
        namespace: Option<String>,
        arguments: String,
        encrypted_function_args: Option<Vec<String>>,
        call_id: String,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    ToolSearchCall {
        id: Option<ResponseItemId>,
        call_id: Option<String>,
        status: Option<String>,
        execution: String,
        arguments: serde_json::Value,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    FunctionCallOutput {
        id: Option<ResponseItemId>,
        call_id: Option<String>,
        name: Option<String>,
        namespace: Option<String>,
        #[serde(with = "FunctionCallOutputPayloadWire")]
        output: FunctionCallOutputPayload,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    CustomToolCall {
        id: Option<ResponseItemId>,
        status: Option<String>,

        call_id: String,
        name: String,
        namespace: Option<String>,
        input: String,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    CustomToolCallOutput {
        id: Option<ResponseItemId>,
        call_id: String,
        name: Option<String>,
        #[serde(with = "FunctionCallOutputPayloadWire")]
        output: FunctionCallOutputPayload,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    ToolSearchOutput {
        id: Option<ResponseItemId>,
        call_id: Option<String>,
        status: String,
        execution: String,
        tools: Vec<serde_json::Value>,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    WebSearchCall {
        id: Option<ResponseItemId>,
        status: Option<String>,
        action: Option<WebSearchAction>,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    ImageGenerationCall {
        id: Option<ResponseItemId>,
        status: String,
        revised_prompt: Option<String>,
        result: String,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    Compaction {
        id: Option<ResponseItemId>,
        encrypted_content: String,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    ConfigurationUpdate {
        #[serde(with = "ConfigurationReasoningWire")]
        reasoning: ConfigurationReasoning,
    },
    CompactionTrigger {},
    ContextCompaction {
        id: Option<ResponseItemId>,
        encrypted_content: Option<String>,
        #[serde(with = "super::metadata::metadata::option")]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    Other,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "FunctionCallOutputPayload", deny_unknown_fields)]
pub(crate) struct FunctionCallOutputPayloadWire {
    body: FunctionCallOutputBody,
    success: Option<bool>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "ConfigurationReasoning", deny_unknown_fields)]
struct ConfigurationReasoningWire {
    #[serde(with = "super::events_scalars::ReasoningEffortWire")]
    effort: ReasoningEffort,
}

remote_adapter!(
    response_item,
    ResponseItem,
    TrustedResponseItem,
    "TrustedResponseItem"
);
