//! Owns accepted native maintenance until cooperative shutdown has joined it.
//!
//! Ordinary background errors retain their historical warning-only policy. A
//! panic or failed worker join is different: close cannot promise that cleanup
//! completed, so every shutdown observer receives the same retained failure.

use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;

use codex_rollout::RolloutCompressionTrigger;
use codex_rollout::RolloutCompressionWorkerError;
use codex_rollout::run_rollout_compression_worker;
use tokio::runtime::Handle;
use tokio::sync::OnceCell;
use tokio::sync::watch;
use tokio::task::JoinError;
use tokio::task::JoinSet;

use super::LocalThreadStore;
use crate::RolloutMaintenance;
use crate::ThreadStoreError;
use crate::ThreadStoreResult;

type Completion = Result<(), String>;

#[derive(Default)]
struct State {
    closing: bool,
    finishing_store: bool,
    jobs: JoinSet<Completion>,
    failure: Option<String>,
    runtime: Option<Handle>,
}

pub(super) struct MaintenanceSupervisor {
    state: Mutex<State>,
    cancellation: watch::Sender<bool>,
    completion: watch::Sender<Option<Completion>>,
    store_completion: watch::Sender<Option<Completion>>,
}

impl Default for MaintenanceSupervisor {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
            cancellation: watch::channel(false).0,
            completion: watch::channel(None).0,
            store_completion: watch::channel(None).0,
        }
    }
}

impl MaintenanceSupervisor {
    fn schedule(
        &self,
        job: impl Future<Output = Completion> + Send + 'static,
    ) -> ThreadStoreResult<()> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closing {
            return Err(ThreadStoreError::Conflict {
                message: "thread-store maintenance is shutting down".to_owned(),
            });
        }
        while let Some(result) = state.jobs.try_join_next() {
            retain_failure(&mut state.failure, result);
        }
        let runtime = Handle::try_current().map_err(|error| ThreadStoreError::Internal {
            message: format!("start thread-store maintenance: {error}"),
        })?;
        state.jobs.spawn_on(job, &runtime);
        state.runtime = Some(runtime);
        Ok(())
    }

    pub(super) fn begin_shutdown(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closing {
            return;
        }
        // Registration and this fence share one lock: every acknowledged job
        // belongs to the drain, and no new job can appear after it takes the set.
        state.closing = true;
        self.cancellation.send_replace(true);
        let mut jobs = std::mem::take(&mut state.jobs);
        let mut failure = state.failure.take();
        if jobs.is_empty() {
            self.completion
                .send_replace(Some(failure.map_or(Ok(()), Err)));
            return;
        }
        let Some(runtime) = state.runtime.clone() else {
            self.completion.send_replace(Some(Err(
                "maintenance jobs have no owning runtime; completion is unknown".to_owned(),
            )));
            return;
        };
        let completion = self.completion.clone();
        let drain = runtime.spawn(async move {
            while let Some(result) = jobs.join_next().await {
                retain_failure(&mut failure, result);
            }
            failure.map_or(Ok(()), Err)
        });
        // Neither task is tied to a close waiter. The monitor also turns a
        // supervisor panic into an observed failure instead of hanging waiters.
        runtime.spawn(async move {
            let result = drain.await.unwrap_or_else(|error| {
                Err(format!(
                    "maintenance supervisor failed; cleanup completion is unknown: {error}"
                ))
            });
            completion.send_replace(Some(result));
        });
    }

    pub(super) async fn shutdown(&self) -> ThreadStoreResult<()> {
        self.begin_shutdown();
        observe_completion(&self.completion).await
    }

    /// Called after writers are fenced. Unlike begin_shutdown, this final stage
    /// may close the store-owned projection pool. It never closes host state DBs.
    pub(super) fn finish_store(self: &Arc<Self>, history: Arc<OnceCell<sqlx::SqlitePool>>) {
        self.begin_shutdown();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.finishing_store {
            return;
        }
        state.finishing_store = true;
        let runtime = state.runtime.clone().or_else(|| Handle::try_current().ok());
        let Some(runtime) = runtime else {
            // A store which never opened asynchronous resources needs no runtime
            // to complete shutdown. Any other case must report uncertainty.
            let result = if history.get().is_none() {
                self.completion.borrow().clone().unwrap_or_else(|| {
                    Err(
                        "maintenance completion is unavailable without an owning runtime"
                            .to_owned(),
                    )
                })
            } else {
                Err("cannot close history pool without an owning runtime".to_owned())
            };
            self.store_completion.send_replace(Some(result));
            return;
        };
        let supervisor = Arc::clone(self);
        let finish = runtime.spawn(async move {
            let result = supervisor
                .shutdown()
                .await
                .map_err(|error| error.to_string());
            // Pool closure still runs after a failed worker join. The retained
            // failure prevents this cleanup attempt from being called durable.
            if let Some(pool) = history.get() {
                pool.close().await;
            }
            result
        });
        let completion = self.store_completion.clone();
        runtime.spawn(async move {
            let result = finish.await.unwrap_or_else(|error| {
                Err(format!(
                    "store shutdown supervisor failed; pool closure is unconfirmed: {error}"
                ))
            });
            completion.send_replace(Some(result));
        });
    }

    pub(super) async fn wait_closed(&self) -> ThreadStoreResult<()> {
        observe_completion(&self.store_completion).await
    }
}

