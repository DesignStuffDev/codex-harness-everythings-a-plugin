//! Native ContextManager methods, lifted without changing algorithm bodies.

use super::*;

impl ContextManager {

    /// Original checkpoint evidence, independent of the selected review window.
    pub fn guardian_history_items(
        &self,
    ) -> Option<Box<dyn Iterator<Item = &ResponseItem> + Send + '_>> {
        self.review_history.as_ref().map(SectionHistory::items)
    }

    pub fn guardian_history_checkpoint(&self) -> Option<GuardianHistoryCheckpoint> {
        self.review_history
            .as_ref()
            .map(TranscriptHistory::checkpoint)
    }

    pub fn restore_review_context(
        &mut self,
        retained_context: Option<&RetainedContext>,
        checkpoint: Option<&GuardianHistoryCheckpoint>,
        reviewer_compaction_hash: Option<&str>,
    ) {
        // A previously promoted checkpoint may have discarded the only complete transcript.
        // Unless independent review was explicitly selected, keep requiring parent context;
        // a compatibility failure must not turn
        // a partial model window into a legacy fallback. Migrating checkpoints keep a backup
        // and can expose retained facts independently of which transcript review uses.
        let requires_parent_context = checkpoint.is_none()
            && retained_context.is_some_and(|context| {
                !context.verified_answers_complete()
                    || context.ordered_entries().any(|(_, entry)| match entry {
                        RetainedContextEntry::VerifiedAnswer(_) => true,
                        RetainedContextEntry::UserMessage(message)
                        | RetainedContextEntry::AssistantMessage(message) => {
                            let source_role =
                                if matches!(entry, RetainedContextEntry::UserMessage(_)) {
                                    "user"
                                } else {
                                    "assistant"
                                };
                            !self.raw_items().any(|item| {
                                if item.id().map(codex_protocol::ResponseItemId::as_str)
                                    != message.message_id.as_deref()
                                    || item.turn_id().unwrap_or_default() != message.turn_id
                                {
                                    return false;
                                }
                                let ResponseItem::Message { role, content, .. } = item else {
                                    return false;
                                };
                                if role != source_role || is_guardian_context_message(item) {
                                    return false;
                                }
                                let text = content
                                    .iter()
                                    .filter_map(|content| match content {
                                        ContentItem::InputText { text }
                                        | ContentItem::OutputText { text } => Some(text.as_str()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                guardian_truncate_text(&text, GUARDIAN_MAX_ROOT_MESSAGE_TOKENS).0
                                    == message.text
                            })
                        }
                    })
            });
        self.guardian_review_mode = if self.guardian_review_mode == GuardianContextMode::Independent
        {
            GuardianContextMode::Independent
        } else if requires_parent_context {
            GuardianContextMode::ThreadOwned
        } else {
            GuardianContextMode::for_checkpoint(&self.items, reviewer_compaction_hash)
        };
        self.restore_retained_context(retained_context);
        // Older retained checkpoints cleared oversized instructions. Recover their
        // bounded root excerpts before discarding the legacy source transcript.
        let items = &self.items;
        Arc::make_mut(&mut self.retained_context).recover_user_message_excerpts(|id| {
            // Prefer the backup over a compacted copy that retains the original ID.
            let original = checkpoint
                .into_iter()
                .flat_map(|checkpoint| checkpoint.0.iter().map(|entry| &entry.item))
                .chain(items.iter().map(|envelope| &envelope.item))
                .find(|item| item.id().is_some_and(|item_id| item_id.as_str() == id));
            let Some(TurnItem::UserMessage(original)) = original.and_then(parse_turn_item) else {
                return None;
            };
            Some(guardian_truncate_text(&original.message(), GUARDIAN_MAX_ROOT_MESSAGE_TOKENS).0)
        });
        let retain_legacy_authorization = self.retain_inherited_user_messages
            && self.retained_context.has_missing_user_messages()
            && (checkpoint.is_some()
                // A text checkpoint can predate both retained facts and Guardian backups.
                // Preserve its surviving instructions without treating an opaque checkpoint's
                // partial model window as a complete compatibility transcript.
                || codex_history::CompactionCheckpoint::latest(&self.items).is_none());
        if self.guardian_review_mode == GuardianContextMode::ThreadOwned
            && !retain_legacy_authorization
        {
            self.review_history = None;
            return;
        }
        let generation = self
            .review_history
            .as_ref()
            .map_or(self.history_version, TranscriptHistory::generation)
            .saturating_add(1);
        let mut history = TranscriptHistory::new(generation);
        if let Some(checkpoint) = checkpoint {
            history.reset(checkpoint.0.iter());
        } else {
            // Old checkpoints may predate independent capture. Start with the surviving
            // raw window; retained instructions/answers still report their own omissions.
            // Subsequent checkpoints persist this bounded transcript across compaction.
            history.reset(
                self.annotated_items()
                    .iter()
                    .filter(|entry| !is_guardian_context_message(&entry.item)),
            );
        }
        self.review_history = Some(history);
        if self.guardian_review_mode == GuardianContextMode::ThreadOwned
            && !self.has_legacy_user_messages()
        {
            self.review_history = None;
        }
    }

    pub fn replace_annotated(&mut self, items: Vec<ResponseItemEnvelope>) {
        self.retained_context = Arc::default();
        self.user_message_revision = self.user_message_revision.saturating_add(1);
        if let Some(review_history) = &mut self.review_history {
            review_history.reset(
                items
                    .iter()
                    .filter(|entry| !is_guardian_context_message(&entry.item)),
            );
        }
        self.items = Arc::new(items);
        self.history_version = self.history_version.saturating_add(1);
        self.reset_version = self.history_version;
        self.world_state_baseline = None;
    }

    /// Returns whether compaction changed Guardian's evidence policy, invalidating older reviews.
    pub fn replace_compacted(
        &mut self,
        items: Vec<ResponseItemEnvelope>,
        reviewer_compaction_hash: Option<&str>,
    ) -> bool {
        let promoted = self.guardian_review_mode == GuardianContextMode::Legacy
            && GuardianContextMode::for_checkpoint(&items, reviewer_compaction_hash)
                == GuardianContextMode::ThreadOwned;
        if promoted {
            self.guardian_review_mode = GuardianContextMode::ThreadOwned;
            self.user_message_revision = self.user_message_revision.saturating_add(/*rhs*/ 1);
        }
        if self.guardian_review_mode == GuardianContextMode::ThreadOwned
            && (!self.retain_inherited_user_messages
                || !self.retained_context.has_missing_user_messages()
                || !self.has_legacy_user_messages())
        {
            self.review_history = None;
        }
        if self.guardian_review_mode == GuardianContextMode::Legacy && self.review_history.is_none()
        {
            let mut retained = TranscriptHistory::new(self.history_version.saturating_add(1));
            for item in self
                .annotated_items()
                .iter()
                .filter(|entry| !is_guardian_context_message(&entry.item))
            {
                retained.record(item);
            }
            self.review_history = Some(retained);
        }
        self.items = Arc::new(items);
        self.history_version = self.history_version.saturating_add(1);
        if promoted {
            self.reset_version = self.history_version;
        }
        self.world_state_baseline = None;
        promoted
    }
}
