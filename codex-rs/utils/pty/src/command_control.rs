//! Shared owner stop requests. A request is not proof of process completion.
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

/// Cancellation observations used by bounded commands. Implementations must not
/// require the command-state mutex. A deadline limits cleanup, not just execution.
pub trait CommandCancellation: Send + Sync {
    fn is_cancelled(&self) -> bool;
    fn deadline(&self) -> Option<Instant> {
        None
    }
    fn wait_for_change(&self, timeout: Duration) {
        std::thread::sleep(timeout);
    }
}
impl CommandCancellation for AtomicBool {
    fn is_cancelled(&self) -> bool {
        self.load(Ordering::Acquire)
    }
}

/// One owner's monotonic stop request, shared with its worker and transports.
/// The only stop reason is an owner request. It cannot be reset, and later or
/// out-of-order requests cannot extend the earliest absolute cleanup deadline.
/// Requesting stop wakes polling waits; it does not join or establish shutdown.
#[derive(Default)]
pub struct CommandControl {
    cancelled: AtomicBool,
    deadline: Mutex<Option<Instant>>,
    changed: Condvar,
}
impl CommandControl {
    pub fn request_stop(&self, deadline: Instant) {
        {
            let mut current = self.deadline.lock().unwrap_or_else(PoisonError::into_inner);
            *current = Some(current.map_or(deadline, |old| old.min(deadline)));
            self.cancelled.store(true, Ordering::Release);
        }
        self.changed.notify_all();
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
    pub fn deadline(&self) -> Option<Instant> {
        *self.deadline.lock().unwrap_or_else(PoisonError::into_inner)
    }
    pub fn wait_for_change(&self, timeout: Duration) {
        let current = self.deadline.lock().unwrap_or_else(PoisonError::into_inner);
        let observed = *current;
        let waited = self
            .changed
            .wait_timeout_while(current, timeout, |value| *value == observed)
            .unwrap_or_else(PoisonError::into_inner);
        drop(waited);
    }
}
impl CommandCancellation for CommandControl {
    fn is_cancelled(&self) -> bool {
        Self::is_cancelled(self)
    }
    fn deadline(&self) -> Option<Instant> {
        Self::deadline(self)
    }
    fn wait_for_change(&self, timeout: Duration) {
        Self::wait_for_change(self, timeout);
    }
}
