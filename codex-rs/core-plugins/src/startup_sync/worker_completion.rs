//! Native-worker observation only; callback drain belongs to a separate scope.

use super::Phase;
use super::SyncFailure;
use super::WorkerGate;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

/// The exact native handle's disposition, not a descendant or durability proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CuratedSyncNativeCompletion {
    Idle,
    AwaitingHandle,
    Running,
    Joining,
    Joined,
    Panicked,
    SpawnFailed { kind: io::ErrorKind },
}

/// A non-owning snapshot. The gate retains the original typed sync outcome.
#[derive(Clone, Debug)]
pub struct CuratedSyncWorkerObservation {
    pub generation: Option<u64>,
    pub native: CuratedSyncNativeCompletion,
    pub quarantined: bool,
    pub unexpected_handles: usize,
    outcome: Option<Result<(), SyncFailure>>,
}

impl CuratedSyncWorkerObservation {
    /// Whether native handle observation is still incomplete.
    /// A false result does not erase quarantine or drain callback tasks.
    pub fn is_pending(&self) -> bool {
        self.unexpected_handles != 0
            || matches!(
                self.native,
                CuratedSyncNativeCompletion::AwaitingHandle
                    | CuratedSyncNativeCompletion::Running
                    | CuratedSyncNativeCompletion::Joining
            )
    }

    /// The immutable sync failure, preserved separately from native join status.
    pub fn failure(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.outcome
            .as_ref()?
            .as_ref()
            .err()
            .map(|error| error as &dyn std::error::Error)
    }

    pub fn sync_succeeded(&self) -> bool {
        matches!(&self.outcome, Some(Ok(())))
    }
}

/// Observer for irreversible process-final admission closure.
/// Dropping this object or a pending wait never detaches the native worker.
#[derive(Clone)]
pub struct CuratedSyncStop {
    gate: Arc<WorkerGate>,
}

impl CuratedSyncStop {
    /// Checks the observer deadline between completion observations, without
    /// extending the stop deadline. Expiry preserves primary gate custody.
    ///
    /// Only an already-finished handle is taken for synchronous join, with no
    /// await between take and result publication. Native final thread teardown
    /// can still delay that join; this is not a hard wall-clock return bound.
    /// Pending HTTP runtime teardown likewise remains owned by the worker.
    pub async fn wait_until(&self, deadline: Instant) -> CuratedSyncWorkerObservation {
        loop {
            let observation = self.gate.poll_stop();
            if !observation.is_pending() || Instant::now() >= deadline {
                return observation;
            }
            let wake_at = deadline.min(Instant::now() + Duration::from_millis(/*millis*/ 10));
            tokio::time::sleep_until(wake_at.into()).await;
        }
    }
}

impl WorkerGate {
    pub(crate) fn begin_stop(self: &Arc<Self>, deadline: Instant) -> CuratedSyncStop {
        self.request_stop(deadline);
        CuratedSyncStop {
            gate: Arc::clone(self),
        }
    }

    fn poll_stop(&self) -> CuratedSyncWorkerObservation {
        let finished = {
            let mut state = self.lock();
            state.active.as_mut().and_then(|record| {
                if record.native_completion == CuratedSyncNativeCompletion::Running
                    && record
                        .handle
                        .as_ref()
                        .is_some_and(std::thread::JoinHandle::is_finished)
                {
                    let handle = record.handle.take()?;
                    record.native_completion = CuratedSyncNativeCompletion::Joining;
                    Some((record.generation, handle))
                } else {
                    None
                }
            })
        };
        if let Some((generation, handle)) = finished {
            // The gate remains closed throughout. Other observers see Joining
            // and can never acquire or discard this exact handle.
            let joined = handle.join();
            let mut state = self.lock();
            if let Some(record) = state.active.as_mut()
                && record.generation == generation
            {
                record.native_completion = if joined.is_ok() {
                    CuratedSyncNativeCompletion::Joined
                } else {
                    record.phase = Phase::Quarantined;
                    CuratedSyncNativeCompletion::Panicked
                };
            }
            // `joined` may contain an arbitrary panic payload. Drop the mutex
            // before that payload, as well as before joining the native thread.
            drop(state);
        }
        let state = self.lock();
        CuratedSyncWorkerObservation {
            generation: state.active.as_ref().map(|record| record.generation.0),
            native: state
                .active
                .as_ref()
                .map_or(CuratedSyncNativeCompletion::Idle, |record| {
                    record.native_completion
                }),
            quarantined: !state.unexpected_handles.is_empty()
                || state
                    .active
                    .as_ref()
                    .is_some_and(|record| record.phase == Phase::Quarantined),
            unexpected_handles: state.unexpected_handles.len(),
            outcome: state
                .active
                .as_ref()
                .and_then(|record| record.outcome.clone()),
        }
    }
}

#[cfg(test)]
#[path = "worker_completion_tests.rs"]
mod tests;
