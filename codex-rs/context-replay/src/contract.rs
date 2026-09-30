//! Owned replay data and the native history-engine dependency boundary.

use codex_history::GuardianHistoryCheckpoint;
use codex_history::PreviousTurnSettings;
use codex_history::ResponseItemEnvelope;
use codex_history::RetainedContext;
use codex_history::RetainedContextEvent;
use codex_history::RolloutItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::TruncationPolicy;
use codex_protocol::protocol::TurnContextItem;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::WorldStateSnapshot;

/// Resolved host policy; a history engine receives no mutable feature/configuration object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewPolicy {
    pub independent_review: bool,
    pub retain_inherited_user_messages: bool,
}

/// The complete input formerly reached through Session and TurnContext.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayInput {
    #[serde(with = "codex_component_state_codec::rollout_item::vec")]
    pub items: Vec<RolloutItem>,
    pub history_mode: ThreadHistoryMode,
    pub truncation_policy: TruncationPolicy,
    pub review_policy: ReviewPolicy,
}

/// Item/evidence projection produced by the supplied history implementation.
pub struct HistoryProjection {
    pub history: Vec<ResponseItemEnvelope>,
    pub retained_context: RetainedContext,
    pub guardian_history: Option<GuardianHistoryCheckpoint>,
}

/// Reconstructed history and hydration metadata from the same surviving replay segments.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RolloutReconstruction {
    #[serde(with = "codex_component_state_codec::envelopes::vec")]
    pub history: Vec<ResponseItemEnvelope>,
    pub retained_context: RetainedContext,
    #[serde(with = "codex_component_state_codec::guardian_checkpoint::option")]
    pub guardian_history: Option<GuardianHistoryCheckpoint>,
    pub last_started_turn_id: Option<String>,
    pub previous_turn_settings: Option<PreviousTurnSettings>,
    #[serde(with = "codex_component_state_codec::turn_context::option")]
    pub reference_context_item: Option<TurnContextItem>,
    pub world_state_baseline: Option<WorldStateSnapshot>,
    pub window_number: u64,
    pub first_window_id: Option<Uuid>,
    pub previous_window_id: Option<Uuid>,
    pub window_id: Option<Uuid>,
}

/// Native item-history semantics needed by the pure replay reducer.
///
/// Implementations are local synchronous algorithms, not synchronous RPC adapters.
/// The future process service must await one complete reconstruction at the existing
/// async host seam, while running these operations inside the component process.
/// Ordinary append, retained-evidence capture, rollback, and legacy migration must
/// preserve their native policy; a Vec-only replacement does not satisfy this contract.
pub trait ReplayHistory: Sized {
    fn new(policy: ReviewPolicy) -> Self;
    fn is_user_turn_boundary(item: &ResponseItem) -> bool;
    fn replace_annotated(&mut self, items: Vec<ResponseItemEnvelope>);
    fn restore_review_context(
        &mut self,
        retained_context: Option<&RetainedContext>,
        checkpoint: Option<&GuardianHistoryCheckpoint>,
        reviewer_compaction_hash: Option<&str>,
    );
    fn record_retained_context(&mut self, event: &RetainedContextEvent);
    fn replay_annotated_item(&mut self, item: &ResponseItemEnvelope, policy: TruncationPolicy);
    fn record_legacy_communication(&mut self, item: &ResponseItem, policy: TruncationPolicy);
    /// Rebuilds a legacy summary while retaining original user-message provenance/evidence.
    fn apply_legacy_compaction(&mut self, summary: &str);
    fn drop_last_n_user_turns(&mut self, num_turns: u32);
    fn finish(self) -> HistoryProjection;
}
