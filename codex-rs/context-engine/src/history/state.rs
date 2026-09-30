//! Native ContextManager methods, lifted without changing algorithm bodies.

use super::*;

impl ContextManager {
    pub fn new() -> Self {
        Self {
            items: Arc::new(Vec::new()),
            review_history: None,
            retained_context: Arc::default(),
            guardian_review_mode: GuardianContextMode::ThreadOwned,
            retain_inherited_user_messages: false,
            history_version: 0,
            reset_version: 0,
            user_message_revision: 0,
            guardian_review_context_revision: next_guardian_review_context_revision(),
            token_info: TokenUsageInfo::new_or_append(
                &None, &None, /*model_context_window*/ None,
            ),
            reference_context_item: None,
            world_state_baseline: None,
        }
    }

    pub fn retained_context(&self) -> &RetainedContext {
        &self.retained_context
    }

    pub fn reserve_input_order(&mut self) -> u64 {
        Arc::make_mut(&mut self.retained_context).reserve_order()
    }

    pub fn record_retained_context(&mut self, event: &RetainedContextEvent) -> bool {
        if !Arc::make_mut(&mut self.retained_context).record(event) {
            return false;
        }
        match event {
            RetainedContextEvent::VerifiedAnswer { .. } => {
                self.user_message_revision =
                    self.user_message_revision.saturating_add(/*rhs*/ 1);
            }
            RetainedContextEvent::DeliveredAssistantMessage { .. } => {
                self.guardian_review_context_revision = next_guardian_review_context_revision();
            }
        }
        true
    }

    pub fn set_reference_context_item(&mut self, item: Option<TurnContextItem>) {
        self.reference_context_item = item;
    }

    pub fn reference_context_item(&self) -> Option<TurnContextItem> {
        self.reference_context_item.clone()
    }

    pub fn set_world_state_baseline(&mut self, snapshot: WorldStateSnapshot) {
        self.world_state_baseline = Some(snapshot);
    }

    pub fn world_state_checkpoint(&self) -> Option<WorldStateItem> {
        self.world_state_baseline
            .clone()
            .map(|snapshot| WorldStateItem::full(snapshot.into_object()))
    }

    pub fn history_version(&self) -> u64 {
        self.history_version
    }
}
