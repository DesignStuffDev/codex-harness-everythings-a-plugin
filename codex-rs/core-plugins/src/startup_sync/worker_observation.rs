//! Read-only lifecycle diagnostics. Observation never requests stop or joins.

use super::SyncFailure;
use super::WorkerGate;
use super::completion::CuratedSyncNativeCompletion;

/// A bounded category of the generation's immutable operation result.
/// Original errors and their resource obligations remain with the primary owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CuratedSyncOperationDisposition {
    /// Repository sync and configured installed-cache refresh returned success.
    /// The success latch is published before callback dispatch begins.
    Succeeded,
    /// The optional operation failed without a retained uncertainty obligation.
    Failed,
    /// Cooperative stop prevented further admission; this is not a join receipt.
    Stopped { publication_may_have_occurred: bool },
    /// The original failure requires retained custody and explicit recovery.
    Uncertain,
}

/// One coherent snapshot of the current registered generation and gate state.
///
/// This contains no paths, credentials, error payloads or owning handles. It
/// never joins a finished handle, advances callback records, reopens admission,
/// or initializes a worker. A later explicit retry may replace a failed
/// generation; callers must compare `generation` across observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CuratedSyncLifecycleObservation {
    pub generation: Option<u64>,
    pub admission_closed: bool,
    /// Recorded native join disposition, not a fresh OS/process-group check.
    /// A finished but unjoined handle continues to report `Running`.
    pub native: CuratedSyncNativeCompletion,
    /// The exact retained handle's `is_finished` observation, if still attached.
    /// `Some(true)` is not `Joined`; final joining remains the real owner's job.
    pub native_handle_finished: Option<bool>,
    /// `None` means no operation outcome has been published for this snapshot.
    pub operation: Option<CuratedSyncOperationDisposition>,
    pub quarantined: bool,
    pub unexpected_handles: usize,
}

impl WorkerGate {
    pub(crate) fn observe(&self) -> CuratedSyncLifecycleObservation {
        let state = self.lock();
        let record = state.active.as_ref();
        let operation = record.and_then(|record| match &record.outcome {
            None => None,
            Some(Ok(())) => Some(CuratedSyncOperationDisposition::Succeeded),
            Some(Err(SyncFailure::Stopped {
                publication_may_have_occurred,
            })) => Some(CuratedSyncOperationDisposition::Stopped {
                publication_may_have_occurred: *publication_may_have_occurred,
            }),
            Some(Err(error)) => Some(if error.retains_resources() {
                CuratedSyncOperationDisposition::Uncertain
            } else {
                CuratedSyncOperationDisposition::Failed
            }),
        });
        CuratedSyncLifecycleObservation {
            generation: record.map(|record| record.generation.0),
            admission_closed: state.closing,
            native: record.map_or(CuratedSyncNativeCompletion::Idle, |record| {
                record.native_completion
            }),
            native_handle_finished: record.and_then(|record| {
                record
                    .handle
                    .as_ref()
                    .map(std::thread::JoinHandle::is_finished)
            }),
            operation,
            quarantined: !state.unexpected_handles.is_empty()
                || record.is_some_and(|record| record.phase == super::Phase::Quarantined),
            unexpected_handles: state.unexpected_handles.len(),
        }
    }
}

#[cfg(test)]
#[path = "worker_observation_tests.rs"]
mod tests;
