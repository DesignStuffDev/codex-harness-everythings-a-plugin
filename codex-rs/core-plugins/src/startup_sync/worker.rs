//! Process-wide admission and custody for the native curated-sync worker.
//!
//! A failed worker remains Running until its exact handle has been registered,
//! finished, and joined. Success and uncertain failure latch across all homes.
//! Quarantine has no production reset: recovering it requires an external
//! termination/fencing guarantee, not merely reaping this native worker.

use super::SyncControl;
use super::SyncFailure;
use std::io;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use std::thread::JoinHandle;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Generation(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Running,
    SuccessLatched,
    Quarantined,
}

struct WorkerRecord {
    generation: Generation,
    phase: Phase,
    control: Arc<SyncControl>,
    // The first observation is immutable, including after a later join.
    outcome: Option<Result<(), SyncFailure>>,
    handle: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct State {
    closing: bool,
    next_generation: u64,
    active: Option<WorkerRecord>,
    // Fail closed if a future caller violates the exact-generation attachment
    // protocol. Even an unexpected handle must never be silently detached.
    unexpected_handles: Vec<JoinHandle<()>>,
}

#[derive(Default)]
pub(crate) struct WorkerGate {
    state: Mutex<State>,
}

impl WorkerGate {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn start(
        self: &Arc<Self>,
        work: impl FnOnce(Arc<SyncControl>) -> Result<(), SyncFailure> + Send + 'static,
    ) -> io::Result<bool> {
        self.start_with_spawn(work, |job| {
            std::thread::Builder::new()
                .name("plugins-curated-repo-sync".to_string())
                .spawn(job)
        })
    }

    fn start_with_spawn(
        self: &Arc<Self>,
        work: impl FnOnce(Arc<SyncControl>) -> Result<(), SyncFailure> + Send + 'static,
        spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<JoinHandle<()>>,
    ) -> io::Result<bool> {
        let Some((generation, control)) = self.reserve() else {
            return Ok(false);
        };
        let gate = Arc::clone(self);
        let job = Box::new(move || {
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                if control.is_cancelled() {
                    return Err(SyncFailure::Stopped {
                        publication_may_have_occurred: false,
                    });
                }
                work(control)
            }))
            .unwrap_or_else(|_| {
                tracing::warn!(
                    "curated plugins worker unwound; keeping its generation quarantined"
                );
                Err(SyncFailure::Unwind)
            });
            gate.complete(generation, outcome);
        });
        match spawn(job) {
            Ok(handle) => {
                self.attach(generation, handle);
                Ok(true)
            }
            Err(error) => {
                let retired = {
                    let mut state = self.lock();
                    if state.active.as_ref().is_some_and(|record| {
                        record.generation == generation
                            && record.outcome.is_none()
                            && record.handle.is_none()
                    }) {
                        state.active.take()
                    } else {
                        None
                    }
                };
                drop(retired);
                Err(error)
            }
        }
    }

    pub(crate) fn request_stop(&self, deadline: Instant) {
        let control = {
            let mut state = self.lock();
            state.closing = true;
            state
                .active
                .as_ref()
                .map(|record| Arc::clone(&record.control))
        };
        // Closing is independent of the worker outcome. Signalling can never
        // hold the registry mutex or reopen admission after a later completion.
        if let Some(control) = control {
            control.request_stop(deadline);
        }
    }

    fn reserve(&self) -> Option<(Generation, Arc<SyncControl>)> {
        loop {
            let (generation, handle) = {
                let mut state = self.lock();
                if state.closing || !state.unexpected_handles.is_empty() {
                    return None;
                }
                let Some(record) = state.active.as_mut() else {
                    let next = state.next_generation.checked_add(1)?;
                    state.next_generation = next;
                    let generation = Generation(next);
                    let control = Arc::new(SyncControl::default());
                    state.active = Some(WorkerRecord {
                        generation,
                        phase: Phase::Running,
                        control: Arc::clone(&control),
                        outcome: None,
                        handle: None,
                    });
                    return Some((generation, control));
                };
                if record.phase != Phase::Running
                    || record.outcome.is_none()
                    || !record.handle.as_ref().is_some_and(JoinHandle::is_finished)
                {
                    return None;
                }
                (record.generation, record.handle.take()?)
            };
            // No state mutex is held while joining. A competing admission sees
            // the same Running generation with no handle and cannot replace it.
            let joined = handle.join();
            let mut state = self.lock();
            let record = state.active.as_mut()?;
            if record.generation != generation || record.phase != Phase::Running {
                return None;
            }
            if joined.is_err() {
                record.phase = Phase::Quarantined;
                return None;
            }
            let retired = state.active.take();
            drop(state);
            drop(retired);
        }
    }

    fn complete(&self, generation: Generation, outcome: Result<(), SyncFailure>) {
        let mut state = self.lock();
        let Some(record) = state.active.as_mut() else {
            return;
        };
        if record.generation != generation || record.outcome.is_some() {
            return;
        }
        record.phase = match &outcome {
            Ok(()) => Phase::SuccessLatched,
            Err(error) if error.retains_resources() => Phase::Quarantined,
            Err(_) => Phase::Running,
        };
        record.outcome = Some(outcome);
    }

    fn attach(&self, generation: Generation, handle: JoinHandle<()>) {
        let mut state = self.lock();
        if let Some(record) = state.active.as_mut()
            && record.generation == generation
            && record.handle.is_none()
        {
            record.handle = Some(handle);
            return;
        }
        state.unexpected_handles.push(handle);
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
