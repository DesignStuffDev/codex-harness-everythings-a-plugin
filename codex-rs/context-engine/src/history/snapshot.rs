//! Existing snapshot contract, enabled by the host-facing snapshot-api feature.
use super::*;
use codex_extension_api::ConversationHistorySnapshot;

struct SharedConversationHistory {
    items: Arc<Vec<ResponseItemEnvelope>>,
    review_history: Option<TranscriptHistory>,
    retained_context: Arc<RetainedContext>,
    guardian_review_mode: GuardianContextMode,
    history_version: u64,
    user_message_revision: u64,
    guardian_review_context_revision: u64,
}

impl ConversationHistorySnapshot for SharedConversationHistory {
    fn latest_compaction(&self) -> Option<codex_history::CompactionCheckpoint<'_>> {
        codex_history::CompactionCheckpoint::latest(&self.items)
    }

    fn retained_context(&self) -> Option<&RetainedContext> {
        Some(&self.retained_context)
    }

    fn uses_parent_context_for_review(&self) -> bool {
        self.guardian_review_mode == GuardianContextMode::ThreadOwned
    }

    fn uses_independent_review_history(&self) -> bool {
        self.guardian_review_mode == GuardianContextMode::Independent
    }

    fn review_items(&self) -> Box<dyn Iterator<Item = &ResponseItem> + Send + '_> {
        Box::new(self.review_items_with_sources().map(|(item, _)| item))
    }

    fn review_items_with_sources(
        &self,
    ) -> Box<dyn Iterator<Item = (&ResponseItem, Option<&codex_history::RetainedSource>)> + Send + '_>
    {
        if self.guardian_review_mode != GuardianContextMode::ThreadOwned {
            let items = self
                .review_history
                .as_ref()
                .map_or_else(|| self.items(), SectionHistory::items);
            return Box::new(items.map(|item| (item, None)));
        }
        Box::new(self.items_with_sources())
    }

    fn review_history_version(&self) -> u64 {
        if self.guardian_review_mode != GuardianContextMode::ThreadOwned {
            return self
                .review_history
                .as_ref()
                .map_or(self.history_version, TranscriptHistory::generation);
        }
        self.history_version
    }

    fn history_version(&self) -> u64 {
        self.history_version
    }

    fn user_message_revision(&self) -> u64 {
        self.user_message_revision
    }

    fn guardian_review_context_revision(&self) -> u64 {
        self.guardian_review_context_revision
    }

    fn items(&self) -> Box<dyn Iterator<Item = &ResponseItem> + Send + '_> {
        Box::new(self.items_with_sources().map(|(item, _)| item))
    }
}

impl SharedConversationHistory {
    fn items_with_sources(
        &self,
    ) -> impl Iterator<Item = (&ResponseItem, Option<&codex_history::RetainedSource>)> + Send + '_
    {
        self.items
            .iter()
            .filter(|envelope| !is_guardian_context_message(&envelope.item))
            .map(|envelope| {
                (
                    &envelope.item,
                    envelope
                        .metadata
                        .as_ref()
                        .and_then(|metadata| metadata.retained_source.as_ref()),
                )
            })
    }
}

impl ContextManager {
    pub fn conversation_history_snapshot(&self) -> Arc<dyn ConversationHistorySnapshot> {
        Arc::new(SharedConversationHistory {
            items: Arc::clone(&self.items),
            review_history: self.review_history.clone(),
            retained_context: Arc::clone(&self.retained_context),
            guardian_review_mode: self.guardian_review_mode,
            history_version: self.history_version,
            user_message_revision: self.user_message_revision,
            guardian_review_context_revision: self.guardian_review_context_revision,
        })
    }
}
