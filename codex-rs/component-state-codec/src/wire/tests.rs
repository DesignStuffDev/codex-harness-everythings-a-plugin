use super::envelope;
use super::response;
use codex_history::CodexHarnessMetadata;
use codex_history::CompactedItem;
use codex_history::GuardianHistoryCheckpoint;
use codex_history::ResponseItemEnvelope;
use codex_history::RetainedContext;
use codex_history::RetainedContextEvent;
use codex_history::RetainedUserMessage;
use codex_history::RolloutItem;
use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::models::ContentItemKind;
use codex_protocol::models::ExecutedToolCall;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::InternalChatMessageMetadataPassthrough;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ReasoningItemReasoningSummary;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::ToolResultMetadata;
use codex_protocol::models::ToolResultSource;
use codex_protocol::models::ToolResultSources;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TokenUsageRecord;
use pretty_assertions::assert_eq;

fn metadata() -> InternalChatMessageMetadataPassthrough {
    let mut call = ExecutedToolCall::new("tool".to_owned(), serde_json::json!({ "x": 3 }));
    call.set_tool_result_metadata(ToolResultMetadata::new(
        &serde_json::json!({ "source": "native" }),
    ));
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
        id: Some(codex_protocol::ResponseItemId::with_suffix(
            "rs",
            "reasoning",
        )),
        summary: vec![ReasoningItemReasoningSummary::SummaryText {
            text: "summary".to_owned(),
        }],
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
        ResponseItemEnvelope {
            item: reasoning(None),
            metadata: None,
        },
        ResponseItemEnvelope {
            item: reasoning(Some(Vec::new())),
            metadata: Some(CodexHarnessMetadata::default()),
        },
    ]
}

fn usage_record() -> TokenUsageRecord {
    let usage = TokenUsage {
        input_tokens: 3,
        cached_input_tokens: 2,
        cache_write_input_tokens: 1,
        output_tokens: 4,
        reasoning_output_tokens: 2,
        total_tokens: 7,
        codex_rollout_budget_units: Some("12.125".parse().unwrap()),
    };
    TokenUsageRecord {
        thread_id: ThreadId::new(),
        turn_id: "turn".to_owned(),
        session_id: SessionId::new(),
        root_turn_id: "root-turn".to_owned(),
        response_id: "response".to_owned(),
        usage: usage.clone(),
        turn_token_usage: usage.clone(),
        thread_token_usage: usage,
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RolloutRecords {
    #[serde(with = "crate::rollout_item::vec")]
    items: Vec<RolloutItem>,
}

fn rollout_records(items: Vec<RolloutItem>) -> RolloutRecords {
    RolloutRecords { items }
}

#[test]
fn reasoning_variants_and_host_only_metadata_survive_trusted_response_roundtrip() {
    for content in [
        None,
        Some(Vec::new()),
        Some(vec![ReasoningItemContent::ReasoningText {
            text: "raw".to_owned(),
        }]),
        Some(vec![ReasoningItemContent::Text {
            text: "legacy".to_owned(),
        }]),
    ] {
        let item = reasoning(content);
        let bytes = serde_json::to_vec(&response::response_item::Borrowed(&item)).unwrap();
        let decoded: response::response_item::Owned = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.0, item);
    }
}

#[test]
fn trusted_payload_preserves_success_and_provider_deserialization_still_omits_it() {
    for success in [None, Some(false), Some(true)] {
        let payload = FunctionCallOutputPayload {
            body: FunctionCallOutputBody::Text("output".to_owned()),
            success,
        };
        let item = ResponseItem::FunctionCallOutput {
            id: Some(codex_protocol::ResponseItemId::with_suffix("fc", "output")),
            call_id: Some("call".to_owned()),
            name: None,
            namespace: None,
            output: payload.clone(),
            internal_chat_message_metadata_passthrough: Some(metadata()),
        };
        let bytes = serde_json::to_vec(&response::response_item::Borrowed(&item)).unwrap();
        let decoded: response::response_item::Owned = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.0, item);
        let provider: ResponseItem =
            serde_json::from_slice(&serde_json::to_vec(&item).unwrap()).unwrap();
        let ResponseItem::FunctionCallOutput {
            output,
            internal_chat_message_metadata_passthrough,
            ..
        } = provider
        else {
            panic!("expected tool output");
        };
        assert_eq!(
            output,
            FunctionCallOutputPayload {
                body: payload.body,
                success: None
            }
        );
        let mut expected_metadata = metadata();
        expected_metadata.cell_id = None;
        expected_metadata.executed_tool_calls = None;
        expected_metadata.tool_calls_complete = None;
        assert_eq!(
            internal_chat_message_metadata_passthrough,
            Some(expected_metadata)
        );
    }
}

#[test]
fn nested_compaction_keeps_review_history_usage_and_per_item_metadata_presence() {
    let compacted = CompactedItem {
        message: "summary".to_owned(),
        replacement_history: Some(history()),
        guardian_history: Some(GuardianHistoryCheckpoint(history())),
        retained_context: Some(RetainedContext::default()),
        mcp_resource_origins: None,
        window_number: Some(4),
        first_window_id: Some("first".to_owned()),
        previous_window_id: None,
        window_id: Some("current".to_owned()),
        compaction_response_id: Some("compact-response".to_owned()),
        latest_token_usage_record: Some(usage_record()),
        resume_metadata: None,
    };
    let input = rollout_records(vec![RolloutItem::Compacted(compacted.clone())]);
    let restored: RolloutRecords =
        serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
    let [RolloutItem::Compacted(actual)] = restored.items.as_slice() else {
        panic!("expected compacted input");
    };
    assert_eq!(actual, &compacted);
}

