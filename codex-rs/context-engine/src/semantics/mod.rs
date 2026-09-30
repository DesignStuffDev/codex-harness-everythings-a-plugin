//! Native context classification, authorization, and protocol event mapping.
//!
//! Matching persisted text preserves visibility compatibility. User authorization is
//! established separately from host annotations, never from matching a text marker.

mod contextual_user_message;
mod event_mapping;
mod guardian_retained_instructions;
mod internal_model_context;
mod legacy_apply_patch_exec_command_warning;
mod legacy_model_mismatch_warning;
mod legacy_unified_exec_process_limit_warning;
mod marker_matchers;
mod subagent_notification;
mod turn_aborted;
mod user_goal;
mod user_instructions;
mod user_shell_command;
mod web_search;

pub use codex_context_fragments::AdditionalContextUserFragment;
pub use codex_context_fragments::ComponentContextFragment;
pub use codex_context_fragments::ContextualUserFragment;
pub use contextual_user_message::is_contextual_user_fragment;
pub use contextual_user_message::is_guardian_context_message;
pub use contextual_user_message::is_user_authorization_message;
pub use contextual_user_message::parse_visible_hook_prompt_message;
pub use event_mapping::has_non_contextual_dev_message_content;
pub use event_mapping::is_contextual_dev_message_content;
pub use event_mapping::is_contextual_user_message_content;
pub use event_mapping::parse_turn_item;
pub use guardian_retained_instructions::GuardianRetainedInstructions;
pub use internal_model_context::InternalContextSource;
pub use internal_model_context::InternalModelContextFragment;
pub use internal_model_context::InvalidInternalContextSource;
pub use marker_matchers::is_model_switch_text;
pub use marker_matchers::is_persistent_mode_text;
pub use user_goal::UserGoalUpdate;

pub(crate) use legacy_apply_patch_exec_command_warning::LegacyApplyPatchExecCommandWarning;
pub(crate) use legacy_model_mismatch_warning::LegacyModelMismatchWarning;
pub(crate) use legacy_unified_exec_process_limit_warning::LegacyUnifiedExecProcessLimitWarning;
pub(crate) use subagent_notification::SubagentNotification;
pub(crate) use turn_aborted::TurnAborted;
pub(crate) use user_instructions::UserInstructions;
pub(crate) use user_shell_command::UserShellCommand;

/// Host classification for a user goal whose oversized objective was omitted whole.
pub const OMITTED_OBJECTIVE_KIND: &str = UserGoalUpdate::OMITTED_OBJECTIVE_KIND;

pub(crate) const APPROVED_COMMAND_PREFIX_SAVED_MESSAGE_PREFIX: &str =
    "Approved command prefix saved:";

#[cfg(test)]
#[path = "authorization_tests.rs"]
mod authorization_tests;

#[cfg(test)]
#[path = "marker_matchers_tests.rs"]
mod marker_matchers_tests;
