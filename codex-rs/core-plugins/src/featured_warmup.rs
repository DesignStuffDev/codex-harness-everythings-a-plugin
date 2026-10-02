//! Owns only the featured startup future and its scoped cache publication.
//! Joining it does not prove HTTP-internal tasks or native work have stopped.

use std::any::Any;
use std::future::Future;
use std::future::poll_fn;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::sync::Weak;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Instant;
use tokio::runtime::Handle;
use tokio::sync::watch;
use tokio::task::JoinError;
use tokio::task::JoinHandle;

static SCOPES: OnceLock<Arc<Registry>> = OnceLock::new();
const MAX_RETAINED_SCOPES: usize = 256;

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeaturedWarmupOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeaturedWarmupAdmission {
    Started,
    Closed,
    CapacityExceeded,
    AlreadyStarted,
    SpawnFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeaturedWarmupOwnership {
    Idle,
    Pending,
    Joined,
    Panicked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeaturedWarmupObservation {
    pub ownership: FeaturedWarmupOwnership,
    pub outcome: Option<FeaturedWarmupOutcome>,
}

impl FeaturedWarmupObservation {
    pub fn is_complete(self) -> bool {
        self.ownership == FeaturedWarmupOwnership::Idle
            || (self.ownership == FeaturedWarmupOwnership::Joined && self.outcome.is_some())
    }
}

enum Record {
    Idle,
    AwaitingHandle,
    Running(JoinHandle<FeaturedWarmupOutcome>),
    Joined(FeaturedWarmupOutcome),
    JoinFailed(JoinError),
    SpawnPanicked { _payload: Box<dyn Any + Send> },
}

struct State {
    closed: bool,
    record: Record,
    attachment_waker: Option<Waker>,
}

impl State {
    fn observation(&self) -> FeaturedWarmupObservation {
        use FeaturedWarmupOwnership::Idle;
        use FeaturedWarmupOwnership::Joined;
        use FeaturedWarmupOwnership::Panicked;
        use FeaturedWarmupOwnership::Pending;
        let (ownership, outcome) = match &self.record {
            Record::Idle => (Idle, None),
            Record::AwaitingHandle | Record::Running(_) => (Pending, None),
            Record::Joined(outcome) => (Joined, Some(*outcome)),
            Record::JoinFailed(error) if error.is_panic() => (Panicked, None),
            Record::JoinFailed(_) => (Joined, None),
            Record::SpawnPanicked { .. } => (Panicked, None),
        };
        FeaturedWarmupObservation { ownership, outcome }
    }
}

struct Registry {
    closing: AtomicBool,
    scopes: Mutex<Vec<Arc<FeaturedWarmupScope>>>,
    capacity: usize,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            closing: AtomicBool::new(false),
            scopes: Mutex::new(Vec::new()),
            capacity: MAX_RETAINED_SCOPES,
        }
    }
}

impl Registry {
    fn begin_close(&self) -> FeaturedWarmupDrain {
        let scopes = {
            let scopes = lock(&self.scopes);
            self.closing.store(true, Ordering::Release);
            scopes.clone()
        };
        for scope in &scopes {
            scope.begin_close();
        }
        FeaturedWarmupDrain { scopes }
    }
}

/// One processor's startup warmup. Explicit lifetime guards close this scope.
/// Its registry retains primary custody when a drain observer is canceled.
pub struct FeaturedWarmupScope {
    runtime: Handle,
    registry: Weak<Registry>,
    state: Mutex<State>,
    stopped: watch::Sender<bool>,
    // A single observer may poll the stored handle; this holds no shared data.
    waiter: tokio::sync::Semaphore,
    capacity_denied: bool,
}

impl FeaturedWarmupScope {
    pub fn new() -> Arc<Self> {
        Self::with_registry(Handle::current(), SCOPES.get_or_init(Default::default))
    }

