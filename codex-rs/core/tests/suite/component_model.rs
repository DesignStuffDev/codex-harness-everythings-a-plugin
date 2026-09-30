//! External model components drive real Codex turns, tools, recovery and cancellation.
use anyhow::Context;
use anyhow::Result;
use codex_component_host::ComponentCatalog;
use codex_core::StartThreadOptions;
use codex_core::TurnInputRequest;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::IsolatedSessionExtensions;
use codex_extension_api::ModelRequestContributor;
use codex_extension_api::ModelRequestInput;
use codex_extension_api::ModelResponseInterceptor;
use codex_extension_api::ModelResponseStream;
use codex_extension_api::SessionIsolation;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::test_codex::TestCodex;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use futures::StreamExt;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

const PLUGIN_ID: &str = "test.model-transport";

// Built into a standalone executable zipapp in a temporary directory. This
// fixture has no Cargo dependency on the harness or access to its source tree.
const PLUGIN: &str = r#"
import json, os, pathlib, sys, time
def send(value):
    print(json.dumps(value), flush=True)
init = json.loads(sys.stdin.readline())
send({'type': 'ready', 'api_version': 1})
call = json.loads(sys.stdin.readline())
assert call['method'] == 'model.stream'
state = pathlib.Path(init['state_dir'])
index = len(list(state.glob('request-*.json')))
(state / ('request-%d.json' % index)).write_text(json.dumps(call['params']))
def event(payload):
    send({'type': 'event', 'id': call['id'], 'event': payload})
def complete():
    event({'type': 'completed', 'response_id': 'component-response-%d' % index, 'end_turn': True})
    send({'type': 'result', 'id': call['id'], 'result': {}})
    assert json.loads(sys.stdin.readline())['type'] == 'shutdown'
mode = init['config'].get('mode', 'tool')
event({'type': 'created', 'response_id': 'component-response-%d' % index})
item = {'type': 'message', 'id': 'msg_component_%d' % index, 'role': 'assistant',
        'content': [], 'phase': 'final_answer'}
if mode == 'invalid' and index == 0:
    event({'type': 'unsupported-model-event'})
    time.sleep(60)
elif mode == 'block' and index == 0:
    (state / 'blocked-pid').write_text(str(os.getpid()))
    event({'type': 'output_item_added', 'item': item})
    event({'type': 'output_text_delta', 'delta': 'waiting for cancellation'})
    time.sleep(60)
elif mode == 'tool' and index == 0:
    arguments = json.dumps({'plan': [{'step': 'component plan', 'status': 'completed'}]})
    tool = {'type': 'function_call', 'id': 'fc_component', 'call_id': 'component-plan',
            'name': 'update_plan', 'arguments': ''}
    event({'type': 'output_item_added', 'item': tool})
    event({'type': 'tool_call_input_delta', 'item_id': 'fc_component',
           'call_id': 'component-plan', 'delta': arguments})
    tool['arguments'] = arguments
    event({'type': 'output_item_done', 'item': tool})
    complete()
else:
    event({'type': 'output_item_added', 'item': item})
    event({'type': 'output_text_delta', 'delta': 'component reply'})
    item['content'] = [{'type': 'output_text', 'text': 'component reply'}]
    event({'type': 'output_item_done', 'item': item})
    complete()
"#;

fn install_fixture(codex_home: &Path, mode: &str) -> Result<()> {
    let source = tempfile::tempdir()?;
    let package = tempfile::tempdir()?;
    std::fs::write(source.path().join("__main__.py"), PLUGIN)?;
    let interpreter = if cfg!(windows) { "python" } else { "python3" };
    let output = Command::new(interpreter)
        .args(["-m", "zipapp"])
        .arg(source.path())
        .args(["--python", "/usr/bin/env python3", "--output"])
        .arg(package.path().join("plugin.pyz"))
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "build standalone model fixture: {output:?}"
    );
    std::fs::write(
        package.path().join("codex-component.json"),
        serde_json::to_vec(&json!({
            "api_version": 1, "id": PLUGIN_ID, "version": "0.1.0",
            "entrypoint": "plugin.pyz", "components": [{
                "kind": "model_transport", "name": "default", "contract_version": 1
            }]
        }))?,
    )?;
    codex_component_host::install(codex_home, package.path())?;
    let settings_path = codex_home.join("components/config.json");
    let mut settings: Value = serde_json::from_slice(&std::fs::read(&settings_path)?)?;
    settings["config"][PLUGIN_ID] = json!({"mode": mode});
    settings["selections"]["model_transport:default"] = json!(PLUGIN_ID);
    std::fs::write(settings_path, serde_json::to_vec(&settings)?)?;
    Ok(())
}

