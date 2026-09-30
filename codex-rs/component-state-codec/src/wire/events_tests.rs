use codex_extension_items::ExtensionItem;
use codex_protocol::items::CommandExecutionItem;
use codex_protocol::items::CommandExecutionStatus;
use codex_protocol::items::ModelInvocationContext;
use codex_protocol::items::TurnItem;
use codex_protocol::models::FileSystemPermissions;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::permissions::FileSystemAccessMode;
use codex_protocol::permissions::FileSystemPath;
use codex_protocol::permissions::FileSystemSandboxEntry;
use codex_protocol::protocol::ErrorEvent;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ExecCommandSource;
use codex_protocol::protocol::HookCompletedEvent;
use codex_protocol::protocol::HookEventName;
use codex_protocol::protocol::HookExecutionMode;
use codex_protocol::protocol::HookHandlerType;
use codex_protocol::protocol::HookRunStatus;
use codex_protocol::protocol::HookRunSummary;
use codex_protocol::protocol::HookScope;
use codex_protocol::protocol::HookSource;
use codex_protocol::protocol::ItemCompletedEvent;
use codex_protocol::protocol::MisalignmentErrorDetails;
use codex_protocol::protocol::MisalignmentSteer;
use codex_protocol::protocol::SessionConfiguredEvent;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::request_permissions::RequestPermissionProfile;
use codex_protocol::request_permissions::RequestPermissionsEvent;
use codex_protocol::sandbox::SandboxType;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::PathUri;