    fn with_registry(runtime: Handle, registry: &Arc<Registry>) -> Arc<Self> {
        let mut scopes = lock(&registry.scopes);
        let closed = registry.closing.load(Ordering::Acquire);
        // Failed/unconfirmed entries keep their slots; never evict primary custody.
        let capacity_denied = !closed && scopes.len() >= registry.capacity;
        let scope = Arc::new(Self {
            runtime,
            registry: Arc::downgrade(registry),
            state: Mutex::new(State {
                closed,
                record: Record::Idle,
                attachment_waker: None,
            }),
            stopped: watch::channel(closed).0,
            waiter: tokio::sync::Semaphore::new(/*permits*/ 1),
            capacity_denied,
        });
        if !closed && !capacity_denied {
            scopes.push(Arc::clone(&scope));
        }
        scope
    }

    pub(crate) fn start<F>(self: &Arc<Self>, work: F) -> FeaturedWarmupAdmission
    where
        F: Future<Output = FeaturedWarmupOutcome> + Send + 'static,
    {
        {
            let mut state = lock(&self.state);
            if state.closed || self.process_closed() {
                return FeaturedWarmupAdmission::Closed;
            }
            if self.capacity_denied {
                return FeaturedWarmupAdmission::CapacityExceeded;
            }
            if !matches!(state.record, Record::Idle) {
                return FeaturedWarmupAdmission::AlreadyStarted;
            }
            state.record = Record::AwaitingHandle;
        }
        let mut stopped = self.stopped.subscribe();
        let spawned = catch_unwind(AssertUnwindSafe(|| {
            self.runtime.spawn(async move {
                let cancellation = async {
                    loop {
                        if *stopped.borrow_and_update() {
                            return;
                        }
                        if stopped.changed().await.is_err() {
                            return;
                        }
                    }
                };
                tokio::select! {
                    biased;
                    _ = cancellation => FeaturedWarmupOutcome::Cancelled,
                    outcome = work => outcome,
                }
            })
        }));
        let (record, admission) = match spawned {
            Ok(handle) => (Record::Running(handle), FeaturedWarmupAdmission::Started),
            Err(payload) => (
                Record::SpawnPanicked { _payload: payload },
                FeaturedWarmupAdmission::SpawnFailed,
            ),
        };
        let wake = {
            let mut state = lock(&self.state);
            // Only this starter can replace AwaitingHandle; closing never takes it.
            state.record = record;
            state.attachment_waker.take()
        };
        if let Some(wake) = wake {
            wake.wake();
        }
        admission
    }

    fn process_closed(&self) -> bool {
        self.registry
            .upgrade()
            .is_none_or(|registry| registry.closing.load(Ordering::Acquire))
    }

    /// The closure must be synchronous and must not reenter this scope.
    /// Only the guarded publication is ordered against close, not cache invalidation.
    pub(crate) fn publish_if_open<T>(&self, publish: impl FnOnce() -> T) -> Option<T> {
        let state = lock(&self.state);
        if state.closed || self.capacity_denied || self.process_closed() {
            return None;
        }
        Some(publish())
    }

    pub fn begin_close(&self) {
        lock(&self.state).closed = true;
        self.stopped.send_replace(true);
        self.retire_if_clean();
    }

    pub async fn wait(&self) -> FeaturedWarmupObservation {
        self.wait_inner(None).await
    }

    /// Bounds async observation, not time waiting for synchronous publication locks.
    /// A timed-out/canceled observer leaves the exact handle in this scope.
    pub async fn wait_until(&self, deadline: Instant) -> FeaturedWarmupObservation {
        self.wait_inner(Some(deadline)).await
    }

