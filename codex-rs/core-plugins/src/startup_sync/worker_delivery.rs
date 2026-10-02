//! Successful sync is delivered once to each registered same-home scope.
//! The worker state mutex linearizes registration and immutable completion;
//! dispatch and destruction of arbitrary callback captures happen after unlock.

use super::Phase;
use crate::startup_sync::CuratedCallbackScope;
use crate::startup_sync::CuratedSyncCallback;
use crate::startup_sync::SyncControl;
use std::sync::Arc;
use std::path::PathBuf;
use std::sync::Weak;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HomeAdmission {
    Started,
    Subscribed,
    Replayed,
    AlreadyRegistered,
    // This single native worker retains the existing process-wide success
    // latch. A different lexical absolute home cannot consume its result.
    UnsupportedHome,
    Closed,
    Unavailable,
}

pub(super) struct Registration {
    pub(super) home: PathBuf,
    pub(super) callback: Option<CuratedSyncCallback>,
}

impl Registration {
    pub(super) fn discard(self) -> Option<AfterUnlock> {
        self.callback.map(AfterUnlock::Discard)
    }
}

pub(super) enum AfterUnlock {
    Dispatch(CuratedSyncCallback, Arc<SyncControl>),
    Discard(CuratedSyncCallback),
}

impl AfterUnlock {
    pub(super) fn run(self) {
        match self {
            Self::Dispatch(callback, control) => {
                if !control.is_cancelled() { callback.dispatch(); }
            }
            Self::Discard(callback) => drop(callback),
        }
    }
}

pub(super) struct Delivery {
    home: PathBuf,
    control: Arc<SyncControl>,
    pending: Vec<CuratedSyncCallback>,
    seen: Vec<Weak<CuratedCallbackScope>>,
}

impl Delivery {
    pub(super) fn new(registration: Registration, control: Arc<SyncControl>) -> Self {
        let mut delivery = Self {
            home: registration.home,
            control,
            pending: Vec::new(),
            seen: Vec::new(),
        };
        if let Some(callback) = registration.callback {
            delivery.seen.push(callback.scope_identity());
            delivery.pending.push(callback);
        }
        delivery
    }

    pub(super) fn register(
        &mut self,
        registration: Registration,
        phase: Phase,
        has_outcome: bool,
    ) -> (HomeAdmission, Option<AfterUnlock>) {
        if registration.home != self.home {
            return (HomeAdmission::UnsupportedHome, registration.discard());
        }
        if phase == Phase::Quarantined || (phase == Phase::Running && has_outcome) {
            return (HomeAdmission::Unavailable, registration.discard());
        }
        let Some(callback) = registration.callback else {
            return (HomeAdmission::AlreadyRegistered, None);
        };
        let identity = callback.scope_identity();
        self.seen.retain(|scope| scope.strong_count() != 0);
        if self.seen.iter().any(|scope| Weak::ptr_eq(scope, &identity)) {
            return (HomeAdmission::AlreadyRegistered, Some(AfterUnlock::Discard(callback)));
        }
        self.seen.push(identity);
        if phase == Phase::SuccessLatched {
            (HomeAdmission::Replayed, Some(AfterUnlock::Dispatch(callback, Arc::clone(&self.control))))
        } else {
            self.pending.push(callback);
            (HomeAdmission::Subscribed, None)
        }
    }

    pub(super) fn take_pending(&mut self) -> Vec<CuratedSyncCallback> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
#[path = "worker_delivery_tests.rs"]
mod tests;
