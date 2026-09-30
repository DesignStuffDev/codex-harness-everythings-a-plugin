//! Native model-window, retained-evidence and token state; no core callbacks.

mod recording;
mod review;
mod rollback;
mod state;
mod token_accounting;
#[path = "history/user_authorization.rs"]
mod user_authorization;
#[path = "history/replay.rs"]
mod replay;
#[cfg(feature = "snapshot-api")]
#[path = "history/snapshot.rs"]
mod snapshot;

use crate::guardian::GuardianContextMode;
use crate::guardian::GUARDIAN_MAX_ROOT_MESSAGE_TOKENS;
use crate::guardian::guardian_truncate_text;
use crate::normalize;
use crate::semantics::has_non_contextual_dev_message_content;
use crate::semantics::is_contextual_dev_message_content;
use crate::semantics::is_contextual_user_message_content;
use crate::semantics::is_guardian_context_message;
use crate::semantics::parse_turn_item;
use crate::token_estimation::estimate_item_token_count;
use codex_context_fragments::set_annotated_content;
use codex_context_fragments::to_annotated_content;
use codex_context_replay::ReviewPolicy;
use codex_context_replay::WorldStateSnapshot;
use codex_guardian_context::SectionHistory;
use codex_guardian_context::TranscriptHistory;
use codex_history::CodexHarnessMetadata;
use codex_history::GuardianHistoryCheckpoint;
use codex_history::ResponseItemEnvelope;
use codex_history::RetainedContext;
use codex_history::RetainedContextEntry;
use codex_history::RetainedContextEvent;
use codex_history::RetainedInputSource;
use codex_protocol::items::TurnItem;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::InputModality;
use codex_protocol::protocol::InterAgentCommunication;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TokenUsageInfo;
use codex_protocol::protocol::TurnContextItem;
use codex_protocol::protocol::WorldStateItem;
use codex_utils_audio::estimate_audio_token_count;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::approx_token_count;
use codex_utils_output_truncation::truncate_function_output_payload;
use codex_utils_output_truncation::with_serialization_allowance;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

static GUARDIAN_REVIEW_CONTEXT_REVISION: AtomicU64 = AtomicU64::new(/*v*/ 1);

fn next_guardian_review_context_revision() -> u64 {
    GUARDIAN_REVIEW_CONTEXT_REVISION.fetch_add(/*val*/ 1, Ordering::Relaxed)
}

