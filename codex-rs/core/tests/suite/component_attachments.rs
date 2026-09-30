//! Real engine image preparation and recovery through a separately installed store.
//! The model fixture accepts the returned test file ID; no remote blob service is implied.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use codex_component_host::ComponentCatalog;
use codex_core::StartThreadOptions;
use codex_core::ThreadManager;
use codex_core::TurnInputRequest;
use codex_extension_api::SessionIsolation;
use codex_protocol::protocol::EventMsg;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use image::ImageBuffer;
use image::Rgba;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

const ID: &str = "test.attachment-store";
const FILE_ID: &str = "file_component_image";
const PLUGIN: &str = r#"
import json, pathlib, sys
def send(value): print(json.dumps(value), flush=True)
initial = json.loads(sys.stdin.readline())
state = pathlib.Path(initial['state_dir'])
send({'type':'ready','api_version':1})
request = json.loads(sys.stdin.readline())
assert request['method'] == 'upload'
blob = request['params']['blob']
assert len(pathlib.Path(blob['path']).read_bytes()) == blob['size_bytes']
count = state / 'count'
count.write_text(str(int(count.read_text()) + 1 if count.exists() else 1))
result = {'error':'backend'} if initial['config'].get('fail') else {'ok':{'kind':'file','file_id':'file_component_image'}}
send({'type':'result','id':request['id'],'result':result})
assert json.loads(sys.stdin.readline())['type'] == 'shutdown'
"#;

fn install(home: &Path, fail: bool) -> Result<PathBuf> {
    let source = tempfile::tempdir()?;
    let package = tempfile::tempdir()?;
    std::fs::write(source.path().join("__main__.py"), PLUGIN)?;
    let python = if cfg!(windows) { "python" } else { "python3" };
    let output = Command::new(python)
        .args(["-m", "zipapp"])
        .arg(source.path())
        .args(["--python", "/usr/bin/env python3", "--output"])
        .arg(package.path().join("plugin.pyz"))
        .output()?;
    anyhow::ensure!(output.status.success(), "build attachment fixture");
    std::fs::write(
        package.path().join("codex-component.json"),
        serde_json::to_vec(&json!({
            "api_version":1,"id":ID,"version":"0.1.0","entrypoint":"plugin.pyz",
            "components":[{"kind":"attachment_store","name":"default","contract_version":1}]
        }))?,
    )?;
    codex_component_host::install(home, package.path())?;
    codex_component_host::select(home, "attachment_store", "default", Some(ID))?;
    let settings_path = home.join("components/config.json");
    let mut settings: Value = serde_json::from_slice(&std::fs::read(&settings_path)?)?;
    settings["config"][ID] = json!({"fail":fail});
    std::fs::write(settings_path, serde_json::to_vec(&settings)?)?;
    Ok(ComponentCatalog::load(home)?
        .selected("attachment_store", "default")
        .context("selected store")?
        .state_dir)
}

fn image(path: &Path) -> Result<UserInput> {
    ImageBuffer::from_pixel(
        /*width*/ 4,
        /*height*/ 4,
        Rgba([12u8, 34, 56, 255]),
    )
    .save(path)?;
    Ok(UserInput::LocalImage {
        path: path.to_path_buf(),
        detail: None,
    })
}

