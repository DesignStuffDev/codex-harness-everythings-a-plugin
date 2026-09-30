//! Process adapter for the selected `model_transport:default` component.
//!
//! The host owns prompt construction, turn state, approvals and event bookkeeping.
//! A component owns one complete inference stream and its backend credentials; host
//! credentials are never included in this request. Each invocation receives full
//! history, so a component may cache by thread ID but cannot require process reuse.
//! Model discovery, compaction policy, realtime audio and unary memory summaries
//! retain their native owners. Compaction inference using `stream` uses this adapter.

use codex_api::ApiError;
use codex_api::ResponseEvent;
use codex_api::ResponsesApiRequest;
use codex_component_host::ComponentBinding;
use codex_component_host::StreamFrame;
use codex_extension_api::ModelResponseStream;
use codex_model_transport_api::DecodedEvent;
use codex_model_transport_api::MODEL_TRANSPORT_CONTRACT_VERSION;
use codex_model_transport_api::ModelEvent;
use codex_protocol::ThreadId;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Version-one inference parameters, using the native normalized request shape.
#[derive(Serialize)]
struct ModelRequest {
    thread_id: ThreadId,
    request: ResponsesApiRequest,
}

pub(crate) async fn stream(
    binding: &ComponentBinding,
    thread_id: ThreadId,
    request: ResponsesApiRequest,
) -> Result<ModelResponseStream, ApiError> {
    let version = binding.spec.contract_version;
    let params = match version {
        1 => serde_json::to_value(ModelRequest { thread_id, request }).map_err(invalid)?,
        MODEL_TRANSPORT_CONTRACT_VERSION => codex_model_transport_api::encode_request(thread_id, request)
            .and_then(codex_model_transport_api::ModelRequest::into_value)
            .map_err(invalid)?,
        _ => return Err(invalid("unsupported model contract version")),
    };
    let stream = binding
        .stream("model.stream", params)
        .await
        .map_err(invalid)?;
    Ok(Box::pin(futures::stream::unfold(
        Some(stream),
        move |state| async move {
            let mut stream = state?;
            let event = match stream.next().await {
                Ok(StreamFrame::Event(value)) if version == MODEL_TRANSPORT_CONTRACT_VERSION => {
                    ModelEvent::from_value(value)
                        .and_then(codex_model_transport_api::decode_event)
                        .map_err(invalid)
                }
                Ok(StreamFrame::Event(value)) => decode(value).map(DecodedEvent::Response),
                Ok(StreamFrame::Done(_)) => Err(invalid("stream ended without completed event")),
                Err(error) => Err(invalid(error)),
            };
            let outcome = match event {
                Ok(DecodedEvent::Response(event @ ResponseEvent::Completed { .. })) => Ok(event),
                // Conversion captures the retry deadline before terminal/exit
                // waits. Preserve that exact error instead of restarting advice.
                Ok(DecodedEvent::BackendError(error)) => Err(error),
                Ok(DecodedEvent::Response(event)) => return Some((Ok(event), Some(stream))),
                Err(error) => return Some((Err(error), None)),
            };
            // Done is observable only after the host validates result/shutdown,
            // child exit/reap and its owned codec work. A malformed terminal
            // sequence overrides a backend classification; it cannot trigger a
            // retry as if the failed invocation were a clean backend response.
            let outcome = match stream.next().await {
                Ok(StreamFrame::Done(value)) if version == MODEL_TRANSPORT_CONTRACT_VERSION
                    && !value.as_object().is_some_and(serde_json::Map::is_empty) => {
                    Err(invalid("model contract 2 requires an empty terminal result"))
                }
                Ok(StreamFrame::Done(_)) => outcome,
                Ok(StreamFrame::Event(_)) => Err(invalid("event received after terminal model event")),
                Err(error) => Err(invalid(error)),
            };
            Some((outcome, None))
        },
    )))
}

