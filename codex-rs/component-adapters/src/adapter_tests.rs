use std::fs;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use codex_component_host::ComponentCatalog;
use codex_context_fragments::ContextualUserFragment;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionRegistry;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::ThreadResumeInput;
use codex_extension_api::ThreadStopInput;
use codex_extension_api::TurnInputContext;
use codex_extension_api::TurnStartPhase;
use codex_extension_api::TurnStopInput;
use codex_protocol::models::ContentItemKind;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use codex_tools::ConversationHistory;
use codex_tools::NoopTurnItemEmitter;
use codex_tools::ToolCall;
use codex_tools::ToolCallSource;
use codex_tools::ToolExecutor;
use codex_tools::ToolName;
use codex_tools::ToolPayload;
use codex_utils_output_truncation::TruncationPolicy;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;

// Cargo builds this std-only plugin outside the workspace; Bazel supplies the same fixture.
// It logs received requests to make the adapter's real process boundary observable.
const PLUGIN: &str = include_str!("../tests/fixtures/component_adapter_plugin.rs");

struct Fixture {
    root: TempDir,
    catalog: ComponentCatalog,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("temporary external plugin directory");
        let package = root.path().join("external-package");
        fs::create_dir_all(&package).expect("create package directory");
        let source = root.path().join("external_plugin.rs");
        fs::write(&source, PLUGIN).expect("write standalone plugin source");
        let executable = format!("plugin{}", std::env::consts::EXE_SUFFIX);
        if option_env!("BAZEL_PACKAGE").is_some() {
            fs::copy(
                codex_utils_cargo_bin::cargo_bin("component-adapter-test-plugin")
                    .expect("resolve Bazel adapter fixture"),
                package.join(&executable),
            )
            .expect("copy standalone adapter fixture");
        } else {
            let output = Command::new("rustc")
                .arg("--edition=2024")
                .arg(&source)
                .arg("-o")
                .arg(package.join(&executable))
                .output()
                .expect("build external plugin using Rust toolchain");
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        fs::write(package.join("codex-component.json"), serde_json::to_vec(&json!({
            "api_version": 1, "id": "fixture", "version": "1.0.0",
            "entrypoint": executable,
            "args": [root.path().join("requests.jsonl"), root.path().join("blocked")],
            "components": [
                {"kind":"tool", "name":"fixture_echo", "contract_version":1,
                 "metadata":{"description":"External fixture tool", "input_schema":{"type":"object"}}},
                {"kind":"context", "name":"fixture_context", "contract_version":1},
                {"kind":"lifecycle", "name":"fixture_observer", "contract_version":1}
            ]
        })).expect("serialize manifest")).expect("write manifest");
        assert_eq!(
            codex_component_host::install(root.path(), &package)
                .expect("install independent package"),
            "fixture"
        );
        fs::remove_dir_all(&package).expect("remove original external package after installation");
        fs::remove_file(&source).expect("remove standalone build source");
        let catalog = ComponentCatalog::load(root.path()).expect("load installed package");
        Self { root, catalog }
    }

    fn registry(&self) -> ExtensionRegistry<()> {
        let mut builder = ExtensionRegistryBuilder::new();
        super::install(&mut builder, &self.catalog).expect("install adapters");
        builder.build()
    }

    fn requests(&self) -> Vec<Value> {
        fs::read_to_string(self.root.path().join("requests.jsonl"))
            .expect("plugin request log")
            .lines()
            .map(|line| serde_json::from_str(line).expect("logged request"))
            .collect()
    }
}

fn call(arguments: Value) -> ToolCall<'static> {
    ToolCall {
        turn_id: "turn-1".to_owned(),
        call_id: "call-1".to_owned(),
        tool_name: ToolName::plain("fixture_echo"),
        model: "fixture-model".to_owned(),
        codex_turn_metadata: None,
        truncation_policy: TruncationPolicy::Bytes(1024),
        source: ToolCallSource::Direct,
        conversation_history: ConversationHistory::default(),
        turn_item_emitter: Arc::new(NoopTurnItemEmitter),
        environments: Vec::new(),
        payload: ToolPayload::Function {
            arguments: arguments.to_string(),
        },
    }
}

fn executor(registry: &ExtensionRegistry<()>) -> Arc<dyn for<'call> ToolExecutor<ToolCall<'call>>> {
    let session = ExtensionData::new("session-1");
    let thread = ExtensionData::new("thread-1");
    registry.tool_contributors()[0]
        .tools(&session, &thread)
        .remove(0)
}

