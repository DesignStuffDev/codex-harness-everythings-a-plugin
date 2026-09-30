//! Native ReplayHistory semantic port: all operations execute in this package.
use super::ContextManager;
use codex_context_replay::HistoryProjection;
use codex_context_replay::ReplayHistory;
use codex_context_replay::ReviewPolicy;
use codex_history::GuardianHistoryCheckpoint;
use codex_history::ResponseItemEnvelope;
use codex_history::RetainedContext;
use codex_history::RetainedContextEvent;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TruncationPolicy;

impl ReplayHistory for ContextManager {
    fn new(policy: ReviewPolicy) -> Self {
        Self::for_policy(policy)
    }

    fn is_user_turn_boundary(item: &ResponseItem) -> bool {
        super::is_user_turn_boundary(item)
    }

    fn replace_annotated(&mut self, items: Vec<ResponseItemEnvelope>) {
        ContextManager::replace_annotated(self, items);
    }

    fn restore_review_context(
        &mut self,
        retained_context: Option<&RetainedContext>,
        checkpoint: Option<&GuardianHistoryCheckpoint>,
        reviewer_compaction_hash: Option<&str>,
    ) {
        ContextManager::restore_review_context(self, retained_context, checkpoint, reviewer_compaction_hash);
    }

    fn record_retained_context(&mut self, event: &RetainedContextEvent) {
        ContextManager::record_retained_context(self, event);
    }

    fn replay_annotated_item(&mut self, item: &ResponseItemEnvelope, policy: TruncationPolicy) {
        ContextManager::replay_annotated_item(self, item, policy);
    }

    fn record_legacy_communication(&mut self, item: &ResponseItem, policy: TruncationPolicy) {
        self.record_items(std::iter::once(item), policy);
    }

    fn apply_legacy_compaction(&mut self, summary: &str) {
        let user_messages = crate::compact::collect_annotated_user_messages(self.annotated_items());
        let rebuilt = crate::compact::build_compacted_history(Vec::new(), &user_messages, summary);
        let retained_context = self.retained_context().clone();
        ContextManager::replace_annotated(self, rebuilt);
        self.restore_retained_context(Some(&retained_context));
    }

    fn drop_last_n_user_turns(&mut self, num_turns: u32) {
        ContextManager::drop_last_n_user_turns(self, num_turns);
    }

    fn finish(self) -> HistoryProjection {
        HistoryProjection {
            retained_context: self.retained_context().clone(),
            guardian_history: self.guardian_history_checkpoint(),
            history: self.into_annotated_items(),
        }
    }
}
