//! Bounded migration leases retain accepted native work through abandoned RPC waiters.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use codex_thread_store::RolloutMigrationReport;
use codex_thread_store::RolloutMigrationRun;
use codex_thread_store::RolloutMigrationSnapshot;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreError;
use codex_thread_store::ThreadStoreResult;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::sync::watch;

use crate::MANUAL_ROLLOUT_MIGRATION_CONTRACT_VERSION;
use crate::StartMigrationRequest;

// Completed RPC slots do not bound retained native reports/handles. Failed and
// release-before-start tombstones share this independent lease budget.
const MAX_MIGRATION_LEASES: usize = 32;
// Spare bounded tombstones let rejected starts receive their paired cleanup.
const MAX_MIGRATION_ENTRIES: usize = MAX_MIGRATION_LEASES + 64;
const MAX_LEASE_ID_BYTES: usize = 128;
const MAX_RETIRED_LEASES: usize = 128;
const UNMATCHED_RELEASE_TIMEOUT: Duration = Duration::from_secs(30);
type Completion = watch::Sender<Option<Result<(), String>>>;

#[derive(Default)]
pub(crate) struct MigrationRegistry {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    closing: bool,
    entries: HashMap<String, Lease>,
    failures: Vec<String>,
    omitted_failures: usize,
    retired: HashMap<String, Result<(), String>>,
    retired_order: VecDeque<String>,
}

enum Lease {
    Preparing {
        completion: Completion,
        released: bool,
    },
    Ready {
        run: Arc<OwnedRun>,
        completion: Completion,
    },
    // The pre-reserved cleanup can be admitted before its paired start handler.
    Released(Completion),
    // Retain failed starts only until their paired cleanup, never a native handle.
    Failed(Completion),
    Closing(Completion),
}

struct OwnedRun {
    run: Box<dyn RolloutMigrationRun>,
    idle: Arc<Notify>,
}

struct Observation {
    // Fields drop in declaration order. The reference must be gone before the
    // trailing guard wakes a close waiter that needs unique ownership.
    run: Arc<OwnedRun>,
    _notify: NotifyAfterObservation,
}

struct NotifyAfterObservation(Arc<Notify>);

impl Observation {
    fn run(&self) -> &dyn RolloutMigrationRun {
        self.run.run.as_ref()
    }
}

impl Drop for NotifyAfterObservation {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

impl MigrationRegistry {
    pub(crate) async fn start(
        &self,
        store: &dyn ThreadStore,
        request: StartMigrationRequest,
    ) -> ThreadStoreResult<()> {
        validate_id(&request.lease_id)?;
        let completion = {
            let mut state = self.state.lock().await;
            if state.retired.contains_key(&request.lease_id) {
                return Err(conflict("migration lease ID has already been retired"));
            }
            match state.entries.remove(&request.lease_id) {
                Some(Lease::Released(completion)) => {
                    retire(&mut state, request.lease_id, Ok(()));
                    completion.send_replace(Some(Ok(())));
                    return Err(conflict("migration was released before native admission"));
                }
                Some(existing) => {
                    state.entries.insert(request.lease_id, existing);
                    return Err(conflict("migration lease ID is already in use"));
                }
                None => {}
            }
            if state.closing {
                return Err(conflict("migration service is shutting down"));
            }
            if state.entries.len() >= MAX_MIGRATION_ENTRIES {
                state.closing = true;
                retire(
                    &mut state,
                    request.lease_id,
                    Err("migration cleanup capacity is exhausted".to_owned()),
                );
                return Err(conflict("migration cleanup capacity is exhausted"));
            }
            let (completion, _) = watch::channel(None);
            let retained = state
                .entries
                .values()
                .filter(|entry| {
                    matches!(
                        entry,
                        Lease::Preparing { .. } | Lease::Ready { .. } | Lease::Closing(_)
                    )
                })
                .count();
            if retained >= MAX_MIGRATION_LEASES {
                state
                    .entries
                    .insert(request.lease_id, Lease::Failed(completion));
                return Err(conflict("migration lease capacity is exhausted"));
            }
            state.entries.insert(
                request.lease_id.clone(),
                Lease::Preparing {
                    completion: completion.clone(),
                    released: false,
                },
            );
            completion
        };

        // Version and native capability are checked before any native work starts.
        let result = if request.contract_version != MANUAL_ROLLOUT_MIGRATION_CONTRACT_VERSION {
            Err(ThreadStoreError::Unsupported {
                operation: "manual_rollout_migration_contract",
            })
        } else if !store.supports_manual_rollout_migration() {
            Err(ThreadStoreError::Unsupported {
                operation: "manual_rollout_migration",
            })
        } else {
            store.start_rollout_migration(request.options).await
        };
        let cleanup = {
            let mut state = self.state.lock().await;
            let previous = state.entries.remove(&request.lease_id);
            let released = matches!(previous, Some(Lease::Preparing { released: true, .. }));
            let closing = state.closing;
            match result {
                Ok(run) => {
                    let run = Arc::new(OwnedRun {
                        run,
                        idle: Arc::new(Notify::new()),
                    });
                    if released || closing {
                        state
                            .entries
                            .insert(request.lease_id.clone(), Lease::Closing(completion.clone()));
                        Some((request.lease_id, run, completion, closing))
                    } else {
                        state
                            .entries
                            .insert(request.lease_id, Lease::Ready { run, completion });
                        None
                    }
                }
                Err(error) => {
                    if released || closing {
                        retire(&mut state, request.lease_id, Ok(()));
                        completion.send_replace(Some(Ok(())));
                    } else {
                        state
                            .entries
                            .insert(request.lease_id, Lease::Failed(completion));
                    }
                    return Err(error);
                }
            }
        };
        if let Some((lease_id, run, completion, closing)) = cleanup {
            let result = close_run(run).await;
            self.complete_close(&lease_id, completion, &result).await;
            result?;
            if closing {
                return Err(conflict("migration service is shutting down"));
            }
        }
        Ok(())
    }

