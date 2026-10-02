//! Process-final observation of the curated native worker and owned callbacks.
//! Processor replacement must close only its own callback scope instead.

use super::callback_scope::CuratedCallbackDrain;
use super::callback_scope::CuratedCallbackObservation;
use super::callback_scope::begin_process_close;
use super::worker::completion::CuratedSyncNativeCompletion;
use super::worker::completion::CuratedSyncStop;
use super::worker::completion::CuratedSyncWorkerObservation;
use crate::manager::PluginsManager;
use std::time::Instant;

/// Irreversible admission closure with a shared, externally owned deadline.
/// Dropping this observer never releases primary custody of pending work.
pub struct CuratedProcessShutdown {
    native: CuratedSyncStop,
    callbacks: CuratedCallbackDrain,
    deadline: Instant,
}

impl CuratedProcessShutdown {
    /// Fence callback registration and dispatch, then stop the native worker.
    /// The host must supply its existing process-final deadline and keep the
    /// callback runtimes alive during observation. This adds no timeout budget.
    pub fn begin(deadline: Instant) -> Self {
        let callbacks = begin_process_close();
        let native = PluginsManager::begin_curated_repo_sync_process_stop(deadline);
        Self {
            native,
            callbacks,
            deadline,
        }
    }

    /// Observe both owners concurrently, without extending the begin deadline.
    /// Expiry or cancellation retains primary custody. Native thread joining
    /// can block beyond the observation deadline; the host's independent native
    /// watchdog must bound process exit. This is not a hard return-time promise.
    pub async fn wait_until(&self, deadline: Instant) -> CuratedProcessShutdownObservation {
        let deadline = deadline.min(self.deadline);
        let (native, callbacks) = tokio::join!(
            self.native.wait_until(deadline),
            self.callbacks.wait_until(deadline),
        );
        let observed = CuratedProcessShutdownObservation { native, callbacks };
        let worker_disposition = match observed.native.native {
            CuratedSyncNativeCompletion::Idle => "idle",
            CuratedSyncNativeCompletion::AwaitingHandle => "awaiting_handle",
            CuratedSyncNativeCompletion::Running => "running",
            CuratedSyncNativeCompletion::Joining => "joining",
            CuratedSyncNativeCompletion::Joined => "joined",
            CuratedSyncNativeCompletion::Panicked => "panicked",
            CuratedSyncNativeCompletion::SpawnFailed { .. } => "spawn_failed",
        };
        let optional_sync_outcome = if observed.native.sync_succeeded() {
            "succeeded"
        } else if observed.native.failure().is_some() {
            "failed"
        } else {
            "not_observed"
        };
        tracing::info!(
            target: "codex_core_plugins::lifecycle",
            event = "curated_process_shutdown_observed",
            receipt_version = 1_u64,
            worker_generation = observed.native.generation.map_or(0, |generation| generation),
            worker_disposition,
            worker_quarantined = observed.native.quarantined,
            worker_unexpected_handles = observed.native.unexpected_handles,
            callback_pending = observed.callbacks.pending,
            callback_completed = observed.callbacks.completed,
            callback_suppressed = observed.callbacks.suppressed,
            callback_failed = observed.callbacks.failed,
            curated_ownership_clean = observed.is_complete(),
            optional_sync_outcome,
            "curated process shutdown observed"
        );
        observed
    }
}

/// Cleanup observations remain separate from the operation being shut down.
/// Callers must preserve their original operation error when reporting cleanup.
/// This does not cover unrelated plugin tasks, detached descendants, or durable
/// repository publication. Original callback failures remain in their registry.
#[derive(Clone, Debug)]
pub struct CuratedProcessShutdownObservation {
    pub native: CuratedSyncWorkerObservation,
    pub callbacks: CuratedCallbackObservation,
}

impl CuratedProcessShutdownObservation {
    /// Repository sync itself is optional. An ordinary or stopped outcome is
    /// nonfatal here when exact native completion and clean custody are known.
    /// A joined handle without any observed outcome is inconsistent, not clean.
    pub fn clean_native_ownership(&self) -> bool {
        if self.native.quarantined || self.native.unexpected_handles != 0 {
            return false;
        }
        match self.native.native {
            CuratedSyncNativeCompletion::Idle | CuratedSyncNativeCompletion::SpawnFailed { .. } => true,
            CuratedSyncNativeCompletion::Joined => {
                self.native.sync_succeeded() || self.native.failure().is_some()
            }
            CuratedSyncNativeCompletion::AwaitingHandle
            | CuratedSyncNativeCompletion::Running
            | CuratedSyncNativeCompletion::Joining
            | CuratedSyncNativeCompletion::Panicked => false,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.clean_native_ownership() && self.callbacks.is_complete()
    }
}

#[cfg(test)]
#[path = "process_shutdown_tests.rs"]
mod tests;
