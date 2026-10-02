//! Fresh child only: real public-handle Drop closes a held featured HTTP request.
//! The parent supplies isolated home/config and unchanged production Git routing.

use super::*;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_app_server_protocol::ClientInfo;
use codex_core::config::ConfigBuilder;
use codex_core::config::ConfigOverrides;
use codex_core_plugins::FeaturedWarmupObservation;
use codex_core_plugins::FeaturedWarmupOutcome;
use codex_core_plugins::FeaturedWarmupOwnership;
use codex_core_plugins::PluginStartupProcessShutdown;
use codex_core_plugins::PluginsManager;
use codex_core_plugins::startup_sync::CuratedSyncOperationDisposition;
use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio::time::timeout_at;

// Includes startup: the native featured request's unchanged timeout is 10s.
const EOF_WINDOW: Duration = Duration::from_secs(5);
const CHILD_WINDOW: Duration = Duration::from_secs(90);

async fn launch(home: &Path) -> Result<InProcessClientHandle> {
    let config = Arc::new(
        ConfigBuilder::default()
            .codex_home(home.to_path_buf())
            .harness_overrides(ConfigOverrides {
                cwd: Some(home.to_path_buf()),
                bypass_hook_trust: Some(true),
                ..Default::default()
            })
            .build()
            .await?,
    );
    ensure!(
        config.sqlite_config().home() == home,
        "fixture SQLite home differs"
    );
    let state_db = codex_rollout::state_db::try_init(config.as_ref()).await?;
    Ok(start(InProcessStartArgs {
        arg0_paths: Arg0DispatchPaths::default(),
        config,
        cli_overrides: Vec::new(),
        loader_overrides: LoaderOverrides::default(),
        strict_config: false,
        cloud_config_bundle: CloudConfigBundleLoader::default(),
        embedded_network_policy: Default::default(),
        thread_config_loader: Arc::new(codex_config::NoopThreadConfigLoader),
        feedback: CodexFeedback::new(),
        log_db: None,
        state_db: Some(state_db),
        environment_manager: Arc::new(EnvironmentManager::default_for_tests()),
        config_warnings: Vec::new(),
        session_source: SessionSource::Cli,
        enable_codex_api_key_env: false,
        initialize: InitializeParams {
            client_info: ClientInfo {
                name: "featured-drop-acceptance".into(),
                title: None,
                version: "1".into(),
            },
            capabilities: None,
        },
        channel_capacity: 1024,
    })
    .await?)
}

async fn held_featured_request(listener: &TcpListener, deadline: Instant) -> Result<TcpStream> {
    timeout_at(deadline, async {
        let (mut peer, address) = listener.accept().await?;
        ensure!(address.ip().is_loopback(), "non-loopback fixture peer");
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            ensure!(
                header.len() < 8192,
                "fixture request headers exceeded bound"
            );
            header.push(peer.read_u8().await?);
        }
        ensure!(
            header.starts_with(b"GET /plugins/featured?platform=codex HTTP/1.1\r\n"),
            "request did not use the native featured route"
        );
        // Never return any response bytes: EOF cannot mean successful publication.
        Ok(peer)
    })
    .await?
}