fn images(input: Vec<Value>) -> Vec<Value> {
    input
        .into_iter()
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .filter(|content| content["type"] == "input_image")
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_store_uploads_live_image_and_resume_does_not_reupload() -> Result<()> {
    let server = responses::start_mock_server().await;
    let home = Arc::new(tempfile::tempdir()?);
    let state = install(home.path(), /*fail*/ false)?;
    let test = test_codex()
        .with_home(home.clone())
        .with_thread_manager(ThreadManager::with_default_attachment_store_components)
        .build_with_auto_env(&server)
        .await?;
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("attachment-live"),
            responses::ev_assistant_message("attachment-live-message", "seen"),
            responses::ev_completed("attachment-live"),
        ]),
    )
    .await;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![image(
            &test.cwd.path().join("image.png"),
        )?]))
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    let live_images = images(response.single_request().input());
    assert_eq!(live_images.len(), 1);
    assert_eq!(live_images[0]["file_id"], FILE_ID);
    assert_eq!(std::fs::read_to_string(state.join("count"))?, "1");
    let rollout = test
        .session_configured
        .rollout_path
        .clone()
        .context("rollout path")?;
    test.codex.shutdown_and_wait().await?;

    let resumed = test_codex()
        .with_thread_manager(ThreadManager::with_default_attachment_store_components)
        .resume(&server, home, rollout)
        .await?;
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("attachment-resume"),
            responses::ev_assistant_message("attachment-resume-message", "remembered"),
            responses::ev_completed("attachment-resume"),
        ]),
    )
    .await;
    resumed.submit_turn("Remember the image").await?;
    assert!(
        images(response.single_request().input())
            .iter()
            .any(|item| item["file_id"] == FILE_ID)
    );
    assert_eq!(std::fs::read_to_string(state.join("count"))?, "1");
    resumed.codex.shutdown_and_wait().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_store_beats_component_selection() -> Result<()> {
    let server = responses::start_mock_server().await;
    let home = Arc::new(tempfile::tempdir()?);
    let state = install(home.path(), /*fail*/ false)?;
    let test = test_codex()
        .with_home(home)
        .with_image_store(Arc::new(
            super::image_rollout::RecordingFileAttachmentStore::default(),
        ))
        .build_with_auto_env(&server)
        .await?;
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("attachment-explicit"),
            responses::ev_completed("attachment-explicit"),
        ]),
    )
    .await;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![image(
            &test.cwd.path().join("image.png"),
        )?]))
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert_eq!(
        images(response.single_request().input())[0]["file_id"],
        "file_uploaded_image"
    );
    assert!(
        !state.exists(),
        "selected component must not start over explicit injection"
    );
    test.codex.shutdown_and_wait().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn isolated_default_session_excludes_installed_attachment_component() -> Result<()> {
    let server = responses::start_mock_server().await;
    let home = Arc::new(tempfile::tempdir()?);
    let state = install(home.path(), /*fail*/ false)?;
    let test = test_codex()
        .with_home(home)
        .with_thread_manager(ThreadManager::with_default_attachment_store_components)
        .build_with_auto_env(&server)
        .await?;
    let mut options = StartThreadOptions::new(test.config.clone());
    options.environments = Some(test.codex.environment_selections().await);
    options
        .thread_extension_init
        .insert(SessionIsolation::Isolated);
    let isolated = test.thread_manager.start_thread(options).await?;
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("attachment-isolated"),
            responses::ev_completed("attachment-isolated"),
        ]),
    )
    .await;
    isolated
        .thread
        .start_or_steer_turn(TurnInputRequest::user_input(vec![image(
            &test.cwd.path().join("image.png"),
        )?]))
        .await?;
    wait_for_event(&isolated.thread, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert!(
        images(response.single_request().input())[0]["image_url"]
            .as_str()
            .context("inline image")?
            .starts_with("data:image/")
    );
    assert!(
        !state.exists(),
        "isolated session must not start globally selected store"
    );
    isolated.thread.shutdown_and_wait().await?;
    test.codex.shutdown_and_wait().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn component_upload_failure_retains_native_inline_fallback() -> Result<()> {
    let server = responses::start_mock_server().await;
    let home = Arc::new(tempfile::tempdir()?);
    let state = install(home.path(), /*fail*/ true)?;
    let test = test_codex()
        .with_home(home)
        .with_thread_manager(ThreadManager::with_default_attachment_store_components)
        .build_with_auto_env(&server)
        .await?;
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("attachment-failed"),
            responses::ev_completed("attachment-failed"),
        ]),
    )
    .await;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![image(
            &test.cwd.path().join("image.png"),
        )?]))
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert!(
        images(response.single_request().input())[0]["image_url"]
            .as_str()
            .context("fallback inline image")?
            .starts_with("data:image/")
    );
    assert_eq!(std::fs::read_to_string(state.join("count"))?, "1");
    test.codex.shutdown_and_wait().await?;
    Ok(())
}
