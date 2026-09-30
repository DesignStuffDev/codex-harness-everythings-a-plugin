//! Native legacy compaction reconstruction; inference remains host-owned.
use codex_context_fragments::ContextualUserFragment;
use codex_history::{CodexHarnessMetadata, ResponseItemEnvelope};
use codex_protocol::items::TurnItem;
use codex_protocol::models::{ContentItem, ContentItemKind, ResponseItem};
use codex_utils_output_truncation::{TruncationPolicy, approx_token_count, truncate_text};
use crate::fragments::CompactionSummary;
use codex_prompts::SUMMARY_PREFIX;
const COMPACT_USER_MESSAGE_MAX_TOKENS: usize = 20_000;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CompactedUserMessage<'a> {
    // Flattened text is only for the existing budget and truncation policy.
    // Whole text messages retain their exact content parts and annotations.
    // Borrow from the history snapshot until selected output is materialized.
    message: String,
    original: &'a ResponseItem,
    harness_metadata: Option<&'a CodexHarnessMetadata>,
}

pub(crate) fn collect_annotated_user_messages(
    items: &[ResponseItemEnvelope],
) -> Vec<CompactedUserMessage<'_>> {
    items
        .iter()
        .filter_map(|envelope| compacted_user_message(&envelope.item, envelope.metadata.as_ref()))
        .collect()
}

fn compacted_user_message<'a>(
    item: &'a ResponseItem,
    harness_metadata: Option<&'a CodexHarnessMetadata>,
) -> Option<CompactedUserMessage<'a>> {
    let Some(TurnItem::UserMessage(user)) = crate::semantics::parse_turn_item(item) else {
        return None;
    };
    let message = user.message();
    if is_summary_message(&message) {
        return None;
    }
    Some(CompactedUserMessage {
        message,
        original: item,
        harness_metadata,
    })
}

pub(crate) fn is_summary_message(message: &str) -> bool {
    message.starts_with(format!("{SUMMARY_PREFIX}\n").as_str())
}

pub(crate) fn build_compacted_history(
    initial_context: Vec<ResponseItemEnvelope>,
    user_messages: &[CompactedUserMessage<'_>],
    summary_text: &str,
) -> Vec<ResponseItemEnvelope> {
    build_compacted_history_with_limit(
        initial_context,
        user_messages,
        summary_text,
        COMPACT_USER_MESSAGE_MAX_TOKENS,
    )
}

fn build_compacted_history_with_limit(
    mut history: Vec<ResponseItemEnvelope>,
    user_messages: &[CompactedUserMessage<'_>],
    summary_text: &str,
    max_tokens: usize,
) -> Vec<ResponseItemEnvelope> {
    let mut selected_messages = Vec::new();
    if max_tokens > 0 {
        let mut remaining = max_tokens;
        for message in user_messages.iter().rev() {
            if remaining == 0 {
                break;
            }
            let tokens = approx_token_count(&message.message);
            let ResponseItem::Message {
                id,
                content,
                internal_chat_message_metadata_passthrough,
                ..
            } = message.original
            else {
                continue;
            };
            let mut passthrough = internal_chat_message_metadata_passthrough.clone();
            let mut harness_metadata = message.harness_metadata.cloned();
            let content = if tokens <= remaining
                && content
                    .iter()
                    .all(|part| matches!(part, ContentItem::InputText { .. }))
            {
                content.clone()
            } else {
                // Rebuild only the text fallback; never clone discarded media.
                if let Some(kinds) = passthrough
                    .as_mut()
                    .and_then(|metadata| metadata.content_item_kinds.as_mut())
                {
                    *kinds = vec![ContentItemKind("user.text".to_owned())];
                }
                vec![ContentItem::InputText {
                    text: truncate_text(&message.message, TruncationPolicy::Tokens(remaining)),
                }]
            };
            if tokens > remaining
                && let Some(metadata) = &mut harness_metadata
            {
                metadata.mark_retained_sources_incomplete();
            }
            selected_messages.push(ResponseItemEnvelope {
                item: ResponseItem::Message {
                    id: id.clone(),
                    role: "user".to_owned(),
                    content,
                    phase: None,
                    internal_chat_message_metadata_passthrough: passthrough,
                },
                metadata: harness_metadata,
            });
            if tokens > remaining {
                break;
            }
            remaining = remaining.saturating_sub(tokens);
        }
        selected_messages.reverse();
    }
    history.extend(selected_messages);

    let summary_text = if summary_text.is_empty() {
        "(no summary available)".to_string()
    } else {
        summary_text.to_string()
    };

    history.push(ResponseItemEnvelope::new(ContextualUserFragment::into(
        CompactionSummary::new(summary_text),
    )));

    history
}
