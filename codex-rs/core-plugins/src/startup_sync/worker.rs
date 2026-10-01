//! Process-wide admission and custody for the native curated-sync worker.
//!
//! A failed worker remains Running until its exact handle has been registered,
//! finished, and joined. Success and uncertain failure latch across all homes.
//! Quarantine has no production reset: recovering it requires an external
//! termination/fencing guarantee, not merely reaping this native worker.

use super::SyncFailure;
use std::io;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use std::thread::JoinHandle;

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
    // The first observation is immutable, including after a later join.
    outcome: Option<Result<(), SyncFailure>>,
    handle: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct State {
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
        work: impl FnOnce() -> Result<(), SyncFailure> + Send + 'static,
    ) -> io::Result<bool> {
        self.start_with_spawn(work, |job| {
            std::thread::Builder::new()
                .name("plugins-curated-repo-sync".to_string())
                .spawn(job)
        })
    }

    fn start_with_spawn(
        self: &Arc<Self>,
        work: impl FnOnce() -> Result<(), SyncFailure> + Send + 'static,
        spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<JoinHandle<()>>,
    ) -> io::Result<bool> {
        let Some(generation) = self.reserve() else {
            return Ok(false);
        };
        let gate = Arc::clone(self);
        let job = Box::new(move || {
            let outcome = catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|_| {
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

    fn reserve(&self) -> Option<Generation> {
        loop {
            let (generation, handle) = {
                let mut state = self.lock();
                if !state.unexpected_handles.is_empty() {
                    return None;
                }
                let Some(record) = state.active.as_mut() else {
                    let next = state.next_generation.checked_add(1)?;
                    state.next_generation = next;
                    let generation = Generation(next);
                    state.active = Some(WorkerRecord {
                        generation,
                        phase: Phase::Running,
                        outcome: None,
                        handle: None,
                    });
                    return Some(generation);
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
