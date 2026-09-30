//! Staged native differential tests. Link only after enabling context-engine/snapshot-api.
//! The current core implementation is the oracle until these and its regressions pass.

use crate::config::ManagedFeatures;
use crate::context_manager::ContextManager;
use codex_context_engine::ContextManager as ExtractedContextManager;
use codex_context_replay::ReplayHistory;
use codex_context_replay::ReviewPolicy;
use codex_features::Feature;
use codex_features::Features;
use codex_history::CodexHarnessMetadata;
use codex_history::ResponseItemEnvelope;
use codex_protocol::items::TurnItem;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ContentItemKind;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ImageReference;
use codex_protocol::models::InternalChatMessageMetadataPassthrough;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::InputModality;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::TokenUsage;
use codex_utils_output_truncation::TruncationPolicy;
use pretty_assertions::assert_eq;

fn message(id: &str, role: &str, text: &str, kind: &str) -> ResponseItemEnvelope {
    ResponseItemEnvelope::new(ResponseItem::Message {
        id: Some(codex_protocol::ResponseItemId::with_suffix("msg", id)),
        role: role.to_owned(),
        content: vec![ContentItem::InputText { text: text.to_owned() }],
        phase: None,
        internal_chat_message_metadata_passthrough: Some(InternalChatMessageMetadataPassthrough {
            turn_id: Some(format!("turn-{id}")),
            content_item_kinds: Some(vec![ContentItemKind(kind.to_owned())]),
            ..Default::default()
        }),
    })
}

fn assert_history_parity(native: &ContextManager, extracted: &ExtractedContextManager) {
    assert_eq!(extracted.annotated_items(), native.annotated_items());
    assert_eq!(extracted.retained_context(), native.retained_context());
    assert_eq!(extracted.guardian_history_checkpoint(), native.guardian_history_checkpoint());
    assert_eq!(extracted.reference_context_item(), native.reference_context_item());
    assert_eq!(extracted.world_state_checkpoint(), native.world_state_checkpoint());
    assert_eq!(extracted.token_info(), native.token_info());
    assert_eq!((extracted.history_version(), extracted.reset_version),
        (native.history_version(), native.reset_version));
    let native_snapshot = native.conversation_history_snapshot();
    let extracted_snapshot = extracted.conversation_history_snapshot();
    assert_eq!(extracted_snapshot.review_items().cloned().collect::<Vec<_>>(),
        native_snapshot.review_items().cloned().collect::<Vec<_>>());
    assert_eq!(
        (extracted_snapshot.review_history_version(), extracted_snapshot.user_message_revision(),
            extracted_snapshot.uses_parent_context_for_review(), extracted_snapshot.uses_independent_review_history()),
        (native_snapshot.review_history_version(), native_snapshot.user_message_revision(),
            native_snapshot.uses_parent_context_for_review(), native_snapshot.uses_independent_review_history()),
    );
    // Guardian evidence revisions are process-unique identities, not equal counters
    // across two simultaneous implementations. All evidence and policy above match.
    let base = BaseInstructions { text: "Preserve the original restrictions.".to_owned(), provenance: None };
    assert_eq!(extracted.estimate_token_count_with_base_instructions(&base),
        native.estimate_token_count_with_base_instructions(&base));
    for server_reasoning_included in [false, true] {
        assert_eq!(extracted.get_total_token_usage(server_reasoning_included),
            native.get_total_token_usage(server_reasoning_included));
    }
    assert_eq!(extracted.clone().for_prompt_annotated(&[InputModality::Text]),
        native.clone().for_prompt_annotated(&[InputModality::Text]));
}