#[derive(serde::Serialize, serde::Deserialize)]
struct Event(#[serde(with = "super::events::EventMsgWire")] EventMsg);

fn roundtrip(event: EventMsg) -> EventMsg {
    let bytes = serde_json::to_vec(&Event(event)).expect("encode trusted event");
    serde_json::from_slice::<Event>(&bytes)
        .expect("decode trusted event")
        .0
}

fn local_path() -> AbsolutePathBuf {
    AbsolutePathBuf::from_absolute_path(std::env::temp_dir()).expect("absolute temporary directory")
}

fn item_completed(item: TurnItem) -> EventMsg {
    EventMsg::ItemCompleted(ItemCompletedEvent {
        thread_id: codex_protocol::ThreadId::new(),
        turn_id: "turn".into(),
        item,
        started_at_ms: Some(2),
        completed_at_ms: 3,
    })
}

#[test]
fn preserves_builtin_hook_marker() {
    let run = HookRunSummary {
        builtin: true,
        id: "hook".into(),
        event_name: HookEventName::PreToolUse,
        handler_type: HookHandlerType::Command,
        execution_mode: HookExecutionMode::Sync,
        scope: HookScope::Turn,
        source_path: local_path(),
        source: HookSource::System,
        display_order: 1,
        status: HookRunStatus::Completed,
        status_message: None,
        started_at: 2,
        completed_at: Some(3),
        duration_ms: Some(1),
        entries: Vec::new(),
    };
    let EventMsg::HookCompleted(actual) = roundtrip(EventMsg::HookCompleted(HookCompletedEvent {
        turn_id: Some("turn".into()),
        run: run.clone(),
    })) else {
        panic!("hook event changed variant")
    };
    assert_eq!(actual.run, run);
}

#[test]
fn preserves_sensitive_error_details_only_on_trusted_transport() {
    let original = ErrorEvent {
        message: "blocked".into(),
        codex_error_info: None,
        misalignment: Some(MisalignmentErrorDetails {
            error_type: Some("policy".into()),
            detailed_explanation: Some("explanation".into()),
            steer: Some(MisalignmentSteer {
                message: "steer".into(),
            }),
        }),
    };
    let public = serde_json::to_value(&original).expect("public event JSON");
    assert!(public.get("misalignment").is_none());
    let EventMsg::Error(actual) = roundtrip(EventMsg::Error(original.clone())) else {
        panic!("error event changed variant")
    };
    assert_eq!(actual, original);
}

#[test]
fn preserves_command_analytics_in_turn_items() {
    let original = CommandExecutionItem {
        sandbox_type: Some(SandboxType::LinuxSeccomp),
        model_context: Some(ModelInvocationContext {
            model_slug: "model".into(),
            reasoning_effort: Some("high".into()),
        }),
        id: "command".into(),
        plugin_id: None,
        script_path: None,
        process_id: Some("process".into()),
        command: vec!["true".into()],
        cwd: PathUri::from(local_path()),
        parsed_cmd: Vec::new(),
        source: ExecCommandSource::Agent,
        interaction_input: None,
        status: CommandExecutionStatus::Completed,
        stdout: Some(String::new()),
        stderr: None,
        aggregated_output: None,
        exit_code: Some(0),
        duration: Some(std::time::Duration::from_nanos(1)),
        formatted_output: None,
    };
    let EventMsg::ItemCompleted(actual) =
        roundtrip(item_completed(TurnItem::CommandExecution(original.clone())))
    else {
        panic!("item event changed variant")
    };
    let TurnItem::CommandExecution(actual) = actual.item else {
        panic!("turn item changed variant")
    };
    assert_eq!(actual, original);
}

#[test]
fn preserves_extension_image_request_and_generation_ids() {
    let original = codex_extension_items::image_generation::ImageGenerationItem {
        id: "image".into(),
        status: "completed".into(),
        revised_prompt: None,
        result: "data".into(),
        transparent_background: None,
        failure: None,
        saved_path: None,
        imagegen_request_id: Some("nested-request".into()),
        generation_id: Some("generation".into()),
    };
    let EventMsg::ItemCompleted(actual) = roundtrip(item_completed(TurnItem::Extension(
        ExtensionItem::ImageGeneration(original.clone()),
    ))) else {
        panic!("item event changed variant")
    };
    let TurnItem::Extension(ExtensionItem::ImageGeneration(actual)) = actual.item else {
        panic!("extension item changed variant")
    };
    assert_eq!(actual, original);
}

#[test]
fn preserves_recursive_events_and_custom_scalar_variants() {
    let thread_id = codex_protocol::ThreadId::new();
    let initial = ErrorEvent {
        message: "nested".into(),
        codex_error_info: None,
        misalignment: Some(MisalignmentErrorDetails {
            error_type: None,
            detailed_explanation: Some("nested explanation".into()),
            steer: None,
        }),
    };
    let EventMsg::SessionConfigured(actual) =
        roundtrip(EventMsg::SessionConfigured(SessionConfiguredEvent {
            session_id: thread_id.into(),
            thread_id,
            forked_from_id: None,
            parent_thread_id: None,
            thread_source: Some(ThreadSource::Feature("user".into())),
            thread_name: None,
            model: "model".into(),
            model_provider_id: "provider".into(),
            service_tier: None,
            approval_policy: codex_protocol::protocol::AskForApproval::Never,
            approvals_reviewer: codex_protocol::config_types::ApprovalsReviewer::default(),
            permission_profile: PermissionProfile::Disabled,
            active_permission_profile: None,
            cwd: local_path(),
            reasoning_effort: Some(ReasoningEffort::Custom("high".into())),
            initial_messages: Some(vec![EventMsg::Error(initial.clone())]),
            network_proxy: None,
            rollout_path: None,
        }))
    else {
        panic!("session event changed variant")
    };
    assert_eq!(
        actual.thread_source,
        Some(ThreadSource::Feature("user".into()))
    );
    assert_eq!(
        actual.reasoning_effort,
        Some(ReasoningEffort::Custom("high".into()))
    );
    let EventMsg::Error(actual) = &actual.initial_messages.expect("initial events")[0] else {
        panic!("nested event changed variant")
    };
    assert_eq!(*actual, initial);
}

#[test]
fn preserves_permission_entry_order_and_missing_path_behavior() {
    let path = FileSystemPath::Path {
        path: PathUri::from(local_path()),
    };
    let original = RequestPermissionProfile {
        network: None,
        file_system: Some(FileSystemPermissions {
            entries: vec![
                FileSystemSandboxEntry::skip_missing_path(
                    path.clone(),
                    FileSystemAccessMode::Write,
                ),
                FileSystemSandboxEntry::new(path, FileSystemAccessMode::Read),
            ],
            glob_scan_max_depth: None,
        }),
    };
    let public = serde_json::to_value(&original).expect("public permissions JSON");
    let public_roundtrip: RequestPermissionProfile =
        serde_json::from_value(public).expect("legacy permissions");
    assert_ne!(public_roundtrip, original);
    let EventMsg::RequestPermissions(actual) =
        roundtrip(EventMsg::RequestPermissions(RequestPermissionsEvent {
            call_id: "request".into(),
            turn_id: "turn".into(),
            environment_id: None,
            started_at_ms: 1,
            reason: None,
            permissions: original.clone(),
            cwd: None,
        }))
    else {
        panic!("request event changed variant")
    };
    assert_eq!(actual.permissions, original);
}

#[test]
fn preserves_arbitrary_precision_elicitation_payloads() {
    let schema: serde_json::Value =
        serde_json::from_str(r#"{"minimum":123456789012345678901234567890.123456789}"#)
            .expect("schema");
    let original = codex_protocol::approvals::ElicitationRequest::Form {
        meta: Some(schema.clone()),
        message: "form".into(),
        requested_schema: schema,
    };
    let EventMsg::ElicitationRequest(actual) = roundtrip(EventMsg::ElicitationRequest(
        codex_protocol::approvals::ElicitationRequestEvent {
            turn_id: None,
            server_name: "server".into(),
            id: codex_protocol::mcp::RequestId::Integer(1),
            request: original.clone(),
        },
    )) else {
        panic!("elicitation event changed variant")
    };
    assert_eq!(actual.request, original);
}

#[test]
fn preserves_nonfinite_float_bits() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Window(
        #[serde(with = "super::events_review::RateLimitWindowWire")]
        codex_protocol::protocol::RateLimitWindow,
    );
    for bits in [
        f64::INFINITY.to_bits(),
        (-0.0_f64).to_bits(),
        0x7ff8000000000042,
    ] {
        let original = Window(codex_protocol::protocol::RateLimitWindow {
            used_percent: f64::from_bits(bits),
            window_minutes: None,
            resets_at: None,
        });
        let wire = serde_json::to_vec(&original).expect("encode float");
        let actual: Window = serde_json::from_slice(&wire).expect("decode float");
        assert_eq!(actual.0.used_percent.to_bits(), bits);
    }
}

#[test]
fn preserves_captured_json_null_in_mcp_results() {
    let original = codex_protocol::protocol::McpToolCallEndEvent {
        call_id: "call".into(),
        turn_id: "turn".into(),
        invocation: codex_protocol::protocol::McpInvocation {
            server: "server".into(),
            tool: "tool".into(),
            arguments: Some(serde_json::Value::Null),
        },
        connector_id: None,
        mcp_app_resource_uri: None,
        mcp_app_ui: None,
        link_id: None,
        app_name: None,
        action_name: None,
        plugin_id: None,
        read_only_hint: None,
        duration: std::time::Duration::from_nanos(7),
        result: Ok(codex_protocol::mcp::CallToolResult {
            content: vec![serde_json::Value::Null],
            structured_content: Some(serde_json::Value::Null),
            is_error: Some(false),
            meta: Some(serde_json::Value::Null),
        }),
    };
    let EventMsg::McpToolCallEnd(actual) = roundtrip(EventMsg::McpToolCallEnd(original.clone()))
    else {
        panic!("MCP event changed variant")
    };
    assert_eq!(actual, original);
}

#[test]
fn preserves_absent_and_captured_null_in_optional_json() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Optional(
        #[serde(with = "super::events_json::optional_value")] Option<serde_json::Value>,
    );
    for original in [
        None,
        Some(serde_json::Value::Null),
        Some(serde_json::json!({"nested": null})),
    ] {
        let wire = serde_json::to_vec(&Optional(original.clone())).expect("encode optional JSON");
        let actual: Optional = serde_json::from_slice(&wire).expect("decode optional JSON");
        assert_eq!(actual.0, original);
    }
}

#[test]
fn preserves_all_custom_effort_collisions_and_empty_value() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Effort(#[serde(with = "super::events_scalars::ReasoningEffortWire")] ReasoningEffort);
    for value in [
        "",
        "none",
        "minimal",
        "low",
        "medium",
        "high",
        "xhigh",
        "max",
        "ultra",
        "persistent",
    ] {
        let original = ReasoningEffort::Custom(value.into());
        let wire = serde_json::to_vec(&Effort(original.clone())).expect("encode custom effort");
        let actual: Effort = serde_json::from_slice(&wire).expect("decode custom effort");
        assert_eq!(actual.0, original);
    }
}

#[test]
fn preserves_all_feature_source_collisions() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Source(#[serde(with = "super::events_scalars::ThreadSourceWire")] ThreadSource);
    for value in [
        "user",
        "subagent",
        "guardian_review",
        "memory_consolidation",
    ] {
        let original = ThreadSource::Feature(value.into());
        let wire = serde_json::to_vec(&Source(original.clone())).expect("encode custom source");
        let actual: Source = serde_json::from_slice(&wire).expect("decode custom source");
        assert_eq!(actual.0, original);
    }
}
