//! Native ContextManager methods, lifted without changing algorithm bodies.

use super::*;

impl ContextManager {

    /// Drop the last `num_turns` instruction turns from this history.
    ///
    /// Instruction turns are history messages that should behave like a new prompt boundary:
    /// ordinary user messages and structured assistant inter-agent instructions.
    ///
    /// Used only to replay historical rollback markers when reconstructing a saved rollout:
    /// - `num_turns == 0` is a no-op
    /// - if there are no user turns, this is a no-op
    /// - if `num_turns` exceeds the number of user turns, all user turns are dropped while
    ///   preserving any items that occurred before the first user message.
    ///
    /// If rollback trims a pre-turn developer message that mixes contextual fragments with
    /// persistent developer text from `build_initial_context`, this also clears
    /// `reference_context_item`. The surviving history no longer contains the full bundle that
    /// established the prior baseline, so future turns must fall back to full reinjection instead
    /// of diffing against stale state.
    pub fn drop_last_n_user_turns(&mut self, num_turns: u32) {
        if num_turns == 0 {
            return;
        }

        let snapshot = self.items.clone();
        let user_positions = user_message_positions(&snapshot);
        let Some(&first_instruction_turn_idx) = user_positions.first() else {
            let retained_context = Arc::clone(&self.retained_context);
            self.replace_annotated(Arc::unwrap_or_clone(snapshot));
            self.retained_context = retained_context;
            return;
        };

        let n_from_end = usize::try_from(num_turns).unwrap_or(usize::MAX);
        let mut cut_idx = if n_from_end >= user_positions.len() {
            first_instruction_turn_idx
        } else {
            user_positions[user_positions.len() - n_from_end]
        };

        let first_removed_message_id = snapshot[cut_idx]
            .id()
            .map(codex_protocol::ResponseItemId::as_str);
        let source = RetainedInputSource::from(snapshot[cut_idx].metadata.as_ref());
        let mut review_history = self.review_history.take();
        if let Some(history) = &mut review_history {
            history.truncate_before(&snapshot[cut_idx]);
        }

        cut_idx =
            self.trim_pre_turn_context_updates(&snapshot, first_instruction_turn_idx, cut_idx);

        // Apply the same acceptance boundary to the parent model window. The independent
        // transcript owns its rollback provenance even after this window is compacted.
        let mut retained_items = snapshot[..cut_idx]
            .iter()
            .filter(|envelope| {
                !source.acceptance_order().is_some_and(|boundary| {
                    (matches!(&envelope.item, ResponseItem::Message { role, .. } if role == "assistant")
                        || matches!(&envelope.item, ResponseItem::FunctionCall { .. }))
                        && RetainedInputSource::from(envelope.metadata.as_ref())
                            .acceptance_order().is_some_and(|order| order >= boundary)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if cut_idx == first_instruction_turn_idx
            && let Some(first_turn_id) = snapshot[first_instruction_turn_idx].turn_id()
        {
            retained_items.retain_mut(|item| {
                if item.turn_id() == Some(first_turn_id)
                    && matches!(&item.item, ResponseItem::Message { role, .. } if role == "developer")
                {
                    let Some(mut content) = to_annotated_content(&mut item.item) else {
                        return false;
                    };
                    content.retain(|content| {
                        // Rebuild these from the next step's model and effort after rollback.
                        !matches!(
                            content.content(),
                            ContentItem::InputText { text }
                                if crate::semantics::is_model_switch_text(text)
                                    || crate::semantics::is_persistent_mode_text(text)
                        )
                    });
                    !content.is_empty() && set_annotated_content(&mut item.item, content).is_some()
                } else {
                    true
                }
            });
        }

        let mut retained_context = Arc::clone(&self.retained_context);
        let removed_turns = snapshot[cut_idx..]
            .iter()
            .filter_map(|item| item.turn_id())
            .collect::<Vec<_>>();
        // Old checkpoints lack an accepted-input boundary. Their answers still follow
        // the original source calls, even after the capture opt-out has been retired.
        if source == RetainedInputSource::Inherited
            || source.acceptance_order().is_some()
            || retained_context
                .ordered_entries()
                .any(|(_, entry)| matches!(entry, RetainedContextEntry::UserMessage(_)))
        {
            Arc::make_mut(&mut retained_context).rollback(
                &removed_turns,
                first_removed_message_id,
                source,
            );
        } else {
            Arc::make_mut(&mut retained_context).retain_answers(|answer| {
                // Legacy answers follow their original call, not later steers in the same turn.
                if let Some(source_index) = snapshot.iter().rposition(|item| {
                    item.turn_id() == Some(answer.turn_id.as_str())
                        && matches!(&item.item, ResponseItem::FunctionCall { call_id, .. }
                            if call_id == &answer.call_id)
                }) {
                    return source_index < cut_idx;
                }
                !removed_turns.contains(&answer.turn_id.as_str())
            });
        }
        self.replace_annotated(retained_items);
        self.retained_context = retained_context;
        self.review_history = review_history;
    }

    /// Walk backward from a rollback cut and trim contiguous pre-turn context-update items.
    ///
    /// Returns the adjusted cut index after removing contextual developer/user items immediately
    /// above the rolled-back turn boundary.
    ///
    /// `first_instruction_turn_idx` is the earliest rollback-eligible instruction-turn boundary
    /// in `snapshot`; the trim walk never crosses it so any session-prefix items that predate the
    /// first real turn survive rollback.
    ///
    /// `cut_idx` is the tentative slice boundary after dropping the requested number of
    /// instruction turns, before stripping contextual pre-turn items that sit immediately above
    /// that boundary.
    ///
    /// If any trimmed developer message was a mixed `build_initial_context` bundle containing both
    /// rollback-trimmable contextual fragments and persistent developer text, this also clears the
    /// stored `reference_context_item` baseline so the next real turn falls back to full
    /// reinjection.
    pub(super) fn trim_pre_turn_context_updates(
        &mut self,
        snapshot: &[ResponseItemEnvelope],
        first_instruction_turn_idx: usize,
        mut cut_idx: usize,
    ) -> usize {
        while cut_idx > first_instruction_turn_idx {
            match &snapshot[cut_idx - 1].item {
                ResponseItem::Message { role, content, .. }
                    if role == "developer" && is_contextual_dev_message_content(content) =>
                {
                    if has_non_contextual_dev_message_content(content) {
                        // Mixed `build_initial_context` bundles are not reconstructible from
                        // steady-state diffs once trimmed, so the next real turn must fully
                        // reinject context instead of diffing against a stale baseline.
                        self.reference_context_item = None;
                    }
                    cut_idx -= 1;
                }
                ResponseItem::Message { role, content, .. }
                    if role == "user" && is_contextual_user_message_content(content) =>
                {
                    cut_idx -= 1;
                }
                _ => break,
            }
        }
        cut_idx
    }
}