#[test]
fn native_engine_preserves_retention_truncation_compaction_and_rollback() {
    for source in [SessionSource::Cli, SessionSource::SubAgent(SubAgentSource::Review)] {
        for independent_review in [false, true] {
            let mut features = Features::with_defaults();
            features.set_enabled(Feature::GuardianReuseParentCompaction, !independent_review);
            let features = ManagedFeatures::from(features);
            let mut native = ContextManager::for_session(&source, &features);
            let mut extracted = ExtractedContextManager::for_policy(ReviewPolicy {
                independent_review,
                retain_inherited_user_messages: !source.is_non_root_agent(),
            });
            let mut inherited = message("inherited", "user", "Never publish credentials.", "user.text");
            inherited.metadata = Some(CodexHarnessMetadata { inherited_user_message: true, ..Default::default() });
            let mut media = message("media", "user", "Read the image without changing files.", "user.text");
            if let ResponseItem::Message { content, .. } = &mut media.item {
                content.push(ContentItem::InputImage {
                    image: ImageReference::File { file_id: "fixture-image".to_owned() }, detail: None,
                });
            }
            let corpus = vec![
                message("system", "system", "discarded native system message", "unknown"),
                inherited,
                message("first", "user", &"Keep restrictions. ".repeat(400), "user.text"),
                message("hidden", "user", "<additional_context>runtime note</additional_context>", "generic.additional_context"),
                ResponseItemEnvelope::new(ResponseItem::FunctionCall {
                    id: Some(codex_protocol::ResponseItemId::with_suffix("fc", "tool-call")), call_id: "tool-call".to_owned(),
                    name: "fixture".to_owned(), namespace: None, arguments: "{}".to_owned(),
                    encrypted_function_args: None, internal_chat_message_metadata_passthrough: None,
                }),
                ResponseItemEnvelope {
                    item: ResponseItem::FunctionCallOutput {
                        id: Some(codex_protocol::ResponseItemId::with_suffix("fc", "tool-result")), call_id: Some("tool-call".to_owned()), name: None,
                        namespace: None, output: FunctionCallOutputPayload::from_text("bounded output ".repeat(200)),
                        internal_chat_message_metadata_passthrough: None,
                    },
                    metadata: Some(CodexHarnessMetadata { history_truncation_token_limit: Some(16), ..Default::default() }),
                },
                message("answer", "assistant", "The original restriction remains.", "unknown"),
                message("settings", "developer", "<model_switch>context</model_switch>", "model_switch.instructions"),
                media,
            ];
            for envelope in corpus {
                let mut native_items = vec![envelope.clone()];
                let mut extracted_items = vec![envelope];
                native.record_annotated_items(&mut native_items, TruncationPolicy::Tokens(64));
                extracted.record_annotated_items(&mut extracted_items, TruncationPolicy::Tokens(64));
                assert_eq!(extracted_items, native_items);
                assert_history_parity(&native, &extracted);
            }
            let usage = TokenUsage { total_tokens: 42, ..Default::default() };
            native.update_token_info(&usage, Some(2048));
            extracted.update_token_info(&usage, Some(2048));
            assert_history_parity(&native, &extracted);
            let retained = native.retained_context().clone();
            let user_messages = crate::compact::collect_annotated_user_messages(native.annotated_items());
            let replacement = crate::compact::build_compacted_history(Vec::new(), &user_messages, "Legacy summary.");
            native.replace_annotated(replacement);
            native.restore_retained_context(Some(&retained));
            ReplayHistory::apply_legacy_compaction(&mut extracted, "Legacy summary.");
            assert_history_parity(&native, &extracted);
            let retained = native.retained_context().clone();
            let checkpoint = native.guardian_history_checkpoint();
            native.restore_review_context(Some(&retained), checkpoint.as_ref(), /*reviewer_compaction_hash*/ None);
            extracted.restore_review_context(Some(&retained), checkpoint.as_ref(), /*reviewer_compaction_hash*/ None);
            assert_history_parity(&native, &extracted);
            for count in [0, 1, 100] {
                native.drop_last_n_user_turns(count);
                extracted.drop_last_n_user_turns(count);
                assert_history_parity(&native, &extracted);
            }
        }
    }
}

#[test]
fn native_engine_classifies_legacy_context_and_authorization_identically() {
    let text_cases = [
        "ordinary user restriction", "# AGENTS.md instructions\n<INSTRUCTIONS>x</INSTRUCTIONS>",
        " <ENVIRONMENT_CONTEXT>x</ENVIRONMENT_CONTEXT> ", "<additional_context>x</additional_context>",
        "<codex_internal_context source=\"user_goal\">x</codex_internal_context>",
        "<codex_internal_context source=\"Invalid\">x</codex_internal_context>",
        "<agent_message_board_notification>x</agent_message_board_notification>",
        "Warning: apply_patch was requested via shell. Use the apply_patch tool instead of exec_command.",
        "Warning: Your account was flagged for potentially high-risk cyber activity",
        "<model_switch>x</model_switch>", "<persistent_mode>x</persistent_mode>",
        "Approved command prefix saved:\necho", "<permissions instructions>x</permissions instructions>",
    ];
    for text in text_cases {
        for role in ["user", "developer", "assistant"] {
            for kind in ["unknown", "user.text", "user.goal", "user.goal.omitted", "components.context"] {
                let item = message("fixed-id", role, text, kind).item;
                let ResponseItem::Message { content, .. } = &item else { unreachable!() };
                assert_eq!(
                    (codex_context_engine::semantics::is_guardian_context_message(&item),
                        codex_context_engine::semantics::is_user_authorization_message(&item),
                        codex_context_engine::semantics::is_contextual_dev_message_content(content),
                        codex_context_engine::semantics::has_non_contextual_dev_message_content(content),
                        codex_context_engine::is_user_turn_boundary(&item)),
                    (crate::context::is_guardian_context_message(&item),
                        crate::context::is_user_authorization_message(&item),
                        crate::event_mapping::is_contextual_dev_message_content(content),
                        crate::event_mapping::has_non_contextual_dev_message_content(content),
                        crate::context_manager::is_user_turn_boundary(&item)),
                );
                let user_text = |parsed| match parsed {
                    Some(TurnItem::UserMessage(message)) => Some(message.message()),
                    _ => None,
                };
                // UserMessageItem constructors mint UUIDs; compare parsed content instead.
                assert_eq!(user_text(codex_context_engine::semantics::parse_turn_item(&item)),
                    user_text(crate::event_mapping::parse_turn_item(&item)));
            }
        }
    }
}
