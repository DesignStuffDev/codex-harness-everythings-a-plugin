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
    let params = serde_json::to_value(ModelRequest { thread_id, request }).map_err(invalid)?;
    let stream = binding
        .stream("model.stream", params)
        .await
        .map_err(invalid)?;
    Ok(Box::pin(futures::stream::unfold(
        Some(stream),
        |state| async move {
            let mut stream = state?;
            let event = match stream.next().await {
                Ok(StreamFrame::Event(value)) => decode(value),
                Ok(StreamFrame::Done(_)) => Err(invalid("stream ended without completed event")),
                Err(error) => Err(invalid(error)),
            };
            match event {
                Ok(event @ ResponseEvent::Completed { .. }) => {
                    // Validate the protocol terminal frame before committing completion.
                    // Returning no state also drops and reaps the invocation process.
                    let event = match stream.next().await {
                        Ok(StreamFrame::Done(_)) => Ok(event),
                        Ok(StreamFrame::Event(_)) => {
                            Err(invalid("event received after completed event"))
                        }
                        Err(error) => Err(invalid(error)),
                    };
                    Some((event, None))
                }
                Ok(event) => Some((Ok(event), Some(stream))),
                Err(error) => Some((Err(error), None)),
            }
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
