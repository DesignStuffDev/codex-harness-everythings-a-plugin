//! Native ContextManager methods, lifted without changing algorithm bodies.

use super::*;

impl ContextManager {

    pub fn token_info(&self) -> Option<TokenUsageInfo> {
        self.token_info.clone()
    }

    pub fn set_token_info(&mut self, info: Option<TokenUsageInfo>) {
        self.token_info = info;
    }

    pub fn set_token_usage_full(&mut self, context_window: i64) {
        match &mut self.token_info {
            Some(info) => info.fill_to_context_window(context_window),
            None => {
                self.token_info = Some(TokenUsageInfo::full_context_window(context_window));
            }
        }
    }

    pub fn estimate_token_count_with_base_instructions(
        &self,
        base_instructions: &BaseInstructions,
    ) -> Option<i64> {
        let base_tokens =
            i64::try_from(approx_token_count(&base_instructions.text)).unwrap_or(i64::MAX);

        let items_tokens = self
            .items
            .iter()
            .map(|envelope| estimate_item_token_count(&envelope.item))
            .fold(0i64, i64::saturating_add);

        Some(base_tokens.saturating_add(items_tokens))
    }

    pub fn update_token_info(
        &mut self,
        usage: &TokenUsage,
        model_context_window: Option<i64>,
    ) {
        self.token_info = TokenUsageInfo::new_or_append(
            &self.token_info,
            &Some(usage.clone()),
            model_context_window,
        );
    }

    pub(super) fn get_non_last_reasoning_items_tokens(&self) -> i64 {
        // Get reasoning items excluding all the ones after the last instruction boundary.
        let Some(last_user_index) = self
            .items
            .iter()
            .rposition(|envelope| is_user_turn_boundary(&envelope.item))
        else {
            return 0;
        };

        self.items
            .iter()
            .take(last_user_index)
            .filter(|envelope| {
                matches!(
                    &envelope.item,
                    ResponseItem::Reasoning {
                        encrypted_content: Some(_),
                        ..
                    }
                )
            })
            .map(|envelope| estimate_item_token_count(&envelope.item))
            .fold(0i64, i64::saturating_add)
    }

    // These are local items added after the most recent model-emitted item.
    // They are not reflected in `last_token_usage.total_tokens`.
    pub(super) fn items_after_last_model_generated_item(
        &self,
    ) -> impl Clone + ExactSizeIterator<Item = &ResponseItem> + DoubleEndedIterator {
        let start = self
            .items
            .iter()
            .rposition(|envelope| is_model_generated_item(&envelope.item))
            .map_or(self.items.len(), |index| index.saturating_add(1));
        self.items[start..].iter().map(|envelope| &envelope.item)
    }

    /// When true, the server already accounted for past reasoning tokens and
    /// the client should not re-estimate them.
    pub fn get_total_token_usage(&self, server_reasoning_included: bool) -> i64 {
        let last_tokens = self
            .token_info
            .as_ref()
            .map(|info| info.last_token_usage.total_tokens)
            .unwrap_or(0);
        let items_after_last_model_generated_tokens = self
            .items_after_last_model_generated_item()
            .map(estimate_item_token_count)
            .fold(0i64, i64::saturating_add);
        if server_reasoning_included {
            last_tokens.saturating_add(items_after_last_model_generated_tokens)
        } else {
            last_tokens
                .saturating_add(self.get_non_last_reasoning_items_tokens())
                .saturating_add(items_after_last_model_generated_tokens)
        }
    }

    pub fn estimated_tokens_after_last_model_generated_item(&self) -> i64 {
        self.items_after_last_model_generated_item()
            .map(estimate_item_token_count)
            .fold(0i64, i64::saturating_add)
    }
}