async fn peer_eof(peer: &mut TcpStream, deadline: Instant) -> Result<()> {
    let mut byte = [0_u8; 1];
    ensure!(
        timeout_at(deadline, peer.read(&mut byte)).await?? == 0,
        "held featured peer did not observe EOF"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires one fresh parent-owned ELF and isolated local-Git fixture"]
async fn production_featured_public_drop_child() -> Result<()> {
    let home = std::path::PathBuf::from(
        std::env::var_os("CODEX_TEST_FEATURED_WARMUP_HOME")
            .context("featured fixture sentinel missing")?,
    );
    ensure!(
        home.is_absolute() && home.is_dir(),
        "fixture home must be absolute"
    );
    ensure!(
        !home.join("auth.json").exists(),
        "fixture must not contain credentials"
    );
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let base_url = format!("http://{}", listener.local_addr()?);
    let config_path = home.join("config.toml");
    let config = tokio::fs::read_to_string(&config_path).await?;
    ensure!(
        !config.contains("chatgpt_base_url"),
        "fixture route already configured"
    );
    tokio::fs::write(
        &config_path,
        format!("chatgpt_base_url = '{base_url}'\n{config}"),
    )
    .await?;
    let deadline = Instant::now() + CHILD_WINDOW;
    let mut runtimes = Vec::new();
    let expected = FeaturedWarmupObservation {
        ownership: FeaturedWarmupOwnership::Joined,
        outcome: Some(FeaturedWarmupOutcome::Cancelled),
    };
    let operation: Result<()> = async {
        let a_deadline = Instant::now() + EOF_WINDOW;
        let a = timeout_at(a_deadline, launch(&home)).await??;
        runtimes.push(a.runtime_handle.abort_handle());
        let scope = a
            ._featured_warmup_lifecycle
            .as_ref()
            .context("A real runtime did not install featured custody")?
            .scope();
        let mut peer = held_featured_request(&listener, a_deadline).await?;
        let generation = PluginsManager::observe_curated_repo_sync()
            .context("curated startup generation absent")?
            .generation;
        ensure!(generation.is_some(), "curated startup generation absent");
        let retained_sender = a.sender();
        drop(a);
        // Critical order: neither wait_until nor process-final closure has run.
        // Keeping the sender alive prevents client channel closure as the proof.
        peer_eof(&mut peer, a_deadline).await?;
        let observed = scope.wait_until(a_deadline.into_std()).await;
        ensure!(
            observed == expected,
            "A scope outcome differs: {observed:?}"
        );
        drop(retained_sender);
        drop(peer);

        // Closing A is local. B in the exact same home still starts real HTTP work.
        let b_deadline = Instant::now() + EOF_WINDOW;
        let b = timeout_at(b_deadline, launch(&home)).await??;
        runtimes.push(b.runtime_handle.abort_handle());
        let scope = b
            ._featured_warmup_lifecycle
            .as_ref()
            .context("B real runtime did not install featured custody")?
            .scope();
        let mut peer = held_featured_request(&listener, b_deadline).await?;
        let (closed, stopped) = tokio::join!(
            peer_eof(&mut peer, b_deadline),
            timeout_at(deadline, b.shutdown()),
        );
        closed?;
        stopped??;
        let observed = scope.wait_until(deadline.into_std()).await;
        ensure!(
            observed == expected,
            "B scope outcome differs: {observed:?}"
        );
        timeout_at(deadline, async {
            loop {
                let native = PluginsManager::observe_curated_repo_sync()
                    .context("curated startup observation absent")?;
                ensure!(
                    native.generation == generation
                        && !native.admission_closed
                        && !native.quarantined
                        && native.unexpected_handles == 0,
                    "embedded replacement changed curated ownership"
                );
                if let Some(outcome) = native.operation {
                    ensure!(
                        outcome == CuratedSyncOperationDisposition::Succeeded,
                        "fixture curated sync did not succeed"
                    );
                    if native.native_handle_finished == Some(true) {
                        break;
                    }
                }
                tokio::task::yield_now().await;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await??;
        Ok(())
    }
    .await;
    drop(listener);
    // Child-only final closure, after replacement proof; never between A and B.
    // Preserve operation failure even if cleanup succeeds, and vice versa.
    let final_owner = PluginStartupProcessShutdown::begin(deadline.into_std())
        .wait_until(deadline.into_std())
        .await;
    let finished = timeout_at(deadline, async {
        while runtimes.iter().any(|runtime| !runtime.is_finished()) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    let cleanup_ok = final_owner.is_complete() && finished;
    tokio::fs::write(
        home.join("featured-warmup-child.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "passed": operation.is_ok() && cleanup_ok,
            "operation_ok": operation.is_ok(), "cleanup_ok": cleanup_ok,
            "owned_scopes_complete": final_owner.is_complete(),
            "tracked_runtimes_finished": finished,
            "a_runtime_result_joined": false,
            "cache_inspected": false, "whole_host_clean": false,
            "scope": "A public Drop/peer EOF/scoped join; B same-home admission/public shutdown",
        }))?,
    )
    .await?;
    ensure!(
        cleanup_ok,
        "featured caller cleanup incomplete; operation={operation:?}"
    );
    operation
}
