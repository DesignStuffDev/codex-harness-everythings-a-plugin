use crate::BackendError;
use crate::Usage;
use crate::WireError;
use codex_protocol::ResponseUsageMetadata;
use codex_protocol::protocol::ModelVerification;
use codex_protocol::protocol::RateLimitSnapshot;
use codex_protocol::protocol::TurnModerationMetadataEvent;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// A resolved buffering decision, including its otherwise serde-skipped UI flag.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Buffering {
    pub use_cases: Vec<String>,
    pub reasons: Vec<String>,
    pub retry_model: Option<String>,
    pub show_buffering_ui: bool,
}

impl std::fmt::Debug for Buffering {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Buffering").finish_non_exhaustive()
    }
}

/// Model events are typed independently from their generic process envelopes.
/// Item JSON retains native provider decoding; it is never trusted storage state.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModelEvent {
    Created { response_id: Option<String> },
    OutputItemAdded { item: Value },
    OutputItemDone { item: Value },
    OutputTextDelta { delta: String },
    ToolCallInputDelta {
        item_id: String,
        call_id: Option<String>,
        delta: String,
    },
    Completed {
        response_id: String,
        token_usage: Option<Usage>,
        usage_metadata: Option<ResponseUsageMetadata>,
        end_turn: Option<bool>,
    },
    ReasoningSummaryDelta { delta: String, summary_index: i64 },
    ReasoningSummaryDone { item_id: String, text: String, summary_index: i64 },
    ReasoningContentDelta { delta: String, content_index: i64 },
    ReasoningSummaryPartAdded { summary_index: i64 },
    ServerModel { model: String },
    ModelVerifications { verifications: Vec<ModelVerification> },
    TurnModerationMetadata { metadata: TurnModerationMetadataEvent },
    ServerReasoningIncluded { included: bool },
    RateLimits { rate_limits: RateLimitSnapshot },
    ModelsEtag { etag: String },
    SafetyBuffering { buffering: Buffering },
    BackendError { error: BackendError },
}

impl ModelEvent {
    pub fn from_value(value: Value) -> Result<Self, WireError> {
        let event: Self = serde_json::from_value(value).map_err(|_| WireError::Event)?;
        event.validate()?;
        Ok(event)
    }

    pub fn into_value(self) -> Result<Value, WireError> {
        self.validate()?;
        serde_json::to_value(self).map_err(|_| WireError::Event)
    }

    /// Validate domain constraints without applying engine budget or retry policy.
    pub fn validate(&self) -> Result<(), WireError> {
        match self {
            Self::BackendError { error } => error.validate(),
            Self::OutputItemAdded { item } | Self::OutputItemDone { item } => {
                // The native conversion performs full provider deserialization
                // once. Do not clone/materialize a large image item merely to
                // repeat that decoding during outer-envelope validation.
                if item.is_object() && item.get("type").is_some_and(Value::is_string) {
                    Ok(())
                } else {
                    Err(WireError::Item)
                }
            }
            Self::Created { .. }
            | Self::OutputTextDelta { .. }
            | Self::ToolCallInputDelta { .. }
            | Self::Completed { .. }
            | Self::ReasoningSummaryDelta { .. }
            | Self::ReasoningSummaryDone { .. }
            | Self::ReasoningContentDelta { .. }
            | Self::ReasoningSummaryPartAdded { .. }
            | Self::ServerModel { .. }
            | Self::ModelVerifications { .. }
            | Self::TurnModerationMetadata { .. }
            | Self::ServerReasoningIncluded { .. }
            | Self::RateLimits { .. }
            | Self::ModelsEtag { .. }
            | Self::SafetyBuffering { .. } => Ok(()),
        }
    }
}

impl std::fmt::Debug for ModelEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Self::Created { .. } => "created",
            Self::OutputItemAdded { .. } => "output_item_added",
            Self::OutputItemDone { .. } => "output_item_done",
            Self::OutputTextDelta { .. } => "output_text_delta",
            Self::ToolCallInputDelta { .. } => "tool_call_input_delta",
            Self::Completed { .. } => "completed",
            Self::ReasoningSummaryDelta { .. } => "reasoning_summary_delta",
            Self::ReasoningSummaryDone { .. } => "reasoning_summary_done",
            Self::ReasoningContentDelta { .. } => "reasoning_content_delta",
            Self::ReasoningSummaryPartAdded { .. } => "reasoning_summary_part_added",
            Self::ServerModel { .. } => "server_model",
            Self::ModelVerifications { .. } => "model_verifications",
            Self::TurnModerationMetadata { .. } => "turn_moderation_metadata",
            Self::ServerReasoningIncluded { .. } => "server_reasoning_included",
            Self::RateLimits { .. } => "rate_limits",
            Self::ModelsEtag { .. } => "models_etag",
            Self::SafetyBuffering { .. } => "safety_buffering",
            Self::BackendError { .. } => "backend_error",
        };
        formatter.debug_struct("ModelEvent").field("type", &kind).finish_non_exhaustive()
    }
}
