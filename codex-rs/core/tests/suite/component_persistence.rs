//! Engine turns, cold recovery, and native interoperability through installed storage.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use codex_core::CodexThread;
use codex_core::StartThreadOptions;
use codex_core::TurnInputRequest;
use codex_core::build_prompt_input;
use codex_core::config::ConfigBuilder;
use codex_core::config::ConfigOverrides;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_history::InitialHistory;
use codex_history::ResumedHistory;
use codex_home::CodexHomeUserInstructionsProvider;
use codex_protocol::mcp::ClientMcpExtensions;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::user_input::UserInput;
use codex_thread_store::ForkBoundary;
use codex_thread_store::LoadThreadHistoryParams;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::PrepareForkParams;
use codex_thread_store_component::ProcessThreadStore;
use core_test_support::responses;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::json;

const ID: &str = "test.engine-storage";

enum Fixture {
    Native,
    IncompatibleHandshake,
}

fn install(home: &Path, fixture: Fixture) -> Result<()> {
    let package = tempfile::tempdir()?;
    let executable = match fixture {
        Fixture::Native => {
            let executable = format!("storage{}", std::env::consts::EXE_SUFFIX);
            std::fs::copy(
                codex_utils_cargo_bin::cargo_bin("codex-thread-store-local-plugin")?,
                package.path().join(&executable),
            )?;
            executable
        }
        Fixture::IncompatibleHandshake => {
            let source = tempfile::tempdir()?;
            std::fs::write(
                source.path().join("__main__.py"),
                "import json, sys\njson.loads(sys.stdin.readline())\nprint(json.dumps({'type':'ready','api_version':999}), flush=True)\n",
            )?;
            let python = if cfg!(windows) { "python" } else { "python3" };
            let output = Command::new(python)
                .args(["-m", "zipapp"])
                .arg(source.path())
                .args(["--python", "/usr/bin/env python3", "--output"])
                .arg(package.path().join("storage.pyz"))
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "build failing storage fixture: {output:?}"
            );
            "storage.pyz".to_owned()
        }
    };
    std::fs::write(
        package.path().join("codex-component.json"),
        serde_json::to_vec(&json!({
            "api_version": 1, "id": ID, "version": "1.0.0", "entrypoint": executable,
            "components": [{"kind":"thread_store", "name":"default", "contract_version":codex_thread_store_component::THREAD_STORE_CONTRACT_VERSION}]
        }))?,
    )?;
    codex_component_host::install(home, package.path())?;
    codex_component_host::select(home, "thread_store", "default", Some(ID))?;
    // The installation must own its bytes before the source package disappears.
    package.close()?;
    Ok(())
}

async fn turn(thread: &CodexThread, text: &str) -> Result<()> {
    thread
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: text.to_owned(),
            text_elements: Vec::new(),
        }]))
        .await?;
    let event = wait_for_event(thread, |event| {
        matches!(event, EventMsg::TurnComplete(_) | EventMsg::Error(_))
    })
    .await;
    let EventMsg::TurnComplete(completed) = event else {
        anyhow::bail!("turn failed: {event:?}");
    };
    assert_eq!(completed.error, None);
    Ok(())
}