async fn observe_completion(
    completion: &watch::Sender<Option<Completion>>,
) -> ThreadStoreResult<()> {
    let mut completion = completion.subscribe();
    loop {
        if let Some(result) = completion.borrow().clone() {
            return result.map_err(|message| ThreadStoreError::Internal { message });
        }
        completion
            .changed()
            .await
            .map_err(|error| ThreadStoreError::Internal {
                message: format!("maintenance supervisor disappeared: {error}"),
            })?;
    }
}

fn retain_failure(failure: &mut Option<String>, result: Result<Completion, JoinError>) {
    let result = result.unwrap_or_else(|error| {
        Err(format!(
            "maintenance worker failed; cleanup completion is unknown: {error}"
        ))
    });
    if let Err(error) = result {
        failure.get_or_insert(error);
    }
}

pub(super) fn schedule(
    store: &LocalThreadStore,
    request: RolloutMaintenance,
) -> ThreadStoreResult<()> {
    let mut cancellation = store.maintenance.cancellation.subscribe();
    let owned_store = store.clone();
    let (migrate, compress) = match request {
        RolloutMaintenance::Startup {
            migrate_rollouts,
            compress_rollouts,
        } => (
            migrate_rollouts,
            compress_rollouts.then_some(RolloutCompressionTrigger::Startup),
        ),
        RolloutMaintenance::MigrateOnStartup => (true, None),
        RolloutMaintenance::Compress => (false, Some(RolloutCompressionTrigger::Rpc)),
    };
    store.maintenance.schedule(async move {
        let should_migrate = !*cancellation.borrow() && migrate && owned_store.state_db.is_some();
        if should_migrate
            && let Err(error) = owned_store.migrate_rollouts_on_startup_cancellable(&mut cancellation).await
        {
            tracing::warn!("failed to migrate legacy rollouts on startup: {error}");
        }
        let stopped = *cancellation.borrow();
        if !stopped
            && let Some(trigger) = compress
        {
            match run_rollout_compression_worker(
                owned_store.config.codex_home,
                trigger,
                cancellation,
            ).await {
                Ok(()) => {}
                Err(RolloutCompressionWorkerError::Operation(error)) => {
                    tracing::warn!("rollout compression worker failed: {error}");
                }
                Err(RolloutCompressionWorkerError::TaskJoin(error)) => {
                    return Err(format!("rollout compression worker join failed; cleanup completion is unknown: {error}"));
                }
            }
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "maintenance_tests.rs"]
mod tests;
