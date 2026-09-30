//! Explicit capabilities previously inferred from the concrete local store type.

use codex_protocol::protocol::SessionSource;
use serde::Deserialize;
use serde::Serialize;

/// Persisted facts needed to resume metadata observation without overwriting an
/// existing title, preview, first user message or guardian classification.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeMetadata {
    pub guardian_review: bool,
    pub preview_seen: bool,
    pub first_user_message_seen: bool,
    pub title_seen: bool,
}

impl From<&codex_state::ThreadMetadata> for ResumeMetadata {
    fn from(metadata: &codex_state::ThreadMetadata) -> Self {
        Self {
            guardian_review: serde_json::from_str::<SessionSource>(&metadata.source)
                .as_ref()
                .is_ok_and(codex_state::is_guardian_review_source),
            preview_seen: metadata
                .preview
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
            first_user_message_seen: metadata.first_user_message.is_some(),
            title_seen: !metadata.title.is_empty(),
        }
    }
}

/// Background maintenance requests. Completion acknowledges scheduling, matching
/// the existing local startup and rollout/compress behavior.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RolloutMaintenance {
    Startup {
        migrate_rollouts: bool,
        compress_rollouts: bool,
    },
    MigrateOnStartup,
    Compress,
}