    async fn observe(&self, id: &str) -> ThreadStoreResult<Observation> {
        validate_id(id)?;
        let state = self.state.lock().await;
        if state.closing {
            return Err(conflict("migration service is shutting down"));
        }
        match state.entries.get(id) {
            Some(Lease::Ready { run, .. }) => Ok(Observation {
                run: Arc::clone(run),
                _notify: NotifyAfterObservation(Arc::clone(&run.idle)),
            }),
            _ => Err(conflict("migration lease is not available")),
        }
    }

    pub(crate) async fn snapshot(&self, id: &str) -> ThreadStoreResult<RolloutMigrationSnapshot> {
        self.observe(id).await?.run().snapshot().await
    }

    pub(crate) async fn report(&self, id: &str) -> ThreadStoreResult<RolloutMigrationReport> {
        // Native report is an immediate observation and returns Conflict while active.
        self.observe(id).await?.run().report().await
    }

    pub(crate) async fn cancel(&self, id: &str) -> ThreadStoreResult<()> {
        // This acknowledges only the stop request. Release is the joined close.
        self.observe(id).await?.run().cancel().await
    }

    pub(crate) async fn release(&self, id: &str) -> ThreadStoreResult<()> {
        validate_id(id)?;
        let (completion, run) = {
            let mut state = self.state.lock().await;
            if let Some(result) = state.retired.get(id) {
                return result
                    .clone()
                    .map_err(|message| ThreadStoreError::Internal { message });
            }
            match state.entries.remove(id) {
                Some(Lease::Ready { run, completion }) => {
                    state
                        .entries
                        .insert(id.to_owned(), Lease::Closing(completion.clone()));
                    (completion, Some(run))
                }
                Some(Lease::Preparing { completion, .. }) => {
                    state.entries.insert(
                        id.to_owned(),
                        Lease::Preparing {
                            completion: completion.clone(),
                            released: true,
                        },
                    );
                    (completion, None)
                }
                Some(Lease::Failed(completion)) => {
                    retire(&mut state, id.to_owned(), Ok(()));
                    completion.send_replace(Some(Ok(())));
                    return Ok(());
                }
                Some(Lease::Released(completion)) => {
                    state
                        .entries
                        .insert(id.to_owned(), Lease::Released(completion.clone()));
                    (completion, None)
                }
                Some(Lease::Closing(completion)) => {
                    state
                        .entries
                        .insert(id.to_owned(), Lease::Closing(completion.clone()));
                    (completion, None)
                }
                None if state.closing => return Ok(()),
                None => {
                    if state.entries.len() >= MAX_MIGRATION_ENTRIES {
                        state.closing = true;
                        retire(
                            &mut state,
                            id.to_owned(),
                            Err("migration cleanup capacity is exhausted".to_owned()),
                        );
                        return Err(conflict("migration cleanup capacity is exhausted"));
                    }
                    let (completion, _) = watch::channel(None);
                    state
                        .entries
                        .insert(id.to_owned(), Lease::Released(completion.clone()));
                    (completion, None)
                }
            }
        };
        if let Some(run) = run {
            let result = close_run(run).await;
            self.complete_close(id, completion, &result).await;
            return result;
        }
        let mut result = completion.subscribe();
        let pair_deadline = tokio::time::Instant::now() + UNMATCHED_RELEASE_TIMEOUT;
        loop {
            if let Some(outcome) = result.borrow_and_update().clone() {
                return outcome.map_err(|message| ThreadStoreError::Internal { message });
            }
            let unmatched = matches!(
                self.state.lock().await.entries.get(id),
                Some(Lease::Released(_))
            );
            if unmatched
                && tokio::time::timeout_at(pair_deadline, result.changed())
                    .await
                    .is_err()
            {
                let state = self.state.lock().await;
                if matches!(state.entries.get(id), Some(Lease::Released(_))) {
                    // Keep the pre-release fence until its start arrives or the
                    // session closes. Timeout must never permit late mutation.
                    let error = "migration cleanup did not receive its paired start".to_owned();
                    completion.send_replace(Some(Err(error.clone())));
                    return Err(ThreadStoreError::Internal { message: error });
                }
            } else if !unmatched {
                // Accepted native work has no pairing timeout: acknowledgement
                // waits for its owner to join, within the outer process budget.
                result
                    .changed()
                    .await
                    .map_err(|_| conflict("migration cleanup acknowledgement lost"))?;
            }
        }
    }