#[test_case::test_case(ThreadHistoryMode::Legacy; "legacy")]
#[test_case::test_case(ThreadHistoryMode::Paginated; "paginated")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_storage_runs_turns_reopens_forks_and_matches_native(
    mode: ThreadHistoryMode,
) -> Result<()> {
    let server = responses::start_mock_server().await;
    let home = Arc::new(tempfile::tempdir()?);
    install(home.path(), Fixture::Native)?;
    let initial = test_codex()
        .with_home(Arc::clone(&home))
        .with_history_mode(Some(mode))
        .build_with_auto_env(&server)
        .await?;
    assert!(initial.thread_store.as_any().is::<ProcessThreadStore>());
    assert_eq!(initial.codex.config_snapshot().await.history_mode, mode);
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("seed-message", "The durable answer is forty two."),
            responses::ev_completed("seed-response"),
        ]),
    )
    .await;
    turn(&initial.codex, "Remember this durable question.").await?;
    initial.codex.shutdown_and_wait().await?;
    let params = LoadThreadHistoryParams {
        thread_id: initial.session_configured.thread_id,
        include_archived: true,
    };
    let saved = initial
        .thread_store
        .load_latest_model_context(params.clone())
        .await?;
    initial.thread_store.shutdown_store().await?;

    // A fresh manager opens a fresh storage process against the existing files.
    let reopened = test_codex()
        .with_home(Arc::clone(&home))
        .with_history_mode(Some(mode))
        .build_with_auto_env(&server)
        .await?;
    assert!(reopened.thread_store.as_any().is::<ProcessThreadStore>());
    reopened.codex.shutdown_and_wait().await?;
    let restored = reopened
        .thread_store
        .load_latest_model_context(params.clone())
        .await?;
    assert_eq!(
        serde_json::to_value(&restored)?,
        serde_json::to_value(&saved)?
    );
    let resumed = reopened
        .thread_manager
        .resume_thread_with_history(
            reopened.config.clone(),
            InitialHistory::Resumed(ResumedHistory {
                conversation_id: restored.thread_id,
                history: Arc::new(restored.items),
                rollout_path: initial.session_configured.rollout_path.clone(),
            }),
            reopened.thread_manager.auth_manager(),
            /*parent_trace*/ None,
            ClientMcpExtensions::default(),
        )
        .await?;
    assert_eq!(resumed.thread.config_snapshot().await.history_mode, mode);
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message(
                "resumed-message",
                "The recovered answer is still forty two.",
            ),
            responses::ev_completed("resumed-response"),
        ]),
    )
    .await;
    turn(
        &resumed.thread,
        "Continue after the storage process restarted.",
    )
    .await?;
    let request = response.single_request();
    assert!(request.body_contains_text("Remember this durable question."));
    assert!(request.body_contains_text("The durable answer is forty two."));

    let fork = if mode == ThreadHistoryMode::Paginated {
        let prepared = reopened
            .thread_store
            .prepare_fork(PrepareForkParams {
                thread_id: params.thread_id,
                boundary: ForkBoundary::Latest,
            })
            .await?;
        let mut options = StartThreadOptions::new(reopened.config.clone());
        options.environments = Some(resumed.thread.environment_selections().await);
        let fork = reopened
            .thread_manager
            .fork_prepared_thread(options, prepared)
            .await?;
        let response = responses::mount_sse_once(
            &server,
            responses::sse(vec![
                responses::ev_assistant_message("fork-message", "The fork remembers both turns."),
                responses::ev_completed("fork-response"),
            ]),
        )
        .await;
        turn(&fork.thread, "Continue in a reference-backed fork.").await?;
        let request = response.single_request();
        assert!(request.body_contains_text("Remember this durable question."));
        assert!(request.body_contains_text("The recovered answer is still forty two."));
        fork.thread.shutdown_and_wait().await?;
        let history = reopened
            .thread_store
            .load_latest_model_context(LoadThreadHistoryParams {
                thread_id: fork.thread_id,
                include_archived: true,
            })
            .await?;
        Some(history)
    } else {
        None
    };
    resumed.thread.shutdown_and_wait().await?;
    let saved = reopened
        .thread_store
        .load_latest_model_context(params.clone())
        .await?;
    reopened.thread_store.shutdown_store().await?;

    // The existing built-in implementation must read exactly the same persisted data.
    codex_component_host::select(
        home.path(),
        "thread_store",
        "default",
        /*plugin_id*/ None,
    )?;
    let native = test_codex()
        .with_home(home)
        .with_history_mode(Some(mode))
        .build_with_auto_env(&server)
        .await?;
    assert!(native.thread_store.as_any().is::<LocalThreadStore>());
    native.codex.shutdown_and_wait().await?;
    let restored = native
        .thread_store
        .load_latest_model_context(params)
        .await?;
    assert_eq!(
        serde_json::to_value(&restored)?,
        serde_json::to_value(&saved)?
    );
    if let Some(fork) = fork {
        let native_fork = native
            .thread_store
            .load_latest_model_context(LoadThreadHistoryParams {
                thread_id: fork.thread_id,
                include_archived: true,
            })
            .await?;
        assert_eq!(
            serde_json::to_value(native_fork)?,
            serde_json::to_value(fork)?
        );
    }
    let native_resumed = native
        .thread_manager
        .resume_thread_with_history(
            native.config.clone(),
            InitialHistory::Resumed(ResumedHistory {
                conversation_id: restored.thread_id,
                history: Arc::new(restored.items),
                rollout_path: initial.session_configured.rollout_path.clone(),
            }),
            native.thread_manager.auth_manager(),
            /*parent_trace*/ None,
            ClientMcpExtensions::default(),
        )
        .await?;
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![responses::ev_completed("native-resumed-response")]),
    )
    .await;
    turn(&native_resumed.thread, "Continue with native storage.").await?;
    let request = response.single_request();
    assert!(request.body_contains_text("Remember this durable question."));
    assert!(request.body_contains_text("The durable answer is forty two."));
    assert!(request.body_contains_text("The recovered answer is still forty two."));
    native_resumed.thread.shutdown_and_wait().await?;
    native.thread_store.shutdown_store().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_storage_startup_failure_stops_engine_and_prompt_debug() -> Result<()> {
    let server = responses::start_mock_server().await;
    let home = Arc::new(tempfile::tempdir()?);
    install(home.path(), Fixture::IncompatibleHandshake)?;
    let error = test_codex()
        .with_home(Arc::clone(&home))
        .build_with_auto_env(&server)
        .await
        .err()
        .context("selected broken storage must fail engine startup")?;
    assert!(
        format!("{error:#}").contains("initialize selected thread-store component"),
        "{error:#}"
    );
    assert!(
        !home.path().join("sessions").exists(),
        "native fallback must not create a rollout"
    );
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .all(|request| !request.url.path().ends_with("/responses"))
    );

    let cwd = tempfile::tempdir()?;
    let config = ConfigBuilder::default()
        .codex_home(home.path().to_path_buf())
        .harness_overrides(ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            codex_self_exe: Some(std::env::current_exe()?),
            ..ConfigOverrides::default()
        })
        .build()
        .await?;
    let error = build_prompt_input(
        config,
        vec![UserInput::Text {
            text: "must not fall back".to_owned(),
            text_elements: Vec::new(),
        }],
        /*state_db*/ None,
        Arc::new(ExtensionRegistryBuilder::new().build()),
        Arc::new(CodexHomeUserInstructionsProvider::new(
            home.path().to_path_buf().try_into()?,
        )),
    )
    .await
    .expect_err("prompt debugging must honor the selected storage component");
    assert!(
        error
            .to_string()
            .contains("initialize selected thread-store component"),
        "{error}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_debug_uses_installed_storage_with_ephemeral_history() -> Result<()> {
    let home = tempfile::tempdir()?;
    let cwd = tempfile::tempdir()?;
    install(home.path(), Fixture::Native)?;
    std::fs::write(
        home.path().join("AGENTS.md"),
        "Keep the storage component instructions.",
    )?;
    let config = ConfigBuilder::default()
        .codex_home(home.path().to_path_buf())
        .harness_overrides(ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            codex_self_exe: Some(std::env::current_exe()?),
            ..ConfigOverrides::default()
        })
        .build()
        .await?;
    let input = build_prompt_input(
        config,
        vec![UserInput::Text {
            text: "debug through installed storage".to_owned(),
            text_elements: Vec::new(),
        }],
        /*state_db*/ None,
        Arc::new(ExtensionRegistryBuilder::new().build()),
        Arc::new(CodexHomeUserInstructionsProvider::new(
            home.path().to_path_buf().try_into()?,
        )),
    )
    .await?;
    let input = serde_json::to_value(input)?;
    assert_eq!(
        input
            .as_array()
            .context("prompt input")?
            .last()
            .context("user message")?["content"][0]["text"],
        "debug through installed storage"
    );
    assert!(
        input
            .to_string()
            .contains("Keep the storage component instructions.")
    );
    assert!(
        !home.path().join("sessions").exists(),
        "ephemeral prompt must not persist a rollout"
    );
    Ok(())
}

