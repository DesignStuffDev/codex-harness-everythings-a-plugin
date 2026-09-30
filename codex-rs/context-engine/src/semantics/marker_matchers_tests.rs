use super::is_contextual_user_fragment;
use super::is_model_switch_text;
use super::is_persistent_mode_text;
use codex_protocol::models::ContentItem;
use pretty_assertions::assert_eq;

#[test]
fn default_fragment_matchers_retain_case_and_whitespace_compatibility() {
    for (start, end) in [
        ("# AGENTS.md instructions", "</INSTRUCTIONS>"),
        ("<environment_context>", "</environment_context>"),
        (
            "<agent_message_board_notification>",
            "</agent_message_board_notification>",
        ),
        ("<skill>", "</skill>"),
        ("<user_shell_command>", "</user_shell_command>"),
        ("<turn_aborted>", "</turn_aborted>"),
        ("<subagent_notification>", "</subagent_notification>"),
        ("<recommended_plugins>", "</recommended_plugins>"),
        ("<component_context>\n", "\n</component_context>"),
    ] {
        let text = format!("  {start}body{end}  ");
        for text in [text.clone(), text.to_ascii_uppercase()] {
            assert!(
                is_contextual_user_fragment(&ContentItem::InputText { text: text.clone() }),
                "fragment: {text}",
            );
        }
    }
}

#[test]
fn custom_fragment_matchers_remain_case_sensitive() {
    for text in [
        "<external_project>body</external_project>",
        "<codex_internal_context source=\"extension\">body</codex_internal_context>",
        "<goal_context>body</goal_context>",
        "Warning: apply_patch was requested via shell. Use the apply_patch tool instead of exec_command.",
        "Warning: Your account was flagged for potentially high-risk cyber activity. Details.",
        "Warning: The maximum number of unified exec processes you can keep open is 64.",
    ] {
        assert_eq!(
            (
                is_contextual_user_fragment(&ContentItem::InputText {
                    text: text.to_owned(),
                }),
                is_contextual_user_fragment(&ContentItem::InputText {
                    text: text.to_ascii_uppercase(),
                }),
            ),
            (true, false),
            "fragment: {text}",
        );
    }
}

#[test]
fn developer_retention_matchers_require_complete_wrappers() {
    for (matches_text, start, end) in [
        (
            is_model_switch_text as fn(&str) -> bool,
            "<model_switch>",
            "</model_switch>",
        ),
        (
            is_persistent_mode_text as fn(&str) -> bool,
            "<persistent_mode>",
            "</persistent_mode>",
        ),
    ] {
        assert_eq!(
            (
                matches_text(&format!(" {start}body{end} ")),
                matches_text(&format!(" {start}body{end} ").to_ascii_uppercase()),
                matches_text(&format!("{start}body")),
                matches_text(&format!("body{end}")),
                matches_text("🦀 unrelated context"),
            ),
            (true, true, false, false, false),
        );
    }
}
