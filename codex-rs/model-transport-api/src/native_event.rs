use crate::Buffering;
use crate::ModelEvent;
use crate::WireError;
use codex_api::ApiError;
use codex_api::ResponseEvent;
use codex_protocol::models::ResponseItem;
use serde_json::Value;

/// An event ready for native policy. A terminal outcome still requires process cleanup.
pub enum DecodedEvent {
    Response(ResponseEvent),
    BackendError(ApiError),
}

impl std::fmt::Debug for DecodedEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("DecodedEvent").finish_non_exhaustive()
    }
}

/// Encode every native event without relying on asymmetric provider-request serde.
pub fn encode_event(event: ResponseEvent) -> Result<ModelEvent, WireError> {
    Ok(match event {
        ResponseEvent::Created { response_id } => ModelEvent::Created { response_id },
        ResponseEvent::OutputItemAdded(item) => ModelEvent::OutputItemAdded { item: encode_item(item)? },
        ResponseEvent::OutputItemDone(item) => ModelEvent::OutputItemDone { item: encode_item(item)? },
        ResponseEvent::OutputTextDelta(delta) => ModelEvent::OutputTextDelta { delta },
        ResponseEvent::ToolCallInputDelta { item_id, call_id, delta } => ModelEvent::ToolCallInputDelta { item_id, call_id, delta },
        ResponseEvent::Completed { response_id, token_usage, usage_metadata, end_turn } => ModelEvent::Completed {
            response_id, token_usage: token_usage.map(Into::into), usage_metadata, end_turn,
        },
        ResponseEvent::ReasoningSummaryDelta { delta, summary_index } => ModelEvent::ReasoningSummaryDelta { delta, summary_index },
        ResponseEvent::ReasoningSummaryDone { item_id, text, summary_index } => ModelEvent::ReasoningSummaryDone { item_id, text, summary_index },
        ResponseEvent::ReasoningContentDelta { delta, content_index } => ModelEvent::ReasoningContentDelta { delta, content_index },
        ResponseEvent::ReasoningSummaryPartAdded { summary_index } => ModelEvent::ReasoningSummaryPartAdded { summary_index },
        ResponseEvent::ServerModel(model) => ModelEvent::ServerModel { model },
        ResponseEvent::ModelVerifications(verifications) => ModelEvent::ModelVerifications { verifications },
        ResponseEvent::TurnModerationMetadata(metadata) => ModelEvent::TurnModerationMetadata { metadata },
        ResponseEvent::ServerReasoningIncluded(included) => ModelEvent::ServerReasoningIncluded { included },
        ResponseEvent::RateLimits(rate_limits) => ModelEvent::RateLimits { rate_limits },
        ResponseEvent::ModelsEtag(etag) => ModelEvent::ModelsEtag { etag },
        ResponseEvent::SafetyBuffering(buffering) => ModelEvent::SafetyBuffering { buffering: Buffering {
            use_cases: buffering.use_cases, reasons: buffering.reasons,
            retry_model: buffering.faster_model, show_buffering_ui: buffering.show_buffering_ui,
        } },
    })
}

/// Decode using provider trust, never the trusted persistence/state codec.
pub fn decode_event(event: ModelEvent) -> Result<DecodedEvent, WireError> {
    let event = match event {
        ModelEvent::BackendError { error } => return Ok(DecodedEvent::BackendError(crate::native_error::decode_backend_error(error)?)),
        ModelEvent::Created { response_id } => ResponseEvent::Created { response_id },
        ModelEvent::OutputItemAdded { item } => ResponseEvent::OutputItemAdded(serde_json::from_value(item).map_err(|_| WireError::Item)?),
        ModelEvent::OutputItemDone { item } => ResponseEvent::OutputItemDone(serde_json::from_value(item).map_err(|_| WireError::Item)?),
        ModelEvent::OutputTextDelta { delta } => ResponseEvent::OutputTextDelta(delta),
        ModelEvent::ToolCallInputDelta { item_id, call_id, delta } => ResponseEvent::ToolCallInputDelta { item_id, call_id, delta },
        ModelEvent::Completed { response_id, token_usage, usage_metadata, end_turn } => ResponseEvent::Completed {
            response_id, token_usage: token_usage.map(Into::into), usage_metadata, end_turn,
        },
        ModelEvent::ReasoningSummaryDelta { delta, summary_index } => ResponseEvent::ReasoningSummaryDelta { delta, summary_index },
        ModelEvent::ReasoningSummaryDone { item_id, text, summary_index } => ResponseEvent::ReasoningSummaryDone { item_id, text, summary_index },
        ModelEvent::ReasoningContentDelta { delta, content_index } => ResponseEvent::ReasoningContentDelta { delta, content_index },
        ModelEvent::ReasoningSummaryPartAdded { summary_index } => ResponseEvent::ReasoningSummaryPartAdded { summary_index },
        ModelEvent::ServerModel { model } => ResponseEvent::ServerModel(model),
        ModelEvent::ModelVerifications { verifications } => ResponseEvent::ModelVerifications(verifications),
        ModelEvent::TurnModerationMetadata { metadata } => ResponseEvent::TurnModerationMetadata(metadata),
        ModelEvent::ServerReasoningIncluded { included } => ResponseEvent::ServerReasoningIncluded(included),
        ModelEvent::RateLimits { rate_limits } => ResponseEvent::RateLimits(rate_limits),
        ModelEvent::ModelsEtag { etag } => ResponseEvent::ModelsEtag(etag),
        ModelEvent::SafetyBuffering { buffering } => {
            // SafetyBuffering's upstream type is not re-exported; infer its type
            // from the variant and explicitly restore the skipped runtime field.
            let show_buffering_ui = buffering.show_buffering_ui;
            let mut native = ResponseEvent::SafetyBuffering(
                serde_json::from_value(serde_json::to_value(buffering).map_err(|_| WireError::Event)?)
                    .map_err(|_| WireError::Event)?
            );
            if let ResponseEvent::SafetyBuffering(buffering) = &mut native {
                buffering.show_buffering_ui = show_buffering_ui;
            }
            native
        }
    };
    Ok(DecodedEvent::Response(event))
}

fn encode_item(item: ResponseItem) -> Result<Value, WireError> {
    let mut encoded = serde_json::to_value(&item).map_err(|_| WireError::Item)?;
    match item {
        ResponseItem::Reasoning { content, .. } => {
            // Request encoding intentionally omits Text-only/empty reasoning.
            // Output events must preserve them for native event/timing consumers.
            encoded["content"] = serde_json::to_value(content).map_err(|_| WireError::Item)?;
        }
        ResponseItem::AdditionalTools { .. }
        | ResponseItem::Message { .. }
        | ResponseItem::AgentMessage { .. }
        | ResponseItem::LocalShellCall { .. }
        | ResponseItem::FunctionCall { .. }
        | ResponseItem::ToolSearchCall { .. }
        | ResponseItem::FunctionCallOutput { .. }
        | ResponseItem::CustomToolCall { .. }
        | ResponseItem::CustomToolCallOutput { .. }
        | ResponseItem::ToolSearchOutput { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::ConfigurationUpdate { .. }
        | ResponseItem::CompactionTrigger { .. }
        | ResponseItem::ContextCompaction { .. }
        | ResponseItem::Other => {}
    }
    Ok(encoded)
}