// Linux /proc provides direct child-exit evidence without changing the native
// storage plugin. Other platforms exercise the generic owner guard separately.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_prompt_debug_closes_storage_while_thread_stop_is_stalled() -> Result<()> {
    struct BlockThreadStop {
        entered: Arc<tokio::sync::Semaphore>,
        release: Arc<tokio::sync::Semaphore>,
    }

    impl codex_extension_api::ThreadLifecycleContributor<codex_core::config::Config>
        for BlockThreadStop
    {
        fn on_thread_stop<'a>(
            &'a self,
            _input: codex_extension_api::ThreadStopInput<'a>,
        ) -> codex_extension_api::ExtensionFuture<'a, ()> {
            Box::pin(async move {
                self.entered.add_permits(1);
                self.release
                    .acquire()
                    .await
                    .expect("release stalled lifecycle")
                    .forget();
            })
        }
    }

    let home = tempfile::tempdir()?;
    let cwd = tempfile::tempdir()?;
    install(home.path(), Fixture::Native)?;
    let executable = codex_component_host::ComponentCatalog::load(home.path())?
        .selected("thread_store", "default")
        .context("selected store")?
        .entrypoint
        .canonicalize()?;
    let config = ConfigBuilder::default()
        .codex_home(home.path().to_path_buf())
        .harness_overrides(ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            codex_self_exe: Some(std::env::current_exe()?),
            ..ConfigOverrides::default()
        })
        .build()
        .await?;
    let entered = Arc::new(tokio::sync::Semaphore::new(0));
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    let mut extensions = ExtensionRegistryBuilder::new();
    extensions.thread_lifecycle_contributor(Arc::new(BlockThreadStop {
        entered: Arc::clone(&entered),
        release: Arc::clone(&release),
    }));
    let prompt = tokio::spawn(build_prompt_input(
        config,
        vec![UserInput::Text {
            text: "cancel this debug prompt".to_owned(),
            text_elements: Vec::new(),
        }],
        /*state_db*/ None,
        Arc::new(extensions.build()),
        Arc::new(CodexHomeUserInstructionsProvider::new(
            home.path().to_path_buf().try_into()?,
        )),
    ));
    tokio::time::timeout(std::time::Duration::from_secs(10), entered.acquire())
        .await??
        .forget();
    let process = std::fs::read_dir("/proc")?
        .filter_map(std::result::Result::ok)
        .find_map(|entry| {
            std::fs::read_link(entry.path().join("exe"))
                .ok()
                .filter(|path| path == &executable)
                .map(|_| entry.path())
        })
        .context("native storage process is alive while thread stop is stalled")?;

    prompt.abort();
    assert!(
        prompt
            .await
            .expect_err("prompt caller cancelled")
            .is_cancelled()
    );
    let stopped = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while process.exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    // Release the unrelated session teardown only after checking process exit:
    // storage cleanup must not depend on that session dropping its store Arc.
    release.add_permits(1);
    stopped.context(
        "cancelled prompt must close native storage before stalled thread stop is released",
    )?;
    Ok(())
}
