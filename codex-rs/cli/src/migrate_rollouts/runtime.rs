//! Dedicated manual-maintenance composition; never starts runtime maintenance.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use codex_component_host::ComponentCatalog;
use codex_core::config::Config;
use codex_rollout::StateDbHandle;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::LocalThreadStoreConfig;
use codex_thread_store::RolloutMigrationCompletion;
use codex_thread_store::RolloutMigrationMode;
use codex_thread_store::RolloutMigrationOptions;
use codex_thread_store::RolloutMigrationPhase;
use codex_thread_store::RolloutMigrationReport;
use codex_thread_store::RolloutMigrationRun;
use codex_thread_store::RolloutMigrationSnapshot;
use codex_thread_store::RolloutMigrationTelemetry;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreError;
use codex_thread_store::ThreadStoreResult;
use codex_thread_store::ThreadStoreShutdownGuard;
use codex_thread_store_component::LocalStoragePaths;
use codex_thread_store_component::ProcessThreadStore;
use codex_thread_store_component::StorageInitialization;
use codex_thread_store_component::THREAD_STORE_CONTRACT_VERSION;
use tokio::sync::watch;

const CLEANUP_BUDGET: Duration = Duration::from_secs(210);

pub(super) struct MigrationExecution {
    pub report: ThreadStoreResult<RolloutMigrationReport>,
    pub completion: RolloutMigrationCompletion,
    pub cleanup: anyhow::Result<()>,
}

struct StoreOwner {
    store: Arc<dyn ThreadStore>,
    guard: ThreadStoreShutdownGuard,
    state_db: Option<StateDbHandle>,
    selected: bool,
}

impl StoreOwner {
    async fn open(config: &Config, mode: RolloutMigrationMode) -> anyhow::Result<Self> {
        // Resolve first: a selected implementation never opens host-native storage.
        let binding =
            ComponentCatalog::load(&config.codex_home)?.selected("thread_store", "default");
        let selected = binding.is_some();
        let (store, state_db): (Arc<dyn ThreadStore>, _) = match binding {
            Some(binding) => {
                let initialization = StorageInitialization {
                    contract_version: THREAD_STORE_CONTRACT_VERSION,
                    paths: LocalStoragePaths {
                        codex_home: config.codex_home.to_path_buf(),
                        sqlite_home: config.sqlite_config().home().to_path_buf(),
                    },
                    default_model_provider_id: config.model_provider_id.clone(),
                    state_db_enabled: mode == RolloutMigrationMode::Apply,
                    startup_migration: false,
                    startup_compression: false,
                };
                (
                    Arc::new(ProcessThreadStore::connect(binding, initialization).await?),
                    None,
                )
            }
            None => {
                let state_db = if mode == RolloutMigrationMode::Apply {
                    Some(
                        codex_rollout::state_db::try_init(config)
                            .await
                            .context("failed to initialize local thread metadata")?,
                    )
                } else {
                    None
                };
                let store = LocalThreadStore::new(
                    LocalThreadStoreConfig::from_config(config),
                    state_db.clone(),
                );
                (Arc::new(store), state_db)
            }
        };
        let guard = ThreadStoreShutdownGuard::new(Arc::clone(&store));
        Ok(Self {
            store,
            guard,
            state_db,
            selected,
        })
    }

    async fn close(&self) -> anyhow::Result<()> {
        let result = self.guard.shutdown().await;
        // Always close native metadata, even if the store reports cleanup failure.
        if let Some(state_db) = &self.state_db {
            state_db.close().await;
        }
        result.context("migration store cleanup failed; durability is uncertain")
    }
}

/// Registers signals before opening a worker and retains the accepted operation
/// on the first interrupt. Only a repeated signal or the outer deadline abandons
/// that wait; neither can be reported as a successful durable migration.
pub(super) async fn execute(
    config: &Config,
    options: RolloutMigrationOptions,
    progress: impl FnMut(RolloutMigrationSnapshot),
) -> anyhow::Result<MigrationExecution> {
    let mut signal = ShutdownSignal::new()?;
    let (stop, receiver) = watch::channel(false);
    let operation = execute_owned(config, options, progress, receiver);
    tokio::pin!(operation);
    let signal_result = tokio::select! {
        result = &mut operation => return result,
        result = signal.recv() => result,
    };
    stop.send_replace(true);
    eprintln!("Stopping migration; waiting for accepted work and storage cleanup...");
    let result = tokio::select! {
        result = &mut operation => result,
        _ = signal.recv() => anyhow::bail!(
            "forced migration shutdown after repeated signal; durability is uncertain"),
        _ = tokio::time::sleep(CLEANUP_BUDGET) => anyhow::bail!(
            "migration shutdown exceeded its cleanup budget; durability is uncertain"),
    };
    match (signal_result, result) {
        (Err(signal), Err(operation)) => {
            Err(operation.context(format!("signal handling failed: {signal}")))
        }
        (Err(signal), Ok(_)) => Err(signal.into()),
        (Ok(()), result) => result,
    }
}