fn state_dir(test: &TestCodex) -> Result<PathBuf> {
    Ok(ComponentCatalog::load(test.codex_home_path())?
        .selected("model_transport", "default")
        .context("selected fixture")?
        .state_dir)
}

async fn submit(test: &TestCodex, text: &str) -> Result<()> {
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: text.into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    Ok(())
}

async fn assert_no_native_inference(server: &wiremock::MockServer) {
    let inference_requests: Vec<_> = server
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|request| request.url.path().ends_with("/responses"))
        .map(|request| request.url.to_string())
        .collect();
    assert_eq!(inference_requests, Vec::<String>::new());
}

#[derive(Debug)]
struct Observer(Arc<AtomicUsize>);

impl ModelRequestContributor for Observer {
    fn request(&self, input: ModelRequestInput<'_>) -> Option<Box<dyn ModelResponseInterceptor>> {
        input
            .client_metadata
            .get_or_insert_default()
            .insert("component-observer".into(), "active".into());
        Some(Box::new(Observer(Arc::clone(&self.0))))
    }
}

impl ModelResponseInterceptor for Observer {
    fn intercept(self: Box<Self>, stream: ModelResponseStream) -> ModelResponseStream {
        Box::pin(stream.map(move |event| {
            self.0.fetch_add(1, Ordering::SeqCst);
            event
        }))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_model_streams_executes_tools_and_resumes() -> Result<()> {
    let server = responses::start_mock_server().await;
    let seen = Arc::new(AtomicUsize::new(0));
    let mut registry = ExtensionRegistryBuilder::new();
    registry.model_request_contributor(Arc::new(Observer(Arc::clone(&seen))));
    let mut builder = test_codex()
        .with_extensions(Arc::new(registry.build()))
        .with_config(|config| {
            install_fixture(&config.codex_home, "tool").expect("install fixture");
            config.model_provider.supports_websockets = true;
            config.update_plan_enabled = true;
        });
    let test = builder.build_with_auto_env(&server).await?;
    submit(&test, "exercise the installed model and native plan tool").await?;
    let plan = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::PlanUpdate(_) | EventMsg::Error(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;
    assert!(
        matches!(plan, EventMsg::PlanUpdate(_)),
        "expected native plan execution: {plan:?}"
    );
    assert_eq!(
        serde_json::to_value(plan)?["plan"],
        json!([{
            "step": "component plan", "status": "completed"
        }])
    );
    let delta = wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::AgentMessageContentDelta(_))
    })
    .await;
    assert_eq!(
        serde_json::to_value(delta)?["delta"],
        json!("component reply")
    );
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    let request: Value =
        serde_json::from_slice(&std::fs::read(state_dir(&test)?.join("request-1.json"))?)?;
    assert!(
        request["request"]["input"]
            .as_array()
            .context("input")?
            .iter()
            .any(|item| {
                item["type"] == "function_call_output" && item["call_id"] == "component-plan"
            })
    );
    assert_eq!(
        request["request"]["client_metadata"]["component-observer"],
        json!("active")
    );
    assert!(seen.load(Ordering::SeqCst) >= 8);
    assert_no_native_inference(&server).await;