/// Transcript of thread history
#[derive(Debug, Clone)]
pub struct ContextManager {
    /// The oldest items are at the beginning of the vector. Snapshots share the vector until a
    /// caller needs to mutate it, avoiding deep copies for read-only history consumers.
    items: Arc<Vec<ResponseItemEnvelope>>,
    /// Compatibility history for legacy review and missing root instructions.
    review_history: Option<TranscriptHistory>,
    /// Host facts independent of the model window; snapshots share immutable state.
    retained_context: Arc<RetainedContext>,
    /// Reviewer policy travels with the history snapshot, independently of capture.
    guardian_review_mode: GuardianContextMode,
    retain_inherited_user_messages: bool,
    /// Bumped whenever history is rewritten, such as compaction or rollback.
    history_version: u64,
    /// Last destructive history replacement; ordinary input and compaction preserve it.
    pub reset_version: u64,
    /// Monotonic user-input/reset revision, independent of compaction's history generation.
    user_message_revision: u64,
    /// Process-unique so resumed roots cannot match a worker's cached assistant evidence.
    guardian_review_context_revision: u64,
    token_info: Option<TokenUsageInfo>,
    /// Reference context snapshot used for diffing and producing model-visible
    /// settings update items.
    ///
    /// This is the baseline for the next regular model turn, and may already
    /// match the current turn after context updates are persisted.
    ///
    /// When this is `None`, settings diffing treats the next turn as having no
    /// baseline and emits a full reinjection of context state. Rollback may
    /// also clear this when it trims a mixed initial-context developer bundle
    /// whose non-diff fragments no longer exist in the surviving history.
    reference_context_item: Option<TurnContextItem>,
    /// World-state comparison checkpoint. After compaction this may contain only
    /// extension metadata, with model-visible context still awaiting reinjection.
    world_state_baseline: Option<WorldStateSnapshot>,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextManager {
    /// Resolved session policy; callers keep configuration and source selection.
    pub fn for_policy(policy: ReviewPolicy) -> Self {
        let independent = policy.independent_review;
        Self {
            retain_inherited_user_messages: policy.retain_inherited_user_messages,
            guardian_review_mode: if independent {
                GuardianContextMode::Independent
            } else {
                GuardianContextMode::ThreadOwned
            },
            review_history: independent.then(TranscriptHistory::default),
            ..Self::new()
        }
    }

    /// Immutable baseline for host-owned world-state renderers.
    pub fn world_state_baseline(&self) -> Option<&WorldStateSnapshot> {
        self.world_state_baseline.as_ref()
    }
}

fn is_api_message(message: &ResponseItem, metadata: Option<&CodexHarnessMetadata>) -> bool {
    match message {
        ResponseItem::Message { role, .. } => role.as_str() != "system",
        ResponseItem::ConfigurationUpdate { .. } => {
            metadata.is_some_and(|metadata| metadata.harness_authored_configuration)
        }
        ResponseItem::AdditionalTools { .. }
        | ResponseItem::AgentMessage { .. }
        | ResponseItem::FunctionCallOutput { .. }
        | ResponseItem::FunctionCall { .. }
        | ResponseItem::ToolSearchCall { .. }
        | ResponseItem::ToolSearchOutput { .. }
        | ResponseItem::CustomToolCall { .. }
        | ResponseItem::CustomToolCallOutput { .. }
        | ResponseItem::LocalShellCall { .. }
        | ResponseItem::Reasoning { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::ContextCompaction { .. } => true,
        ResponseItem::CompactionTrigger { .. } => false,
        ResponseItem::Other => false,
    }
}

fn is_model_generated_item(item: &ResponseItem) -> bool {
    match item {
        ResponseItem::Message { role, .. } => role == "assistant",
        ResponseItem::Reasoning { .. }
        | ResponseItem::FunctionCall { .. }
        | ResponseItem::ToolSearchCall { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::CustomToolCall { .. }
        | ResponseItem::LocalShellCall { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::ContextCompaction { .. } => true,
        ResponseItem::ConfigurationUpdate { .. } | ResponseItem::CompactionTrigger { .. } => false,
        ResponseItem::AdditionalTools { .. }
        | ResponseItem::FunctionCallOutput { .. }
        | ResponseItem::ToolSearchOutput { .. }
        | ResponseItem::CustomToolCallOutput { .. }
        | ResponseItem::AgentMessage { .. }
        | ResponseItem::Other => false,
    }
}

pub fn is_user_turn_boundary(item: &ResponseItem) -> bool {
    if matches!(item, ResponseItem::AgentMessage { .. }) {
        return true;
    }
    let ResponseItem::Message { role, content, .. } = item else {
        return false;
    };

    (role == "user"
        && !is_contextual_user_message_content(content)
        // Local compaction persists its synthetic summary as a user-role message.
        // It must not consume a rollback turn or become the transcript boundary.
        && !content.iter().any(|part| matches!(part,
            ContentItem::InputText { text } if crate::compact::is_summary_message(text))))
        || (role == "assistant" && is_inter_agent_instruction_content(content))
}

fn is_inter_agent_instruction_content(content: &[ContentItem]) -> bool {
    InterAgentCommunication::is_message_content(content)
}

fn user_message_positions(items: &[ResponseItemEnvelope]) -> Vec<usize> {
    let mut positions = Vec::new();
    for (idx, envelope) in items.iter().enumerate() {
        if is_user_turn_boundary(&envelope.item) {
            positions.push(idx);
        }
    }
    positions
}
