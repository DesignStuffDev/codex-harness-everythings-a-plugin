//! Coalesced native admission: retained work lives in one slot, not in wakeups.

use crossbeam_channel::Receiver;
use crossbeam_channel::SendError;
use crossbeam_channel::Sender;
use crossbeam_channel::TrySendError;
use crossbeam_channel::bounded;
use std::sync::Arc;
use std::sync::Mutex;

pub(super) enum WorkSignal {
    QueryUpdated { query: String, query_id: u64 },
    NucleoNotify,
    WalkComplete,
    Shutdown,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct PendingWork {
    pub query: Option<(String, u64)>,
    pub notified: bool,
    pub walk_complete: bool,
    pub shutdown: bool,
}

#[derive(Clone)]
pub(super) struct WorkSender {
    pending: Arc<Mutex<PendingWork>>,
    wake: Sender<()>,
}

impl WorkSender {
    pub fn channel() -> (Self, Receiver<()>) {
        let (wake, receiver) = bounded(1);
        (
            Self {
                pending: Arc::new(Mutex::new(PendingWork::default())),
                wake,
            },
            receiver,
        )
    }

    /// Query IDs are serialized by the session's admission lock. A superseded
    /// query is discarded here; lifecycle flags survive arbitrary query floods.
    pub fn send(&self, signal: WorkSignal) -> Result<(), SendError<()>> {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match signal {
            WorkSignal::QueryUpdated { query, query_id } => {
                pending.query = Some((query, query_id));
            }
            WorkSignal::NucleoNotify => pending.notified = true,
            WorkSignal::WalkComplete => pending.walk_complete = true,
            WorkSignal::Shutdown => pending.shutdown = true,
        }
        // Commit pending work before publishing its wake. Full already means
        // that the receiver will inspect this slot. Never wait for its callback.
        match self.wake.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => Ok(()),
            Err(TrySendError::Disconnected(())) => Err(SendError(())),
        }
    }

    pub fn take(&self) -> PendingWork {
        std::mem::take(
            &mut *self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }
}

#[cfg(test)]
#[path = "work_queue_tests.rs"]
mod tests;