fn invalid(error: impl std::fmt::Display) -> ApiError {
    // Component failures are terminal for this request. Never silently switch to
    // the native transport, which could send private input to a different backend.
    ApiError::InvalidRequest {
        message: format!("model transport component: {error:#}"),
    }
}

fn field<T: DeserializeOwned>(value: &Value, name: &str) -> Result<T, ApiError> {
    serde_json::from_value(value.get(name).cloned().unwrap_or(Value::Null)).map_err(invalid)
}

/// Decode every native response-event variant without changing the upstream API.
/// Unknown events fail explicitly, avoiding silent loss of new protocol behavior.
fn decode(value: Value) -> Result<ResponseEvent, ApiError> {
    let mut event = match value.get("type").and_then(Value::as_str) {
        Some("created") => Ok(ResponseEvent::Created {
            response_id: field(&value, "response_id")?,
        }),
        Some("output_item_added") => Ok(ResponseEvent::OutputItemAdded(field(&value, "item")?)),
        Some("output_item_done") => Ok(ResponseEvent::OutputItemDone(field(&value, "item")?)),
        Some("output_text_delta") => Ok(ResponseEvent::OutputTextDelta(field(&value, "delta")?)),
        Some("tool_call_input_delta") => Ok(ResponseEvent::ToolCallInputDelta {
            item_id: field(&value, "item_id")?,
            call_id: field(&value, "call_id")?,
            delta: field(&value, "delta")?,
        }),
        Some("completed") => Ok(ResponseEvent::Completed {
            response_id: field(&value, "response_id")?,
            token_usage: field(&value, "token_usage")?,
            usage_metadata: field(&value, "usage_metadata")?,
            end_turn: field(&value, "end_turn")?,
        }),
        Some("reasoning_summary_delta") => Ok(ResponseEvent::ReasoningSummaryDelta {
            delta: field(&value, "delta")?,
            summary_index: field(&value, "summary_index")?,
        }),
        Some("reasoning_summary_done") => Ok(ResponseEvent::ReasoningSummaryDone {
            item_id: field(&value, "item_id")?,
            text: field(&value, "text")?,
            summary_index: field(&value, "summary_index")?,
        }),
        Some("reasoning_content_delta") => Ok(ResponseEvent::ReasoningContentDelta {
            delta: field(&value, "delta")?,
            content_index: field(&value, "content_index")?,
        }),
        Some("reasoning_summary_part_added") => Ok(ResponseEvent::ReasoningSummaryPartAdded {
            summary_index: field(&value, "summary_index")?,
        }),
        Some("server_model") => Ok(ResponseEvent::ServerModel(field(&value, "model")?)),
        Some("model_verifications") => Ok(ResponseEvent::ModelVerifications(field(
            &value,
            "verifications",
        )?)),
        Some("turn_moderation_metadata") => Ok(ResponseEvent::TurnModerationMetadata(field(
            &value, "metadata",
        )?)),
        Some("server_reasoning_included") => Ok(ResponseEvent::ServerReasoningIncluded(field(
            &value, "included",
        )?)),
        Some("rate_limits") => Ok(ResponseEvent::RateLimits(field(&value, "rate_limits")?)),
        Some("models_etag") => Ok(ResponseEvent::ModelsEtag(field(&value, "etag")?)),
        Some("safety_buffering") => Ok(ResponseEvent::SafetyBuffering(field(&value, "buffering")?)),
        other => Err(invalid(format!("unsupported event type: {other:?}"))),
    }?;
    if let ResponseEvent::SafetyBuffering(buffering) = &mut event {
        // Native deserialization skips this runtime flag. The component event
        // contract carries the already-resolved presentation decision explicitly.
        buffering.show_buffering_ui =
            field::<Option<bool>>(&value["buffering"], "show_buffering_ui")?.unwrap_or(false);
    }
    Ok(event)
}

#[cfg(test)]
#[path = "component_model_tests.rs"]
mod tests;
