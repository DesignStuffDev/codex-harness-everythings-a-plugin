use super::*;
use codex_protocol::items::ModelInvocationContext;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::sandbox::SandboxType;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::LegacyAppPathString;
use pretty_assertions::assert_eq;
use std::collections::HashMap;

#[derive(Serialize)]
#[serde(transparent)]
struct Borrowed<'a>(#[serde(with = "super")] &'a TimelinePage);

#[derive(Deserialize)]
#[serde(transparent)]
struct Owned(#[serde(with = "super")] TimelinePage);

fn round_trip(items: Vec<api::ThreadItem>) {
    let page = TimelinePage {
        items: items
            .into_iter()
            .enumerate()
            .map(|(position, item)| api::ThreadTimelineEntry::Item {
                position: u64::MAX - u64::try_from(position).unwrap(),
                turn_id: "turn\0boundary".to_owned(),
                item: Box::new(item),
            })
            .collect(),
        next_cursor: Some("opaque:{\"position\":18446744073709551615}".to_owned()),
        active_realtime_session_at_page_start: Some("voice-session".to_owned()),
    };
    let encoded = serde_json::to_string(&Borrowed(&page)).unwrap();
    assert_eq!(serde_json::from_str::<Owned>(&encoded).unwrap().0, page);
    let value = serde_json::to_value(Borrowed(&page)).unwrap();
    assert_eq!(serde_json::from_value::<Owned>(value).unwrap().0, page);
}

#[test]
fn preserves_command_image_analytics_and_custom_effort() {
    round_trip(vec![
        api::ThreadItem::CommandExecution {
            sandbox_type: Some(SandboxType::LinuxSeccomp),
            model_context: Some(ModelInvocationContext {
                model_slug: "selected-model".to_owned(),
                reasoning_effort: Some("custom-effort".to_owned()),
            }),
            id: "command".to_owned(),
            plugin_id: Some("plugin".to_owned()),
            script_path: Some("bin/run".to_owned()),
            command: "printf hello".to_owned(),
            cwd: LegacyAppPathString::from_string(r"C:\foreign\workspace"),
            process_id: Some("process".to_owned()),
            source: api::CommandExecutionSource::Agent,
            status: api::CommandExecutionStatus::Completed,
            command_actions: vec![],
            aggregated_output: Some("hello".to_owned()),
            exit_code: Some(0),
            duration_ms: Some(53),
        },
        api::ThreadItem::ImageGeneration(api::ImageGenerationItem {
            id: "image".to_owned(),
            status: "completed".to_owned(),
            revised_prompt: Some("retained prompt".to_owned()),
            result: "base64".to_owned(),
            transparent_background: Some(true),
            failure: None,
            saved_path: Some(
                AbsolutePathBuf::from_absolute_path(std::env::current_dir().unwrap()).unwrap(),
            ),
            imagegen_request_id: Some("image-request".to_owned()),
            generation_id: Some("generation".to_owned()),
        }),
        api::ThreadItem::CollabAgentToolCall {
            id: "collaboration".to_owned(),
            tool: api::CollabAgentTool::SpawnAgent,
            status: api::CollabAgentToolCallStatus::Completed,
            sender_thread_id: "sender".to_owned(),
            receiver_thread_ids: vec!["receiver".to_owned()],
            prompt: Some("work".to_owned()),
            model: Some("model".to_owned()),
            // Native provider serde would reconstruct High instead of Custom.
            reasoning_effort: Some(ReasoningEffort::Custom("high".to_owned())),
            agents_states: HashMap::new(),
        },
    ]);
}

#[test]
fn preserves_opaque_tool_and_search_json_with_large_numbers() {
    let opaque: serde_json::Value = serde_json::from_str(
        r#"{"type":"commandExecution","sandbox_type":"opaque","model_context":{"model_slug":"user data"},"number":123456789012345678901234567890.1234567890123456789}"#,
    )
    .unwrap();
    round_trip(vec![
        api::ThreadItem::McpToolCall {
            id: "mcp".to_owned(),
            server: "server".to_owned(),
            tool: "tool".to_owned(),
            status: api::McpToolCallStatus::Completed,
            arguments: opaque.clone(),
            app_context: None,
            mcp_app_resource_uri: None,
            mcp_app_ui: None,
            plugin_id: None,
            read_only_hint: Some(true),
            result: Some(Box::new(api::McpToolCallResult {
                content: vec![opaque.clone()],
                structured_content: Some(opaque.clone()),
                meta: Some(opaque.clone()),
            })),
            error: None,
            duration_ms: Some(7),
        },
        api::ThreadItem::DynamicToolCall {
            id: "dynamic".to_owned(),
            namespace: None,
            tool: "tool".to_owned(),
            arguments: opaque.clone(),
            status: api::DynamicToolCallStatus::Completed,
            content_items: None,
            success: Some(true),
            duration_ms: Some(8),
        },
        api::ThreadItem::WebSearch(api::WebSearchItem {
            id: "search".to_owned(),
            query: "query".to_owned(),
            action: None,
            results: Some(vec![opaque]),
        }),
    ]);
}

#[cfg(unix)]
#[test]
fn preserves_non_utf8_input_and_saved_image_paths() {
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    let path = PathBuf::from(std::ffi::OsString::from_vec(
        b"/tmp/component-\xff".to_vec(),
    ));
    round_trip(vec![
        api::ThreadItem::UserMessage {
            id: "user".to_owned(),
            client_id: None,
            content: vec![
                api::UserInput::LocalImage {
                    detail: None,
                    path: path.clone(),
                },
                api::UserInput::LocalAudio { path: path.clone() },
                api::UserInput::Skill {
                    name: "skill".to_owned(),
                    path: path.clone(),
                },
            ],
        },
        api::ThreadItem::FileChange {
            id: "patch".to_owned(),
            changes: vec![api::FileUpdateChange {
                path: "old-path".to_owned(),
                kind: api::PatchChangeKind::Update {
                    move_path: Some(path.clone()),
                },
                diff: "@@ -1 +1 @@\n-old\n+new".to_owned(),
            }],
            status: api::PatchApplyStatus::Completed,
        },
        api::ThreadItem::ImageGeneration(api::ImageGenerationItem {
            id: "image".to_owned(),
            status: "completed".to_owned(),
            revised_prompt: None,
            result: "base64".to_owned(),
            transparent_background: None,
            failure: None,
            saved_path: Some(AbsolutePathBuf::from_absolute_path(path).unwrap()),
            imagegen_request_id: None,
            generation_id: None,
        }),
    ]);
}

#[test]
fn preserves_absent_and_explicit_null_mcp_metadata() {
    let mut items = Vec::new();
    for structured_content in [None, Some(serde_json::Value::Null)] {
        for meta in [None, Some(serde_json::Value::Null)] {
            items.push(api::ThreadItem::McpToolCall {
                id: "mcp".to_owned(),
                server: "server".to_owned(),
                tool: "tool".to_owned(),
                status: api::McpToolCallStatus::Completed,
                arguments: serde_json::Value::Null,
                app_context: None,
                mcp_app_resource_uri: None,
                mcp_app_ui: None,
                plugin_id: None,
                read_only_hint: None,
                result: Some(Box::new(api::McpToolCallResult {
                    content: vec![],
                    structured_content: structured_content.clone(),
                    meta,
                })),
                error: None,
                duration_ms: None,
            });
        }
    }
    round_trip(items);
}