async fn execute_owned(
    config: &Config,
    options: RolloutMigrationOptions,
    mut progress: impl FnMut(RolloutMigrationSnapshot),
    mut stop: watch::Receiver<bool>,
) -> anyhow::Result<MigrationExecution> {
    let owner = StoreOwner::open(config, options.mode).await?;
    let telemetry = owner
        .selected
        .then(|| RolloutMigrationTelemetry::manual(&options));
    let mut completion = RolloutMigrationCompletion::Completed;
    let mut cleanup = Ok(());
    let report = if *stop.borrow() {
        completion = RolloutMigrationCompletion::Cancelled;
        Err(ThreadStoreError::Conflict {
            message: "migration cancelled before admission".into(),
        })
    } else if !owner.store.supports_manual_rollout_migration() {
        Err(ThreadStoreError::Unsupported {
            operation: "manual_rollout_migration",
        })
    } else {
        // Abandoning Start queues its pre-reserved release. Whole-store shutdown
        // below still joins any native work admitted before that release arrives.
        let start = tokio::select! {
            biased;
            _ = stop.changed() => None,
            result = owner.store.start_rollout_migration(options) => Some(result),
        };
        match start {
            Some(Ok(run)) => {
                let result = observe(run.as_ref(), &mut progress, &mut stop).await;
                let report = match result {
                    Ok(phase) => {
                        if phase == RolloutMigrationPhase::Cancelled || *stop.borrow() {
                            completion = RolloutMigrationCompletion::Cancelled;
                        }
                        run.report().await
                    }
                    Err(error) => Err(error),
                };
                cleanup = run
                    .close()
                    .await
                    .context("migration release failed; durability is uncertain");
                report
            }
            Some(Err(error)) => Err(error),
            None => {
                completion = RolloutMigrationCompletion::Cancelled;
                Err(ThreadStoreError::Conflict {
                    message: "migration cancelled during admission".into(),
                })
            }
        }
    };
    if let Some(telemetry) = telemetry {
        telemetry.finish(&report, completion);
    }
    let store_cleanup = owner.close().await;
    cleanup = combine_cleanup(cleanup, store_cleanup);
    if *stop.borrow() {
        completion = RolloutMigrationCompletion::Cancelled;
    }
    Ok(MigrationExecution {
        report,
        completion,
        cleanup,
    })
}

async fn observe(
    run: &dyn RolloutMigrationRun,
    progress: &mut impl FnMut(RolloutMigrationSnapshot),
    stop: &mut watch::Receiver<bool>,
) -> ThreadStoreResult<RolloutMigrationPhase> {
    let mut cancelled = false;
    let mut revision = None;
    let mut interval = tokio::time::interval(super::TTY_PROGRESS_INTERVAL);
    loop {
        if *stop.borrow() && !cancelled {
            run.cancel().await?;
            cancelled = true;
        }
        tokio::select! {
            _ = stop.changed(), if !cancelled => continue,
            _ = interval.tick() => {}
        }
        let snapshot = tokio::select! {
            _ = stop.changed(), if !cancelled => continue,
            snapshot = run.snapshot() => snapshot?,
        };
        let phase = snapshot.phase;
        if revision != Some(snapshot.revision) {
            revision = Some(snapshot.revision);
            progress(snapshot);
        }
        match phase {
            RolloutMigrationPhase::Completed | RolloutMigrationPhase::Cancelled => {
                return Ok(phase);
            }
            RolloutMigrationPhase::Scanning | RolloutMigrationPhase::Running => {}
        }
    }
}

pub(super) fn combine_cleanup(
    result: anyhow::Result<()>,
    cleanup: anyhow::Result<()>,
) -> anyhow::Result<()> {
    match (result, cleanup) {
        (Ok(()), cleanup) => cleanup,
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("also failed cleanup: {cleanup:#}")))
        }
    }
}

struct ShutdownSignal {
    #[cfg(unix)]
    interrupt: tokio::signal::unix::Signal,
    #[cfg(unix)]
    terminate: tokio::signal::unix::Signal,
    #[cfg(windows)]
    interrupt: tokio::signal::windows::CtrlC,
}

impl ShutdownSignal {
    fn new() -> std::io::Result<Self> {
        #[cfg(unix)]
        {
            Ok(Self {
                interrupt: tokio::signal::unix::signal(
                    tokio::signal::unix::SignalKind::interrupt(),
                )?,
                terminate: tokio::signal::unix::signal(
                    tokio::signal::unix::SignalKind::terminate(),
                )?,
            })
        }
        #[cfg(windows)]
        {
            Ok(Self {
                interrupt: tokio::signal::windows::ctrl_c()?,
            })
        }
    }

    async fn recv(&mut self) -> std::io::Result<()> {
        #[cfg(unix)]
        let event = tokio::select! {
            event = self.interrupt.recv() => event,
            event = self.terminate.recv() => event,
        };
        #[cfg(windows)]
        let event = self.interrupt.recv().await;
        event.ok_or_else(|| std::io::Error::other("migration signal stream closed"))
    }
}
