//! Persisted marker recognition without importing the producers' runtime services.
//!
//! Every producer below uses `ContextualUserFragment`'s default matcher. Keep its
//! exact algorithm and marker pairs here until that private helper is exposed by
//! `codex-context-fragments`; custom matchers remain on their native fragment types.

use codex_protocol::protocol::ENVIRONMENT_CONTEXT_CLOSE_TAG;
use codex_protocol::protocol::ENVIRONMENT_CONTEXT_OPEN_TAG;

pub(super) fn is_environment_context_text(text: &str) -> bool {
    matches_marked_text(
        ENVIRONMENT_CONTEXT_OPEN_TAG,
        ENVIRONMENT_CONTEXT_CLOSE_TAG,
        text,
    )
}

pub(super) fn is_agent_message_board_notification_text(text: &str) -> bool {
    matches_marked_text(
        "<agent_message_board_notification>",
        "</agent_message_board_notification>",
        text,
    )
}

pub(super) fn is_recommended_plugins_text(text: &str) -> bool {
    matches_marked_text("<recommended_plugins>", "</recommended_plugins>", text)
}

pub(super) fn is_skill_prompt_fragment(text: &str) -> bool {
    matches_marked_text("<skill>", "</skill>", text)
}

/// Recognizes the native model-switch wrapper retained in developer history.
pub fn is_model_switch_text(text: &str) -> bool {
    matches_marked_text("<model_switch>", "</model_switch>", text)
}

/// Recognizes the native persistent-mode wrapper retained in developer history.
pub fn is_persistent_mode_text(text: &str) -> bool {
    matches_marked_text("<persistent_mode>", "</persistent_mode>", text)
}

fn matches_marked_text(start_marker: &str, end_marker: &str, text: &str) -> bool {
    if start_marker.is_empty() || end_marker.is_empty() {
        return false;
    }

    let trimmed = text.trim_start();
    let starts_with_marker = trimmed
        .get(..start_marker.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(start_marker));
    let trimmed = trimmed.trim_end();
    let ends_with_marker = trimmed
        .get(trimmed.len().saturating_sub(end_marker.len())..)
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(end_marker));
    starts_with_marker && ends_with_marker
}
