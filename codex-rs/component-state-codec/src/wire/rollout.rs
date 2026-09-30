use codex_history::CompactedItem;
use codex_history::CompactionResumeMetadata;
use codex_history::GuardianHistoryCheckpoint;
use codex_history::ResponseItemEnvelope;
use codex_history::RetainedContext;
use codex_history::RetainedContextEvent;
use codex_history::RolloutItem;
use codex_protocol::mcp::McpResourceOriginCheckpoint;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::InterAgentCommunication;
use codex_protocol::protocol::SessionMetaLine;
use codex_protocol::protocol::TokenUsageRecord;
use codex_protocol::protocol::TurnContextItem;
use codex_protocol::protocol::WorldStateItem;
use codex_protocol::realtime::RealtimeItem;
use codex_protocol::security_risk::SecurityRiskScore;
use serde::Deserialize;
use serde::Serialize;

/// External tags avoid the persisted compatibility encoder and buffered JSON numbers.
#[derive(Serialize, Deserialize)]
#[serde(remote = "RolloutItem")]
pub(crate) enum RolloutWire {
    SessionMeta(#[serde(with = "super::session::SessionMetaLineWire")] SessionMetaLine),
    ResponseItem(#[serde(with = "super::envelope::EnvelopeWire")] ResponseItemEnvelope),
    InterAgentCommunication(#[serde(with = "InterAgentCommunicationWire")] InterAgentCommunication),
    InterAgentCommunicationMetadata { trigger_turn: bool },
    Compacted(#[serde(with = "CompactedWire")] CompactedItem),
    TurnContext(#[serde(with = "super::session::TurnContextItemWire")] TurnContextItem),
    TokenUsageRecord(#[serde(with = "super::usage::TokenUsageRecordWire")] TokenUsageRecord),
    WorldState(WorldStateItem),
    SecurityRiskScore(#[serde(with = "super::security::SecurityRiskScoreWire")] SecurityRiskScore),
    // Preserve the actual retained event, including assistant phase and provenance.
    // RolloutItem's ordinary compatibility encoder intentionally rewrites this case.
    RetainedContext(RetainedContextEvent),
    EventMsg(#[serde(with = "super::events::EventMsgWire")] EventMsg),
    RealtimeItem(RealtimeItem),
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "CompactedItem", deny_unknown_fields)]
pub(crate) struct CompactedWire {
    message: String,
    #[serde(with = "super::envelope::envelopes::option_vec")]
    replacement_history: Option<Vec<ResponseItemEnvelope>>,
    #[serde(with = "super::envelope::guardian_checkpoint::option")]
    guardian_history: Option<GuardianHistoryCheckpoint>,
    retained_context: Option<RetainedContext>,
    mcp_resource_origins: Option<McpResourceOriginCheckpoint>,
    window_number: Option<u64>,
    first_window_id: Option<String>,
    previous_window_id: Option<String>,
    window_id: Option<String>,
    compaction_response_id: Option<String>,
    #[serde(with = "super::usage::token_usage_record::option")]
    latest_token_usage_record: Option<TokenUsageRecord>,
    resume_metadata: Option<CompactionResumeMetadata>,
}

remote_adapter!(rollout_item, RolloutItem, RolloutWire, "RolloutWire");

#[derive(Serialize, Deserialize)]
#[serde(remote = "InterAgentCommunication", deny_unknown_fields)]
struct InterAgentCommunicationWire {
    id: Option<codex_protocol::ResponseItemId>,
    author: codex_protocol::AgentPath,
    recipient: codex_protocol::AgentPath,
    other_recipients: Vec<codex_protocol::AgentPath>,
    content: String,
    encrypted_content: Option<String>,
    #[serde(with = "super::metadata::metadata::option")]
    internal_chat_message_metadata_passthrough:
        Option<codex_protocol::models::InternalChatMessageMetadataPassthrough>,
    trigger_turn: bool,
}
