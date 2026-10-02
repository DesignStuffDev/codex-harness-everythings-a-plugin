//! Process-wide admission and custody for the native curated-sync worker.
//!
//! A failed worker remains Running until its exact handle has been registered,
//! finished, and joined. Success and uncertain failure latch across all homes.
//! Quarantine has no production reset: recovering it requires an external
//! termination/fencing guarantee, not merely reaping this native worker.

#[path = "worker_completion.rs"]
pub(crate) mod completion;

#[path = "worker_observation.rs"]
pub(crate) mod observation;

#[path = "worker_delivery.rs"]
mod delivery;
pub(crate) use delivery::HomeAdmission;
use delivery::Delivery;
use delivery::AfterUnlock;
use delivery::Registration;

use super::CuratedSyncCallback;
use super::SyncControl;
use super::SyncFailure;
use completion::CuratedSyncNativeCompletion;
use std::io;
use std::path::PathBuf;
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
    native_completion: CuratedSyncNativeCompletion,
    delivery: Option<Delivery>,
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

enum Reservation {
    Start(Generation, Arc<SyncControl>),
    Existing(HomeAdmission, Option<AfterUnlock>),
}

#[derive(Default)]
pub(crate) struct WorkerGate {
    state: Mutex<State>,
}

impl WorkerGate {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[cfg(test)]
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

    #[cfg(test)]
    fn start_with_spawn(
        self: &Arc<Self>,
        work: impl FnOnce(Arc<SyncControl>) -> Result<(), SyncFailure> + Send + 'static,
        spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<JoinHandle<()>>,
    ) -> io::Result<bool> {
        self.start_registered_with_spawn(/*registration*/ None, work, spawn)
            .map(|admission| admission == HomeAdmission::Started)
    }

    pub(crate) fn start_for_home(
        self: &Arc<Self>,
        home: PathBuf,
        callback: Option<CuratedSyncCallback>,
        work: impl FnOnce(Arc<SyncControl>) -> Result<(), SyncFailure> + Send + 'static,
    ) -> io::Result<HomeAdmission> {
        self.start_registered_with_spawn(Some(Registration { home, callback }), work, |job| {
            std::thread::Builder::new()
                .name("plugins-curated-repo-sync".to_string())
                .spawn(job)
        })
    }

    fn start_registered_with_spawn(
        self: &Arc<Self>,
        registration: Option<Registration>,
        work: impl FnOnce(Arc<SyncControl>) -> Result<(), SyncFailure> + Send + 'static,
        spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<JoinHandle<()>>,
    ) -> io::Result<HomeAdmission> {
        let (generation, control) = match self.reserve(registration) {
            Reservation::Start(generation, control) => (generation, control),
            Reservation::Existing(admission, dispatch) => {
                // Registration and completion linearize under the worker mutex;
                // callback scheduling and arbitrary capture destruction do not.
                if let Some(action) = dispatch { action.run(); }
                return Ok(admission);
            }
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
                Ok(HomeAdmission::Started)
            }
            Err(error) => {
                let retired = {
                    let mut state = self.lock();
                    if state.active.as_ref().is_some_and(|record| {
                        record.generation == generation
                            && record.outcome.is_none()
                            && record.handle.is_none()
                    }) {
                        if state.closing {
                            if let Some(record) = state.active.as_mut() {
                                record.native_completion =
                                    CuratedSyncNativeCompletion::SpawnFailed { kind: error.kind() };
                            }
                            None
                        } else {
                            state.active.take()
                        }
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

    fn reserve(&self, mut registration: Option<Registration>) -> Reservation {
        loop {
            let (generation, handle) = {
                let mut state = self.lock();
                if state.closing || !state.unexpected_handles.is_empty() {
                    return Reservation::Existing(HomeAdmission::Closed, None);
                }
                let Some(record) = state.active.as_mut() else {
                    let Some(next) = state.next_generation.checked_add(1) else {
                        return Reservation::Existing(HomeAdmission::Unavailable, None);
                    };
                    state.next_generation = next;
                    let generation = Generation(next);
                    let control = Arc::new(SyncControl::default());
                    state.active = Some(WorkerRecord {
                        generation,
                        phase: Phase::Running,
                        control: Arc::clone(&control),
                        outcome: None,
                        handle: None,
                        native_completion: CuratedSyncNativeCompletion::AwaitingHandle,
                        delivery: registration.take().map(|registration| Delivery::new(registration, Arc::clone(&control))),
                    });
                    return Reservation::Start(generation, control);
                };
                let retryable = record.phase == Phase::Running
                    && record.outcome.is_some()
                    && record.handle.as_ref().is_some_and(JoinHandle::is_finished);
                if !retryable {
                    let Some(registration) = registration.take() else {
                        return Reservation::Existing(HomeAdmission::Unavailable, None);
                    };
                    let Some(delivery) = &mut record.delivery else {
                        return Reservation::Existing(HomeAdmission::Unavailable, registration.discard());
                    };
                    let result = delivery.register(registration, record.phase, record.outcome.is_some());
                    return Reservation::Existing(result.0, result.1);
                }
                record.native_completion = CuratedSyncNativeCompletion::Joining;
                let Some(handle) = record.handle.take() else {
                    return Reservation::Existing(HomeAdmission::Unavailable, None);
                };
                (record.generation, handle)
            };
            let joined = handle.join();
            let mut state = self.lock();
            let Some(record) = state.active.as_mut() else {
                return Reservation::Existing(HomeAdmission::Unavailable, None);
            };
            if record.generation != generation || record.phase != Phase::Running {
                return Reservation::Existing(HomeAdmission::Unavailable, None);
            }
            record.native_completion = if joined.is_ok() {
                CuratedSyncNativeCompletion::Joined
            } else {
                record.phase = Phase::Quarantined;
                CuratedSyncNativeCompletion::Panicked
            };
            if joined.is_err() || state.closing {
                return Reservation::Existing(HomeAdmission::Closed, None);
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
        let succeeded = outcome.is_ok();
        let control = Arc::clone(&record.control);
        record.outcome = Some(outcome);
        let callbacks = record.delivery.as_mut().map(Delivery::take_pending).unwrap_or_default();
        drop(state);
        // Closed scopes reject late delivery; each admitted action is owned by
        // its current processor, never by the manager which started this thread.
        if succeeded {
            for callback in callbacks {
                if !control.is_cancelled() { callback.dispatch(); }
            }
        }
    }

    fn attach(&self, generation: Generation, handle: JoinHandle<()>) {
        let mut state = self.lock();
        if let Some(record) = state.active.as_mut()
            && record.generation == generation
            && record.handle.is_none()
            && record.native_completion == CuratedSyncNativeCompletion::AwaitingHandle
        {
            record.native_completion = CuratedSyncNativeCompletion::Running;
            record.handle = Some(handle);
            return;
        }
        state.unexpected_handles.push(handle);
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
