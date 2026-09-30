//! Reachable path-bearing records must use the trusted codec all the way down.

use codex_protocol::items::TurnItem;
use codex_protocol::items::UserMessageItem;
use codex_protocol::parse_command::ParsedCommand;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::FileChange;
use codex_protocol::protocol::UserMessageEvent;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
struct Event(#[serde(with = "super::events::EventMsgWire")] EventMsg);

#[derive(serde::Serialize, serde::Deserialize)]
struct Item(#[serde(with = "super::events_items::TurnItemWire")] TurnItem);

#[derive(serde::Serialize, serde::Deserialize)]
struct Changes(
    #[serde(with = "super::events_file_changes::file_changes")] HashMap<PathBuf, FileChange>,
);

#[cfg(unix)]
fn native_path(suffix: u8) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    let mut bytes = b"/tmp/trusted-replay-".to_vec();
    bytes.push(suffix);
    PathBuf::from(std::ffi::OsString::from_vec(bytes))
}

#[cfg(windows)]
fn native_path(suffix: u8) -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    let mut units = r"C:\trusted-replay-".encode_utf16().collect::<Vec<_>>();
    units.push(0xd800 + u16::from(suffix));
    PathBuf::from(std::ffi::OsString::from_wide(&units))
}

#[test]
fn preserves_all_local_user_input_path_variants_inside_turn_items() {
    let original = TurnItem::UserMessage(UserMessageItem {
        id: "message".to_owned(),
        client_id: Some("client".to_owned()),
        content: vec![
            UserInput::LocalImage {
                path: native_path(0xff),
                detail: None,
            },
            UserInput::LocalAudio {
                path: native_path(0xfe),
            },
            UserInput::Skill {
                name: "skill".to_owned(),
                path: native_path(0xfd),
            },
            UserInput::Mention {
                name: "string reference".to_owned(),
                path: "plugin://plugin".to_owned(),
            },
        ],
    });
    assert!(serde_json::to_vec(&original).is_err());
    let bytes = serde_json::to_vec(&Item(original.clone())).expect("trusted item encoding");
    let decoded: Item = serde_json::from_slice(&bytes).expect("trusted item decoding");
    let (TurnItem::UserMessage(actual), TurnItem::UserMessage(expected)) = (decoded.0, original)
    else {
        panic!("expected user message")
    };
    assert_eq!(
        (actual.id, actual.client_id, actual.content),
        (expected.id, expected.client_id, expected.content),
    );
}

#[test]
fn preserves_local_media_vectors_inside_legacy_user_events() {
    let original = UserMessageEvent {
        message: "media".to_owned(),
        local_images: vec![native_path(0xff), native_path(0xfe)],
        local_audio: vec![native_path(0xfd)],
        ..Default::default()
    };
    let bytes = serde_json::to_vec(&Event(EventMsg::UserMessage(original.clone())))
        .expect("trusted event encoding");
    let Event(EventMsg::UserMessage(decoded)) =
        serde_json::from_slice(&bytes).expect("trusted event decoding")
    else {
        panic!("expected user event")
    };
    assert_eq!(decoded, original);
}

#[test]
fn preserves_distinct_non_unicode_file_keys_and_move_targets() {
    let changes = HashMap::from([
        (
            native_path(0xff),
            FileChange::Update {
                unified_diff: "@@ native change @@".to_owned(),
                move_path: Some(native_path(0xfd)),
            },
        ),
        (
            native_path(0xfe),
            FileChange::Delete {
                content: "second file".to_owned(),
            },
        ),
    ]);
    // A lossy path-string encoding would collapse these two distinct keys.
    assert_eq!(
        native_path(0xff).to_string_lossy(),
        native_path(0xfe).to_string_lossy()
    );
    let encoded = serde_json::to_vec(&Changes(changes.clone())).expect("trusted path-key map");
    let decoded: Changes = serde_json::from_slice(&encoded).expect("trusted path-key decode");
    assert_eq!(decoded.0, changes);
}

#[test]
fn duplicate_file_change_keys_are_rejected_instead_of_overwriting_evidence() {
    let original = Changes(HashMap::from([(
        native_path(0xff),
        FileChange::Add {
            content: "original".to_owned(),
        },
    )]));
    let mut value = serde_json::to_value(original).expect("encoded entry array");
    let entries = value.as_array_mut().expect("entry array");
    entries.push(entries[0].clone());
    let error = serde_json::from_value::<Changes>(value)
        .err()
        .expect("duplicate rejected");
    assert!(
        error
            .to_string()
            .contains("duplicate trusted file change path")
    );
}