    async fn complete_close(
        &self,
        id: &str,
        completion: Completion,
        result: &ThreadStoreResult<()>,
    ) {
        let mut state = self.state.lock().await;
        state.entries.remove(id);
        retire(
            &mut state,
            id.to_owned(),
            result.as_ref().copied().map_err(ToString::to_string),
        );
        if let Err(error) = result {
            record_failure(&mut state, error.to_string());
        }
        completion.send_replace(Some(result.as_ref().copied().map_err(ToString::to_string)));
    }

    /// Fence new migrations and signal accepted ones before the worker drains handlers.
    /// No registry lock is retained across native cancellation or cleanup work.
    pub(crate) async fn begin_shutdown(&self) {
        let runs = {
            let mut state = self.state.lock().await;
            state.closing = true;
            let mut runs = Vec::new();
            state.entries.retain(|_, lease| match lease {
                Lease::Ready { run, .. } => {
                    runs.push(Observation {
                        run: Arc::clone(run),
                        _notify: NotifyAfterObservation(Arc::clone(&run.idle)),
                    });
                    true
                }
                Lease::Preparing { .. } | Lease::Closing(_) => true,
                Lease::Released(completion) | Lease::Failed(completion) => {
                    completion.send_replace(Some(Ok(())));
                    false
                }
            });
            runs
        };
        for run in runs {
            if let Err(error) = run.run().cancel().await {
                record_failure(&mut *self.state.lock().await, error.to_string());
            }
        }
    }

    /// Called only after accepted handlers have joined. Close every retained run,
    /// including completed reports, before closing the store or its state runtime.
    pub(crate) async fn finish_shutdown(&self) -> ThreadStoreResult<()> {
        let entries = {
            let mut state = self.state.lock().await;
            state.closing = true;
            std::mem::take(&mut state.entries)
        };
        for (_, lease) in entries {
            match lease {
                Lease::Ready { run, completion } => {
                    let result = close_run(run).await;
                    if let Err(error) = &result {
                        record_failure(&mut *self.state.lock().await, error.to_string());
                    }
                    completion.send_replace(Some(result.map_err(|error| error.to_string())));
                }
                Lease::Preparing { completion, .. } | Lease::Closing(completion) => {
                    let message = "migration handler ended without confirming cleanup".to_owned();
                    record_failure(&mut *self.state.lock().await, message.clone());
                    completion.send_replace(Some(Err(message)));
                }
                Lease::Released(completion) | Lease::Failed(completion) => {
                    completion.send_replace(Some(Ok(())));
                }
            }
        }
        let state = self.state.lock().await;
        if state.failures.is_empty() && state.omitted_failures == 0 {
            Ok(())
        } else {
            Err(ThreadStoreError::Internal {
                message: format!(
                    "migration cleanup failed: {}; {} additional failures",
                    state.failures.join("; "),
                    state.omitted_failures,
                ),
            })
        }
    }
}

async fn close_run(mut run: Arc<OwnedRun>) -> ThreadStoreResult<()> {
    // Readers admitted before release own their observations until completion.
    // New readers cannot acquire a Closing lease. Only its unique owner closes.
    let cancellation = run.run.cancel().await;
    let idle = Arc::clone(&run.idle);
    loop {
        let notified = idle.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        match Arc::try_unwrap(run) {
            Ok(owned) => {
                let joined = owned.run.close().await;
                return match (cancellation, joined) {
                    (Ok(()), result) | (result, Ok(())) => result,
                    (Err(cancel), Err(close)) => Err(ThreadStoreError::Internal {
                        message: format!(
                            "migration cancellation failed: {cancel}; close failed: {close}"
                        ),
                    }),
                };
            }
            Err(shared) => run = shared,
        }
        notified.await;
    }
}

fn retire(state: &mut State, id: String, result: Result<(), String>) {
    if !state.retired.contains_key(&id) {
        state.retired_order.push_back(id.clone());
    }
    state.retired.insert(id, result);
    while state.retired_order.len() > MAX_RETIRED_LEASES {
        if let Some(oldest) = state.retired_order.pop_front() {
            state.retired.remove(&oldest);
        }
    }
}

fn validate_id(id: &str) -> ThreadStoreResult<()> {
    if id.is_empty() || id.len() > MAX_LEASE_ID_BYTES {
        Err(conflict("invalid migration lease ID"))
    } else {
        Ok(())
    }
}

fn conflict(message: &str) -> ThreadStoreError {
    ThreadStoreError::Conflict {
        message: message.to_owned(),
    }
}

fn record_failure(state: &mut State, message: String) {
    if state.failures.len() < MAX_MIGRATION_LEASES {
        state.failures.push(message.chars().take(1024).collect());
    } else {
        state.omitted_failures = state.omitted_failures.saturating_add(1);
    }
}

#[cfg(test)]
#[path = "migration_service_tests.rs"]
mod tests;