    let resumed = test_codex().restart(&server, &test).await?;
    submit(&resumed, "continue after restart").await?;
    wait_for_event(&resumed.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    let resumed_request: Value =
        serde_json::from_slice(&std::fs::read(state_dir(&resumed)?.join("request-2.json"))?)?;
    assert_eq!(resumed_request["thread_id"], request["thread_id"]);
    assert!(
        resumed_request["request"]["input"]
            .as_array()
            .context("resumed input")?
            .iter()
            .any(|item| {
                item["type"] == "function_call_output" && item["call_id"] == "component-plan"
            })
    );
    assert_no_native_inference(&server).await;

    // Explicit removal of the replacement takes effect at the next startup.
    // The original transport must still operate on the same persisted thread.
    codex_component_host::select(
        resumed.codex_home_path(),
        "model_transport",
        "default",
        /*plugin_id*/ None,
    )?;
    let native = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("native-restored"),
            responses::ev_assistant_message("msg-native", "native transport restored"),
            responses::ev_completed("native-restored"),
        ]),
    )
    .await;
    let restored = test_codex().restart(&server, &resumed).await?;
    submit(&restored, "continue with native transport").await?;
    wait_for_event(&restored.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert!(native.single_request().input().iter().any(|item| {
        item["type"] == "message"
            && item["content"]
                .as_array()
                .is_some_and(|content| content.iter().any(|part| part["text"] == "component reply"))
    }));
    restored.codex.shutdown_and_wait().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn component_failure_does_not_fall_back_and_next_turn_runs() -> Result<()> {
    let server = responses::start_mock_server().await;
    let test = test_codex()
        .with_config(|config| {
            install_fixture(&config.codex_home, "invalid").expect("install fixture")
        })
        .build_with_auto_env(&server)
        .await?;
    submit(&test, "reject malformed model events").await?;
    let error = wait_for_event(&test.codex, |event| matches!(event, EventMsg::Error(_))).await;
    assert!(serde_json::to_string(&error)?.contains("unsupported event type"));
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    submit(&test, "recover on the next turn").await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::AgentMessageContentDelta(_))
    })
    .await;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert_no_native_inference(&server).await;
    test.codex.shutdown_and_wait().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interruption_cancels_component_and_allows_next_turn() -> Result<()> {
    let server = responses::start_mock_server().await;
    let test = test_codex()
        .with_config(|config| {
            install_fixture(&config.codex_home, "block").expect("install fixture")
        })
        .build_with_auto_env(&server)
        .await?;
    submit(&test, "interrupt a streaming component").await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::AgentMessageContentDelta(_))
    })
    .await;
    test.codex.submit(Op::Interrupt).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnAborted(_))
    })
    .await;
    #[cfg(unix)]
    {
        let pid = std::fs::read_to_string(state_dir(&test)?.join("blocked-pid"))?;
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let status = Command::new("python3")
                    .args([
                        "-c",
                        "import os,sys; os.kill(int(sys.argv[1]),0)",
                        pid.trim(),
                    ])
                    .output()
                    .expect("check component process")
                    .status;
                if !status.success() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        })
        .await
        .context("component process survived cancellation")?;
    }
    submit(&test, "new turn after cancellation").await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert_no_native_inference(&server).await;
    test.codex.shutdown_and_wait().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn isolated_session_excludes_installed_model_but_keeps_explicit_extensions() -> Result<()> {
    let server = responses::start_mock_server().await;
    let fixture = test_codex().build_with_auto_env(&server).await?;
    install_fixture(fixture.codex_home_path(), "invalid")?;
    let component_state = state_dir(&fixture)?;
    let seen = Arc::new(AtomicUsize::new(0));
    let mut registry = ExtensionRegistryBuilder::new();
    registry.model_request_contributor(Arc::new(Observer(Arc::clone(&seen))));
    let mut options = StartThreadOptions::new(fixture.config.clone());
    options.environments = Some(fixture.codex.environment_selections().await);
    options
        .thread_extension_init
        .insert(SessionIsolation::Isolated);
    options
        .thread_extension_init
        .insert(IsolatedSessionExtensions::<codex_core::config::Config>(
            Arc::new(registry.build()),
        ));
    let isolated = fixture.thread_manager.start_thread(options).await?;
    let native = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("isolated-native"),
            responses::ev_assistant_message("msg-isolated", "isolated native response"),
            responses::ev_completed("isolated-native"),
        ]),
    )
    .await;
    isolated
        .thread
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "isolated inference".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&isolated.thread, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert_eq!(
        native.single_request().body_json()["client_metadata"]["component-observer"],
        json!("active")
    );
    assert!(seen.load(Ordering::SeqCst) >= 3);
    assert!(
        !component_state.try_exists()?,
        "global component must not be started"
    );
    isolated.thread.shutdown_and_wait().await?;
    fixture.codex.shutdown_and_wait().await?;
    Ok(())
}