    async fn wait_inner(&self, deadline: Option<Instant>) -> FeaturedWarmupObservation {
        self.begin_close();
        let waiter = match deadline {
            Some(end) => tokio::select! {
                biased;
                permit = self.waiter.acquire() => permit,
                _ = tokio::time::sleep_until(end.into()) => return lock(&self.state).observation(),
            },
            None => self.waiter.acquire().await,
        };
        // The private semaphore is never closed. If that invariant changes,
        // report current custody rather than polling without a permit.
        let Ok(waiter) = waiter else {
            return lock(&self.state).observation();
        };
        let observed = match deadline {
            Some(end) => tokio::select! {
                biased;
                observed = poll_fn(|cx| self.poll_completion(cx)) => observed,
                _ = tokio::time::sleep_until(end.into()) => lock(&self.state).observation(),
            },
            None => poll_fn(|cx| self.poll_completion(cx)).await,
        };
        drop(waiter);
        self.retire_if_clean();
        observed
    }

    fn poll_completion(&self, cx: &mut Context<'_>) -> Poll<FeaturedWarmupObservation> {
        let mut state = lock(&self.state);
        match &mut state.record {
            Record::AwaitingHandle => {
                state.attachment_waker = Some(cx.waker().clone());
                return Poll::Pending;
            }
            Record::Running(handle) => match Pin::new(handle).poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(outcome)) => state.record = Record::Joined(outcome),
                Poll::Ready(Err(error)) => state.record = Record::JoinFailed(error),
            },
            Record::Idle
            | Record::Joined(_)
            | Record::JoinFailed(_)
            | Record::SpawnPanicked { .. } => {}
        }
        Poll::Ready(state.observation())
    }

    fn retire_if_clean(&self) {
        let clean = {
            let state = lock(&self.state);
            state.closed && state.observation().is_complete()
        };
        if clean && let Some(registry) = self.registry.upgrade() {
            let retired = {
                let mut scopes = lock(&registry.scopes);
                scopes
                    .iter()
                    .position(|scope| std::ptr::eq(scope.as_ref(), self))
                    .map(|position| scopes.swap_remove(position))
            };
            drop(retired);
        }
    }
}

pub fn begin_featured_warmup_process_close() -> FeaturedWarmupDrain {
    SCOPES.get_or_init(Default::default).begin_close()
}

/// Separate from curated receipts. Dropping this observer never drops primary custody.
/// Counts cover retained scopes in this snapshot, not historical warmup starts.
pub struct FeaturedWarmupDrain {
    scopes: Vec<Arc<FeaturedWarmupScope>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeaturedWarmupProcessObservation {
    pub scopes: usize,
    pub idle: usize,
    pub pending: usize,
    pub joined: usize,
    pub panicked: usize,
    pub join_failed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: usize,
}

impl FeaturedWarmupProcessObservation {
    pub fn is_complete(&self) -> bool {
        self.pending == 0 && self.panicked == 0 && self.join_failed == 0
    }
}

impl FeaturedWarmupDrain {
    pub async fn wait_until(&self, deadline: Instant) -> FeaturedWarmupProcessObservation {
        let mut total = FeaturedWarmupProcessObservation {
            scopes: self.scopes.len(),
            ..Default::default()
        };
        for scope in &self.scopes {
            let observed = scope.wait_until(deadline).await;
            match observed.ownership {
                FeaturedWarmupOwnership::Idle => total.idle += 1,
                FeaturedWarmupOwnership::Pending => total.pending += 1,
                FeaturedWarmupOwnership::Joined => {
                    total.joined += 1;
                    if observed.outcome.is_none() {
                        total.join_failed += 1;
                    }
                }
                FeaturedWarmupOwnership::Panicked => total.panicked += 1,
            }
            match observed.outcome {
                Some(FeaturedWarmupOutcome::Succeeded) => total.succeeded += 1,
                Some(FeaturedWarmupOutcome::Failed) => total.failed += 1,
                Some(FeaturedWarmupOutcome::Cancelled) => total.cancelled += 1,
                None => {}
            }
        }
        total
    }
}

#[cfg(test)]
#[path = "featured_warmup_tests.rs"]
mod tests;
