//! Replay aggregate serde must keep using the shared trusted component adapters.

use crate::RolloutReconstruction;
use codex_history::CodexHarnessMetadata;
use codex_history::GuardianHistoryCheckpoint;
use codex_history::ResponseItemEnvelope;
use codex_history::RetainedContext;
use codex_protocol::models::ContentItemKind;
use codex_protocol::models::ExecutedToolCall;
use codex_protocol::models::InternalChatMessageMetadataPassthrough;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ReasoningItemReasoningSummary;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::ToolResultMetadata;
use codex_protocol::models::ToolResultSource;
use codex_protocol::models::ToolResultSources;
use pretty_assertions::assert_eq;

fn metadata() -> InternalChatMessageMetadataPassthrough {
    let mut call = ExecutedToolCall::new("tool".to_owned(), serde_json::json!({ "x": 3 }));
    call.set_tool_result_metadata(ToolResultMetadata::new(&serde_json::json!({ "source": "native" })));
    call.set_tool_result_sources(ToolResultSources::new(vec![ToolResultSource {
        r#type: "resource".to_owned(),
        id: "original".to_owned(),
    }]));
    InternalChatMessageMetadataPassthrough {
        turn_id: Some("turn".to_owned()),
        create_time: Some("1750000000.125".parse().unwrap()),
        content_item_kinds: Some(vec![ContentItemKind("user.text".to_owned())]),
        cell_id: Some("cell".to_owned()),
        executed_tool_calls: Some(vec![call]),
        tool_calls_complete: Some(true),
    }
}

fn reasoning(content: Option<Vec<ReasoningItemContent>>) -> ResponseItem {
    ResponseItem::Reasoning {
        id: Some(codex_protocol::ResponseItemId::with_suffix("rs", "reasoning")),
        summary: vec![ReasoningItemReasoningSummary::SummaryText { text: "summary".to_owned() }],
        content,
        encrypted_content: Some("opaque".to_owned()),
        internal_chat_message_metadata_passthrough: Some(metadata()),
    }
}

fn history() -> Vec<ResponseItemEnvelope> {
    vec![
        ResponseItemEnvelope {
            item: reasoning(Some(vec![ReasoningItemContent::ReasoningText {
                text: "native raw reasoning".to_owned(),
            }])),
            metadata: Some(CodexHarnessMetadata {
                client_authored: true,
                history_truncation_token_limit: Some(99),
                user_input_order: Some(7),
                ..Default::default()
            }),
        },
        ResponseItemEnvelope { item: reasoning(None), metadata: None },
        ResponseItemEnvelope { item: reasoning(Some(Vec::new())), metadata: Some(CodexHarnessMetadata::default()) },
    ]
}

#[test]
fn output_projection_keeps_full_envelopes_and_guardian_checkpoint() {
    let output = RolloutReconstruction {
        history: history(),
        retained_context: RetainedContext::default(),
        guardian_history: Some(GuardianHistoryCheckpoint(history())),
        last_started_turn_id: None,
        previous_turn_settings: None,
        reference_context_item: None,
        world_state_baseline: None,
        window_number: 0,
        first_window_id: None,
        previous_window_id: None,
        window_id: None,
    };
    let restored: RolloutReconstruction = serde_json::from_slice(&serde_json::to_vec(&output).unwrap()).unwrap();
    assert_eq!(restored, output);
}

#[test]
fn reference_context_preserves_custom_effort_after_output_transport() {
    let mut context: codex_protocol::protocol::TurnContextItem = serde_json::from_value(serde_json::json!({
        "cwd": std::env::current_dir().unwrap(),
        "approval_policy": "never",
        "sandbox_policy": { "type": "danger-full-access" },
        "model": "custom-model",
        "summary": "auto",
    })).unwrap();
    context.effort = Some(codex_protocol::openai_models::ReasoningEffort::Custom("high".to_owned()));
    let output = RolloutReconstruction {
        history: Vec::new(),
        retained_context: RetainedContext::default(),
        guardian_history: None,
        last_started_turn_id: None,
        previous_turn_settings: None,
        reference_context_item: Some(context),
        world_state_baseline: None,
        window_number: 0,
        first_window_id: None,
        previous_window_id: None,
        window_id: None,
    };
    let restored: RolloutReconstruction = serde_json::from_slice(&serde_json::to_vec(&output).unwrap()).unwrap();
    assert_eq!(restored, output);
}