#[tokio::test]
async fn independently_built_tool_runs_through_native_registry_and_preserves_output() {
    let fixture = Fixture::new();
    let registry = fixture.registry();
    let executor = executor(&registry);
    let invocation = call(json!({"message":"hello"}));
    let output = executor
        .handle(invocation.clone())
        .await
        .expect("external tool result");
    assert_eq!(
        output.to_response_item(&invocation.call_id, &invocation.payload),
        ResponseInputItem::FunctionCallOutput {
            call_id: "call-1".to_owned(),
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::Text("native external tool executed".to_owned()),
                success: Some(true),
            },
        }
    );
    assert_eq!(
        fixture.requests()[0]["params"],
        json!({
            "call_id":"call-1", "name":"fixture_echo", "arguments":{"message":"hello"}
        })
    );
    assert!(output.contains_external_context());
    assert!(!executor.replaces_existing_tool());
}

#[tokio::test]
async fn tool_limits_unicode_output_and_reports_invalid_results() {
    let fixture = Fixture::new();
    let executor = executor(&fixture.registry());
    let invocation = call(json!({"large":true}));
    let budget = invocation.response_byte_budget(16_384);
    let result = executor.handle(invocation).await.expect("bounded result");
    assert!(result.log_output().len() <= budget);
    assert!(!result.success_for_logging());
    let error = executor
        .handle(call(json!({"malformed":true})))
        .await
        .err()
        .expect("invalid result rejected");
    assert!(
        error
            .to_string()
            .contains("invalid component tool response")
    );
}

#[tokio::test]
async fn cancelling_native_tool_call_terminates_external_work() {
    let fixture = Fixture::new();
    let executor = executor(&fixture.registry());
    let mut future = executor.handle(call(json!({"block":true})));
    tokio::select! {
        _ = &mut future => panic!("blocking plugin completed unexpectedly"),
        () = async {
            tokio::time::timeout(Duration::from_secs(10), async {
                while !fixture.root.path().join("blocked").exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }).await.expect("plugin entered invocation");
        } => {}
    }
    drop(future);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(!fixture.root.path().join("blocked.finished").exists());
}

#[tokio::test]
async fn context_and_lifecycle_use_bounded_owned_values() {
    let fixture = Fixture::new();
    let registry = fixture.registry();
    let session = ExtensionData::new("session-1");
    let thread = ExtensionData::new("thread-1");
    let turn = ExtensionData::new("turn-1");
    let fragments = registry.turn_input_contributors()[0]
        .contribute(
            TurnInputContext {
                turn_id: "turn-1".to_owned(),
                user_input: Vec::new(),
                environments: Vec::new(),
            },
            /*extension_metrics*/ None,
            &session,
            &thread,
            &turn,
        )
        .await;
    assert_eq!(fragments.len(), 1);
    let fragment = &fragments[0];
    assert_eq!(
        (fragment.role(), fragment.content_kind()),
        ("user", ContentItemKind("components.context".to_owned()))
    );
    assert!(fragment.render().len() <= 1000);
    assert!(codex_context_fragments::ComponentContextFragment::matches_text(&fragment.render()));
    let lifecycle = &registry.thread_lifecycle_contributors()[0];
    lifecycle
        .on_thread_resume(ThreadResumeInput {
            session_store: &session,
            thread_store: &thread,
        })
        .await;
    registry.turn_lifecycle_contributors()[0]
        .on_turn_stop(TurnStopInput {
            session_store: &session,
            thread_store: &thread,
            turn_store: &turn,
        })
        .await;
    lifecycle
        .on_thread_stop(ThreadStopInput {
            session_store: &session,
            thread_store: &thread,
        })
        .await;
    assert_eq!(
        registry.turn_lifecycle_contributors()[0].turn_start_phase(&thread),
        TurnStartPhase::RegularTaskStart
    );
    let requests = fixture.requests();
    assert_eq!(
        requests
            .iter()
            .map(|request| request["params"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!({"session_id":"session-1", "thread_id":"thread-1", "turn_id":"turn-1"}),
            json!({"event":"thread/resume", "session_id":"session-1", "thread_id":"thread-1", "turn_id":null, "details":{}}),
            json!({"event":"turn/stop", "session_id":"session-1", "thread_id":"thread-1", "turn_id":"turn-1", "details":{}}),
            json!({"event":"thread/stop", "session_id":"session-1", "thread_id":"thread-1", "turn_id":null, "details":{}}),
        ]
    );
}

#[test]
fn tool_replacement_requires_explicit_selection() {
    let mut fixture = Fixture::new();
    assert!(!executor(&fixture.registry()).replaces_existing_tool());
    codex_component_host::select(fixture.root.path(), "tool", "fixture_echo", Some("fixture"))
        .expect("select replacement");
    fixture.catalog = ComponentCatalog::load(fixture.root.path()).expect("explicit selection");
    assert!(executor(&fixture.registry()).replaces_existing_tool());
}
