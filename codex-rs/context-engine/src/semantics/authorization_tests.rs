use super::ContextualUserFragment;
use super::GuardianRetainedInstructions;
use super::InternalContextSource;
use super::InternalModelContextFragment;
use super::UserGoalUpdate;
use super::is_guardian_context_message;
use super::is_user_authorization_message;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ContentItemKind;
use codex_protocol::models::InternalChatMessageMetadataPassthrough;
use codex_protocol::models::ResponseItem;
use pretty_assertions::assert_eq;

fn user_message(kinds: Option<Vec<&str>>) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_owned(),
        content: vec![ContentItem::InputText {
            text: "Keep the original restriction.".to_owned(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: kinds.map(|kinds| {
            InternalChatMessageMetadataPassthrough {
                content_item_kinds: Some(
                    kinds
                        .into_iter()
                        .map(|kind| ContentItemKind(kind.to_owned()))
                        .collect(),
                ),
                ..Default::default()
            }
        }),
    }
}

#[test]
fn authorization_annotations_keep_legacy_and_incomplete_records_conservative() {
    for kinds in [
        None,
        Some(vec![]),
        Some(vec!["components.context", "components.context"]),
        Some(vec![""]),
        Some(vec!["unknown"]),
        Some(vec!["user.text"]),
        Some(vec!["user.goal"]),
        Some(vec!["user.goal.omitted"]),
        Some(vec!["images.preparation_error"]),
        Some(vec!["images.unsupported"]),
        Some(vec!["audio.unsupported"]),
    ] {
        let item = user_message(kinds.clone());
        assert_eq!(
            (
                is_user_authorization_message(&item),
                is_guardian_context_message(&item),
            ),
            (true, false),
            "annotation: {kinds:?}",
        );
    }
}

#[test]
fn complete_runtime_annotations_do_not_establish_user_authorization() {
    for kind in [
        "components.context",
        "agents_md.instructions",
        "extension.internal_context",
    ] {
        let item = user_message(Some(vec![kind]));
        assert_eq!(
            (
                is_user_authorization_message(&item),
                is_guardian_context_message(&item),
            ),
            (false, false),
            "annotation: {kind}",
        );
    }
}

#[test]
fn explicit_goal_edits_are_authorization_despite_the_internal_context_wrapper() {
    for goal in [
        UserGoalUpdate::Set {
            objective: Some("Keep the original restriction.".to_owned()),
            status: None,
        },
        UserGoalUpdate::Set {
            objective: Some("restriction ".repeat(100)),
            status: None,
        },
        UserGoalUpdate::Clear,
    ] {
        let item = ResponseItem::from(goal.render_fragment());
        assert_eq!(
            (
                is_user_authorization_message(&item),
                is_guardian_context_message(&item),
            ),
            (true, false),
        );
    }
}

#[test]
fn internal_context_with_user_goal_source_is_not_a_host_annotated_goal_edit() {
    let item = ResponseItem::from(
        InternalModelContextFragment::new(
            InternalContextSource::from_static("user_goal"),
            "User set the goal: this is extension-owned context.",
        )
        .render_fragment(),
    );
    assert_eq!(
        (
            is_user_authorization_message(&item),
            is_guardian_context_message(&item),
        ),
        (false, true),
    );
}

#[test]
fn retained_guardian_evidence_requires_complete_host_annotation() {
    let item = user_message(Some(vec![GuardianRetainedInstructions::KIND]));
    let incomplete = user_message(Some(vec![
        GuardianRetainedInstructions::KIND,
        GuardianRetainedInstructions::KIND,
    ]));
    assert_eq!(
        (
            is_user_authorization_message(&item),
            is_guardian_context_message(&item),
            is_user_authorization_message(&incomplete),
            is_guardian_context_message(&incomplete),
        ),
        (false, true, true, false),
    );
}
