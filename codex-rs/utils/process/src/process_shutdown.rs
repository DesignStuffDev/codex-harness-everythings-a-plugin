//! Executable-owned process shutdown clock. Embedded replacement must not arm
//! this clock. The independent watchdog remains armed through runtime teardown.

use std::io;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::PoisonError;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

pub const GRACEFUL_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 200);
pub const HARD_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 205);
/// Forced exit initiation; ownership and durability remain unconfirmed.
pub const HARD_TIMEOUT_EXIT_CODE: i32 = 124;
/// No independent watchdog could be created; shutdown is unconfirmed.
pub const WATCHDOG_UNAVAILABLE_EXIT_CODE: i32 = 125;
/// Explicit force bypasses graceful joins; cleanup and durability are unknown.
pub const FORCED_SHUTDOWN_EXIT_CODE: i32 = 126;

#[derive(Clone, Copy, Debug)]
pub struct ProcessShutdownDeadline {
    graceful: Instant,
    hard: Instant,
}

impl ProcessShutdownDeadline {
    pub fn graceful(self) -> Instant { self.graceful }
    pub fn hard(self) -> Instant { self.hard }
}

/// Explicit permission from an executable to start irreversible process-final
/// shutdown. Ordinary library and embedded-provider lifetimes do not create it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessFinalCapability { _private: () }

impl ProcessFinalCapability {
    pub fn for_executable() -> Self { Self { _private: () } }
    pub fn begin(&self) -> ProcessShutdownDeadline { begin() }
}

#[derive(Default)]
struct Clock { watchdog: OnceLock<Arc<Watchdog>> }

impl Clock {
    fn begin_with(
        &self,
        graceful: Duration,
        hard: Duration,
        on_timeout: impl FnOnce() + Send + 'static,
    ) -> ProcessShutdownDeadline {
        self.watchdog.get_or_init(|| {
            Watchdog::start(graceful, hard, on_timeout)
                .unwrap_or_else(|_| std::process::exit(WATCHDOG_UNAVAILABLE_EXIT_CODE))
        }).deadline
    }

    fn finish(&self) -> io::Result<()> {
        self.watchdog.get().map_or(Ok(()), |watchdog| watchdog.finish())
    }
}

static CLOCK: Clock = Clock { watchdog: OnceLock::new() };

/// Start the first shared deadline without depending on Tokio or logging. Later
/// calls reuse it. Failure to create its watchdog is an unconfirmed hard exit.
pub fn begin() -> ProcessShutdownDeadline {
    CLOCK.begin_with(GRACEFUL_TIMEOUT, HARD_TIMEOUT, || std::process::exit(HARD_TIMEOUT_EXIT_CODE))
}

pub fn current() -> Option<ProcessShutdownDeadline> {
    CLOCK.watchdog.get().map(|watchdog| watchdog.deadline)
}

/// Clamp an existing local service budget only after process-final shutdown has
/// begun. This does not arm a process watchdog for an embedded replacement.
pub fn cap_deadline(local: Instant) -> Instant {
    current().map_or(local, |deadline| local.min(deadline.graceful()))
}

/// Call only after the executable's actual runtime has been dropped and its
/// native main thread joined. Early disarming would hide blocked teardown.
pub fn finish_after_runtime() -> io::Result<()> { CLOCK.finish() }

#[derive(Default)]
struct WatchdogState {
    stopped: bool,
    expired: bool,
    joining: bool,
    failed: bool,
    worker: Option<JoinHandle<()>>,
}

struct Watchdog {
    deadline: ProcessShutdownDeadline,
    state: Mutex<WatchdogState>,
    changed: Condvar,
}

impl Watchdog {
    fn start(
        graceful: Duration,
        hard: Duration,
        on_timeout: impl FnOnce() + Send + 'static,
    ) -> io::Result<Arc<Self>> {
        let started = Instant::now();
        let watchdog = Arc::new(Self {
            deadline: ProcessShutdownDeadline { graceful: started + graceful, hard: started + hard },
            state: Mutex::new(WatchdogState::default()),
            changed: Condvar::new(),
        });
        let owner = Arc::clone(&watchdog);
        let worker = std::thread::Builder::new().name("codex-process-shutdown".to_string()).spawn(move || {
            let timed_out = {
                let mut state = owner.state.lock().unwrap_or_else(PoisonError::into_inner);
                loop {
                    if state.stopped { break false; }
                    let now = Instant::now();
                    if now >= owner.deadline.hard {
                        state.expired = true;
                        break true;
                    }
                    let (next, _) = owner.changed.wait_timeout(state, owner.deadline.hard - now)
                        .unwrap_or_else(PoisonError::into_inner);
                    state = next;
                }
            };
            // No logging or user callback runs under the watchdog mutex.
            if timed_out { on_timeout(); }
        })?;
        watchdog.state.lock().unwrap_or_else(PoisonError::into_inner).worker = Some(worker);
        Ok(watchdog)
    }

    fn finish(&self) -> io::Result<()> {
        let worker = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if !state.stopped {
                state.expired |= Instant::now() >= self.deadline.hard;
                state.stopped = true;
                self.changed.notify_all();
            }
            while state.joining {
                state = self.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
            }
            let worker = state.worker.take();
            state.joining = worker.is_some();
            worker
        };
        if let Some(worker) = worker {
            let joined = worker.join();
            {
                let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
                state.failed |= joined.is_err();
                state.joining = false;
                self.changed.notify_all();
            }
            drop(joined);
        }
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.expired {
            Err(io::Error::new(io::ErrorKind::TimedOut, "process shutdown exceeded its independent hard deadline; cleanup is unconfirmed"))
        } else if state.failed {
            Err(io::Error::other("process shutdown watchdog failed; cleanup is unconfirmed"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "process_shutdown_tests.rs"]
mod tests;