#[test]
fn direct_usage_and_retained_delivery_keep_native_fields_without_rollout_reencoding() {
    let usage = usage_record();
    let delivery = RetainedContextEvent::DeliveredAssistantMessage {
        message: RetainedUserMessage {
            turn_id: "turn".to_owned(),
            message_id: Some("assistant".to_owned()),
            text: "exact incomplete text".to_owned(),
            complete: false,
            origin: codex_history::UserInputOrigin::User,
            phase: Some(MessagePhase::Commentary),
        },
        acceptance_order: 31,
    };
    let input = rollout_records(vec![
        RolloutItem::TokenUsageRecord(usage.clone()),
        RolloutItem::RetainedContext(delivery.clone()),
    ]);
    let restored: RolloutRecords =
        serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
    let [
        RolloutItem::TokenUsageRecord(actual_usage),
        RolloutItem::RetainedContext(actual_delivery),
    ] = restored.items.as_slice()
    else {
        panic!("expected native usage and retained event variants");
    };
    assert_eq!(actual_usage, &usage);
    assert_eq!(actual_delivery, &delivery);
}

#[test]
fn absent_metadata_stays_absent_instead_of_becoming_an_empty_annotation() {
    for metadata in [None, Some(CodexHarnessMetadata::default())] {
        let item = ResponseItemEnvelope {
            item: ResponseItem::Other,
            metadata,
        };
        let bytes = serde_json::to_vec(&envelope::envelopes::Borrowed(&item)).unwrap();
        let decoded: envelope::envelopes::Owned = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.0, item);
    }
}

#[test]
fn inter_agent_metadata_survives_the_same_trusted_codec() {
    let mut communication = codex_protocol::protocol::InterAgentCommunication::new(
        "/root".parse().unwrap(),
        "/root/worker".parse().unwrap(),
        Vec::new(),
        "continue".to_owned(),
        /*trigger_turn*/ true,
    );
    communication.internal_chat_message_metadata_passthrough = Some(metadata());
    let input = rollout_records(vec![RolloutItem::InterAgentCommunication(
        communication.clone(),
    )]);
    let restored: RolloutRecords =
        serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
    let [RolloutItem::InterAgentCommunication(actual)] = restored.items.as_slice() else {
        panic!("expected communication input");
    };
    assert_eq!(actual, &communication);
}

#[test]
fn session_metadata_does_not_collapse_custom_source_or_dynamic_tool_schema() {
    use codex_protocol::dynamic_tools::DynamicToolFunctionSpec;
    use codex_protocol::dynamic_tools::DynamicToolSpec;
    use codex_protocol::protocol::SessionMeta;
    use codex_protocol::protocol::SessionMetaLine;
    use codex_protocol::protocol::ThreadSource;
    let dynamic_tools = Some(vec![DynamicToolSpec::Function(DynamicToolFunctionSpec {
        name: "external".to_owned(),
        description: "schema".to_owned(),
        input_schema: serde_json::from_str(
            r#"{"type":"number","multipleOf":0.125,"default":null}"#,
        )
        .unwrap(),
        defer_loading: true,
    })]);
    let input = rollout_records(vec![RolloutItem::SessionMeta(SessionMetaLine {
        meta: SessionMeta {
            thread_source: Some(ThreadSource::Feature("user".to_owned())),
            dynamic_tools: dynamic_tools.clone(),
            ..Default::default()
        },
        git: None,
    })]);
    let restored: RolloutRecords =
        serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
    let [RolloutItem::SessionMeta(actual)] = restored.items.as_slice() else {
        panic!("expected session metadata");
    };
    assert_eq!(
        actual.meta.thread_source,
        Some(ThreadSource::Feature("user".to_owned()))
    );
    assert_eq!(actual.meta.dynamic_tools, dynamic_tools);
}

#[test]
fn security_scores_preserve_float_bits_and_a_captured_json_null() {
    let score = codex_protocol::security_risk::SecurityRiskScore {
        scores: std::collections::BTreeMap::from([
            ("negative-zero".to_owned(), -0.0),
            ("nan".to_owned(), f64::from_bits(0x7ff8_0000_0000_0042)),
        ]),
        call_id: None,
        action: Some(serde_json::Value::Null),
        sampled_at: None,
    };
    let input = rollout_records(vec![RolloutItem::SecurityRiskScore(score.clone())]);
    let restored: RolloutRecords =
        serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
    let [RolloutItem::SecurityRiskScore(actual)] = restored.items.as_slice() else {
        panic!("expected security score");
    };
    let bits = |scores: &std::collections::BTreeMap<String, f64>| {
        scores
            .iter()
            .map(|(name, score)| (name.clone(), score.to_bits()))
            .collect::<Vec<_>>()
    };
    assert_eq!(bits(&actual.scores), bits(&score.scores));
    assert_eq!(actual.action, Some(serde_json::Value::Null));
}
