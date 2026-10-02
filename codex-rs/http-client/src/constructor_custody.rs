//! Own the route-aware native constructor, not reqwest's internal runtime or sockets.

use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tokio::runtime::Handle;
use tokio::sync::watch;
use tokio::task::JoinError;

mod task;

use crate::RouteAwareClientPoolError;
use crate::client::TransportClient;
use task::Task;

type BuildOutput = Result<TransportClient, RouteAwareClientPoolError>;
const MAX_RETAINED_CONSTRUCTORS: usize = 256;

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HttpConstructionError {
    #[error("HTTP construction admission or publication is closed")]
    Closed,
    #[error("HTTP construction custody capacity is exhausted")]
    CapacityExceeded,
    #[error("HTTP construction runtime is unavailable")]
    RuntimeUnavailable,
    #[error("HTTP construction ownership is unconfirmed")]
    OwnershipUnconfirmed,
}

struct Job {
    native: Task<BuildOutput>,
    disposal: Task<()>,
    cleanup: Task<()>,
    abandoned: AtomicBool,
    delivered: AtomicBool,
    recovery_gate: Mutex<()>,
    recovery_attempts: AtomicUsize,
    changes: watch::Sender<u64>,
}

impl Job {
    fn changed(&self) {
        self.changes
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }

    fn start_cleanup(self: &Arc<Self>) {
        let Ok(runtime) = Handle::try_current() else {
            return;
        };
        let _gate = lock(&self.recovery_gate);
        self.cleanup.try_join();
        self.disposal.try_join();
        if !self.abandoned.load(Ordering::Acquire)
            || self.settled()
            || (self.native.uncertain() && !self.native.joined())
            || self.cleanup.uncertain()
            || self.disposal.uncertain()
        {
            return;
        }
        let recovering = !self.cleanup.idle();
        if recovering {
            if !self.cleanup.joined() {
                return;
            }
            // A running disposer is kept in place. Only a positively joined
            // queued cancellation permits another disposal attempt.
            if self.disposal.cancelled.load(Ordering::Acquire) && !self.disposal.prepare_retry() {
                return;
            }
            if !self.cleanup.prepare_retry() {
                return;
            }
        }
        if recovering {
            self.recovery_attempts.fetch_add(1, Ordering::AcqRel);
        }
        let job = Arc::clone(self);
        let cleanup_runtime = runtime.clone();
        self.cleanup.attach(catch_unwind(AssertUnwindSafe(|| {
            runtime.spawn(async move {
                job.native.join().await;
                if job.native.joined() {
                    if job.disposal.idle() {
                        let disposal_job = Arc::clone(&job);
                        // Admission failure retains output in the native record.
                        job.disposal.attach(catch_unwind(AssertUnwindSafe(|| {
                            cleanup_runtime.spawn_blocking(move || drop(disposal_job.native.take()))
                        })));
                    }
                    job.disposal.join().await;
                }
                job.changed();
            })
        })));
        self.changed();
    }

    fn failed(&self) -> bool {
        self.native.uncertain() || self.disposal.uncertain() || self.cleanup.uncertain()
    }

    fn cancellations(&self) -> usize {
        self.native.cancellations.load(Ordering::Acquire)
            + self.disposal.cancellations.load(Ordering::Acquire)
            + self.cleanup.cancellations.load(Ordering::Acquire)
    }

    fn settled(&self) -> bool {
        self.native.joined()
            && (self.delivered.load(Ordering::Acquire)
                || (self.disposal.succeeded()
                    && self.cleanup.joined()
                    && !self.cleanup.uncertain()))
    }

    async fn settle(self: &Arc<Self>) {
        let mut changes = self.changes.subscribe();
        self.native.join().await;
        loop {
            if self.abandoned.load(Ordering::Acquire) {
                self.start_cleanup();
                self.cleanup.join().await;
                // A later admission/observation can recover another canceled
                // attempt. Never spin retries on a shutting-down runtime.
                return;
            }
            if self.delivered.load(Ordering::Acquire) {
                return;
            }
            if changes.changed().await.is_err() {
                return;
            }
        }
    }
}

pub(super) struct ConstructionTicket(Arc<Job>);

impl ConstructionTicket {
    pub(super) async fn wait(
        self,
    ) -> Result<Result<BuildOutput, JoinError>, HttpConstructionError> {
        self.0.native.join().await;
        let output = self
            .0
            .native
            .take()
            .ok_or(HttpConstructionError::OwnershipUnconfirmed)?;
        self.0.delivered.store(true, Ordering::Release);
        self.0.changed();
        Ok(output)
    }
}

impl Drop for ConstructionTicket {
    fn drop(&mut self) {
        if !self.0.delivered.load(Ordering::Acquire) {
            self.0.abandoned.store(true, Ordering::Release);
            self.0.start_cleanup();
        }
    }
}

struct RegistryState {
    closed: Option<Instant>,
    records: Vec<Arc<Job>>,
    compacted: usize,
    compacted_cancelled: usize,
    compacted_recovery_attempts: usize,
}

pub(super) struct Registry {
    state: Mutex<RegistryState>,
    capacity: usize,
}

