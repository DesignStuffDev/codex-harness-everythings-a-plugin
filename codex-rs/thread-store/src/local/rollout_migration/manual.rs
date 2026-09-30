//! Lease observers never own the migration future or its accepted filesystem work.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;

use futures::FutureExt;
use tokio::sync::watch;

use super::LocalThreadStore;
use super::MigrationControl;
use super::RolloutMigrationOptions;
use super::RolloutMigrationPaths;
use super::RolloutMigrationProgress;
use super::RolloutMigrationRateLimiter;
use super::RolloutMigrationReport;
use super::RolloutMigrationTrigger;
use crate::RolloutMigrationPhase;
use crate::RolloutMigrationRun;
use crate::RolloutMigrationSnapshot;
use crate::ThreadStoreError;
use crate::ThreadStoreFuture;
use crate::ThreadStoreResult;

pub(super) struct Observer<P, D> {
    pub(super) progress: P,
    pub(super) discovered: D,
}

#[derive(Default)]
struct State {
    snapshot: RolloutMigrationSnapshot,
    report: Option<ThreadStoreResult<RolloutMigrationReport>>,
}

struct Job {
    state: Mutex<State>,
    cancellation: watch::Sender<bool>,
    completion: watch::Sender<Option<ThreadStoreResult<()>>>,
}

impl Job {
    fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
            cancellation: watch::channel(false).0,
            completion: watch::channel(None).0,
        }
    }

    fn discovered(&self, total: usize) {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        state.snapshot.phase = RolloutMigrationPhase::Running;
        state.snapshot.total_paths = Some(total as u64);
        state.snapshot.revision = state.snapshot.revision.saturating_add(1);
    }

    fn progress(&self, progress: RolloutMigrationProgress) {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        state.snapshot.processed_paths = progress.processed_paths as u64;
        state.snapshot.total_paths = Some(progress.total_paths as u64);
        if let Some(status) = progress.outcome_status {
            state.snapshot.counts.observe(status);
        }
        state.snapshot.revision = state.snapshot.revision.saturating_add(1);
    }

    fn finish(&self, result: ThreadStoreResult<RolloutMigrationReport>, cleanup: ThreadStoreResult<()>) {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        state.snapshot.phase = if *self.cancellation.borrow() {
            RolloutMigrationPhase::Cancelled
        } else {
            RolloutMigrationPhase::Completed
        };
        state.snapshot.revision = state.snapshot.revision.saturating_add(1);
        state.report = Some(result);
        self.completion.send_replace(Some(cleanup));
    }

    async fn wait_closed(&self) -> ThreadStoreResult<()> {
        let mut completion = self.completion.subscribe();
        loop {
            if let Some(result) = completion.borrow().clone() {
                return result;
            }
            completion.changed().await.map_err(|error| ThreadStoreError::Internal {
                message: format!("manual migration owner disappeared: {error}"),
            })?;
        }
    }
}

struct NativeRun {
    job: Arc<Job>,
}

impl RolloutMigrationRun for NativeRun {
    fn snapshot(&self) -> ThreadStoreFuture<'_, RolloutMigrationSnapshot> {
        Box::pin(async {
            Ok(self.job.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).snapshot.clone())
        })
    }

    fn report(&self) -> ThreadStoreFuture<'_, RolloutMigrationReport> {
        Box::pin(async {
            self.job.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).report.clone()
                .unwrap_or_else(|| Err(ThreadStoreError::Conflict {
                    message: "manual rollout migration is still running".to_owned(),
                }))
        })
    }

    fn cancel(&self) -> ThreadStoreFuture<'_, ()> {
        self.job.cancellation.send_replace(true);
        Box::pin(async { Ok(()) })
    }

    fn close(self: Box<Self>) -> ThreadStoreFuture<'static, ()> {
        self.job.cancellation.send_replace(true);
        Box::pin(async move { self.job.wait_closed().await })
    }
}

impl Drop for NativeRun {
    fn drop(&mut self) {
        self.job.cancellation.send_replace(true);
    }
}

pub(in crate::local) fn start(
    store: &LocalThreadStore,
    options: RolloutMigrationOptions,
) -> ThreadStoreResult<Box<dyn RolloutMigrationRun>> {
    // Reject invalid options before accepting a job, including multiplication overflow.
    RolloutMigrationRateLimiter::new(options.max_mib_per_second)?;
    let job = Arc::new(Job::new());
    let worker_job = Arc::clone(&job);
    let owned_store = store.clone();
    let mut store_stop = store.maintenance.cancellation();
    store.maintenance.schedule(async move {
        let cancellation = worker_job.cancellation.subscribe();
        let run = owned_store.migrate_rollouts_observed(
            options,
            Observer {
                progress: |progress| worker_job.progress(progress),
                discovered: |total| worker_job.discovered(total),
            },
            RolloutMigrationTrigger::Manual,
            RolloutMigrationPaths::Discover,
            MigrationControl::Cooperative(&cancellation),
        );
        let run = AssertUnwindSafe(run).catch_unwind();
        tokio::pin!(run);
        let result = tokio::select! {
            biased;
            _ = store_stop.wait_for(|stopped| *stopped) => {
                worker_job.cancellation.send_replace(true);
                run.await
            }
            result = &mut run => result,
        };
        match result {
            Ok(result) => {
                // Operation failures are observable in report; the native algorithm
                // has already joined its cleanup, so releasing the lease can succeed.
                worker_job.finish(result, Ok(()));
                Ok(())
            }
            Err(_) => {
                let message = "manual migration worker panicked; cleanup completion is unknown".to_owned();
                let error = ThreadStoreError::Internal { message: message.clone() };
                worker_job.finish(Err(error.clone()), Err(error));
                Err(message)
            }
        }
    })?;
    Ok(Box::new(NativeRun { job }))
}

#[cfg(test)]
#[path = "manual_tests.rs"]
mod tests;