#[test]
fn approval_patch_map_and_grant_root_roundtrip_together() {
    let original = codex_protocol::approvals::ApplyPatchApprovalRequestEvent {
        call_id: "call".to_owned(),
        turn_id: "turn".to_owned(),
        started_at_ms: 123,
        changes: HashMap::from([(
            native_path(0xff),
            FileChange::Add {
                content: "new file".to_owned(),
            },
        )]),
        reason: Some("authorized fixture".to_owned()),
        grant_root: Some(native_path(0xfe)),
    };
    let bytes = serde_json::to_vec(&Event(EventMsg::ApplyPatchApprovalRequest(
        original.clone(),
    )))
    .expect("approval encode");
    let Event(EventMsg::ApplyPatchApprovalRequest(decoded)) =
        serde_json::from_slice(&bytes).expect("approval decode")
    else {
        panic!("expected patch approval")
    };
    assert_eq!(
        (
            decoded.call_id,
            decoded.turn_id,
            decoded.started_at_ms,
            decoded.changes,
            decoded.reason,
            decoded.grant_root
        ),
        (
            original.call_id,
            original.turn_id,
            original.started_at_ms,
            original.changes,
            original.reason,
            original.grant_root
        ),
    );
}

#[test]
fn nested_command_paths_preserve_native_bytes_and_foreign_uri_cwd() {
    let original = codex_protocol::protocol::ExecCommandBeginEvent {
        call_id: "call".to_owned(),
        plugin_id: None,
        script_path: None,
        process_id: None,
        turn_id: "turn".to_owned(),
        started_at_ms: 17,
        command: vec!["fixture".to_owned()],
        cwd: codex_utils_path_uri::PathUri::parse("file:///D:/remote/%FF").expect("URI"),
        parsed_cmd: vec![ParsedCommand::Read {
            cmd: "fixture".to_owned(),
            name: "read".to_owned(),
            path: native_path(0xff),
        }],
        source: codex_protocol::protocol::ExecCommandSource::Agent,
        interaction_input: None,
    };
    let bytes = serde_json::to_vec(&Event(EventMsg::ExecCommandBegin(original.clone())))
        .expect("exec event encode");
    let Event(EventMsg::ExecCommandBegin(decoded)) =
        serde_json::from_slice(&bytes).expect("exec event decode")
    else {
        panic!("expected exec event")
    };
    assert_eq!(decoded.parsed_cmd, original.parsed_cmd);
    assert_eq!(decoded.cwd.to_string(), original.cwd.to_string());
}

#[test]
fn image_generation_item_preserves_absolute_non_unicode_saved_path() {
    let original = TurnItem::ImageGeneration(codex_protocol::items::ImageGenerationItem {
        id: "image".to_owned(),
        status: "completed".to_owned(),
        revised_prompt: None,
        result: "image-result".to_owned(),
        saved_path: Some(
            AbsolutePathBuf::from_absolute_path_checked(native_path(0xff)).expect("absolute path"),
        ),
    });
    let bytes = serde_json::to_vec(&Item(original.clone())).expect("image item encode");
    let decoded: Item = serde_json::from_slice(&bytes).expect("image item decode");
    let (TurnItem::ImageGeneration(actual), TurnItem::ImageGeneration(expected)) =
        (decoded.0, original)
    else {
        panic!("expected image generation")
    };
    assert_eq!(actual, expected);
}

#[test]
fn review_finding_preserves_absolute_native_path() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Finding(
        #[serde(with = "super::events_review::ReviewFindingWire")]
        codex_protocol::protocol::ReviewFinding,
    );
    let original = codex_protocol::protocol::ReviewFinding {
        title: "finding".to_owned(),
        body: "description".to_owned(),
        confidence_score: 0.5,
        priority: 2,
        code_location: codex_protocol::protocol::ReviewCodeLocation {
            absolute_file_path: native_path(0xff),
            line_range: codex_protocol::protocol::ReviewLineRange { start: 1, end: 2 },
        },
    };
    let bytes = serde_json::to_vec(&Finding(original.clone())).expect("finding encode");
    let decoded: Finding = serde_json::from_slice(&bytes).expect("finding decode");
    assert_eq!(decoded.0, original);
}