impl Registry {
    pub(super) fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(RegistryState {
                closed: None,
                records: Vec::new(),
                compacted: 0,
                compacted_cancelled: 0,
                compacted_recovery_attempts: 0,
            }),
            capacity,
        })
    }

    pub(super) fn begin_close(&self, deadline: Instant) -> Instant {
        *lock(&self.state).closed.get_or_insert(deadline)
    }

    fn compact(state: &mut RegistryState) {
        let before = state.records.len();
        let mut cancelled = 0;
        let mut recovery_attempts = 0;
        state.records.retain(|job| {
            job.native.try_join();
            job.start_cleanup();
            job.cleanup.try_join();
            let retain = !job.settled() || job.failed();
            if !retain {
                cancelled += job.cancellations();
                recovery_attempts += job.recovery_attempts.load(Ordering::Acquire);
            }
            retain
        });
        state.compacted += before - state.records.len();
        state.compacted_cancelled += cancelled;
        state.compacted_recovery_attempts += recovery_attempts;
    }

    fn observation(&self) -> HttpConstructionObservation {
        let mut state = lock(&self.state);
        Self::compact(&mut state);
        HttpConstructionObservation {
            retained: state.records.len(),
            pending: state.records.iter().filter(|job| !job.settled()).count(),
            failed: state.records.iter().filter(|job| job.failed()).count(),
            joined: state.compacted
                + state
                    .records
                    .iter()
                    .filter(|job| job.native.joined())
                    .count(),
            cancelled: state.compacted_cancelled
                + state
                    .records
                    .iter()
                    .map(|job| job.cancellations())
                    .sum::<usize>(),
            recovery_attempts: state.compacted_recovery_attempts
                + state
                    .records
                    .iter()
                    .map(|job| job.recovery_attempts.load(Ordering::Acquire))
                    .sum::<usize>(),
        }
    }

    pub(super) async fn wait_until(
        self: &Arc<Self>,
        deadline: Instant,
    ) -> HttpConstructionObservation {
        let (records, deadline) = {
            let state = lock(&self.state);
            (
                state.records.clone(),
                state.closed.map_or(deadline, |first| deadline.min(first)),
            )
        };
        let waits = records.iter().map(Job::settle);
        let _ = tokio::time::timeout_at(deadline.into(), futures::future::join_all(waits)).await;
        self.observation()
    }
}

fn process_registry() -> &'static Arc<Registry> {
    static REGISTRY: OnceLock<Arc<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Registry::new(MAX_RETAINED_CONSTRUCTORS))
}

#[derive(Clone)]
pub(super) struct PublicationFence {
    registry: Arc<Registry>,
    closed: Arc<Mutex<bool>>,
}

impl PublicationFence {
    /// The callback is synchronous and must not reenter this fence or registry.
    pub(super) fn publish_if_open<T>(
        &self,
        publish: impl FnOnce() -> T,
    ) -> Result<T, HttpConstructionError> {
        let state = lock(&self.registry.state);
        let closed = lock(&self.closed);
        if state.closed.is_some() || *closed {
            return Err(HttpConstructionError::Closed);
        }
        Ok(publish())
    }
}

struct PublicLease(PublicationFence);
impl Drop for PublicLease {
    fn drop(&mut self) {
        *lock(&self.0.closed) = true;
    }
}

#[derive(Clone)]
pub(super) struct PoolLease(Arc<PublicLease>);

impl PoolLease {
    pub(super) fn new() -> Self {
        Self::with_registry(Arc::clone(process_registry()))
    }
    pub(super) fn with_registry(registry: Arc<Registry>) -> Self {
        Self(Arc::new(PublicLease(PublicationFence {
            registry,
            closed: Arc::new(Mutex::new(false)),
        })))
    }
    pub(super) fn publication_fence(&self) -> PublicationFence {
        self.0.0.clone()
    }
    pub(super) fn ensure_open(&self) -> Result<(), HttpConstructionError> {
        self.0.0.publish_if_open(|| ())
    }
    pub(super) fn start(
        &self,
        work: impl FnOnce() -> BuildOutput + Send + 'static,
    ) -> Result<ConstructionTicket, HttpConstructionError> {
        let runtime =
            Handle::try_current().map_err(|_| HttpConstructionError::RuntimeUnavailable)?;
        let mut state = lock(&self.0.0.registry.state);
        let closed = lock(&self.0.0.closed);
        if state.closed.is_some() || *closed {
            return Err(HttpConstructionError::Closed);
        }
        Registry::compact(&mut state);
        if state.records.len() >= self.0.0.registry.capacity {
            return Err(HttpConstructionError::CapacityExceeded);
        }
        let job = Arc::new(Job {
            native: Task::new(),
            disposal: Task::new(),
            cleanup: Task::new(),
            abandoned: AtomicBool::new(false),
            delivered: AtomicBool::new(false),
            recovery_gate: Mutex::new(()),
            recovery_attempts: AtomicUsize::new(0),
            changes: watch::channel(0).0,
        });
        state.records.push(Arc::clone(&job));
        job.native.attach(catch_unwind(AssertUnwindSafe(|| {
            runtime.spawn_blocking(work)
        })));
        Ok(ConstructionTicket(job))
    }
}

/// Constructor custody only: no claim about reqwest tasks, sockets or durability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpConstructionObservation {
    pub retained: usize,
    pub pending: usize,
    pub failed: usize,
    pub joined: usize,
    pub cancelled: usize,
    pub recovery_attempts: usize,
}
impl HttpConstructionObservation {
    pub fn is_complete(self) -> bool {
        self.pending == 0 && self.failed == 0
    }
}

/// Executable-only process fence. Never use for embedded processor replacement.
pub struct HttpConstructionProcessShutdown {
    registry: Arc<Registry>,
    deadline: Instant,
}
impl HttpConstructionProcessShutdown {
    pub fn begin(deadline: Instant) -> Self {
        let registry = Arc::clone(process_registry());
        let deadline = registry.begin_close(deadline);
        Self { registry, deadline }
    }
    pub async fn wait_until(&self, deadline: Instant) -> HttpConstructionObservation {
        self.registry.wait_until(deadline.min(self.deadline)).await
    }
}

#[cfg(test)]
#[path = "constructor_custody_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "constructor_cross_runtime_tests.rs"]
mod cross_runtime_tests;
