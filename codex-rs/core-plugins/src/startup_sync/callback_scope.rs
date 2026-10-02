//! Owned curated notification actions. This scope does not drain other plugin
//! callbacks, session MCP prewarming, or the process-wide native sync worker.

use futures::FutureExt;
use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::sync::PoisonError;
use std::sync::Weak;
use std::time::Duration;
use std::time::Instant;
use tokio::runtime::Handle;
use tokio::task::JoinError;
use tokio::task::JoinHandle;

type Action = dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;
type CallbackJob = Pin<Box<dyn Future<Output = ActionOutcome> + Send>>;

static CALLBACK_SCOPES: OnceLock<Arc<CallbackRegistry>> = OnceLock::new();

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Default)]
struct CallbackRegistry {
    scopes: Mutex<Vec<Arc<CuratedCallbackScope>>>,
}

#[derive(Clone, Copy)]
enum ActionOutcome {
    Completed,
    Suppressed,
}

enum Record {
    AwaitingHandle,
    Running(JoinHandle<ActionOutcome>),
    Joining,
    Completed,
    Suppressed,
    Failed {
        // Original failures remain owned, including arbitrary panic payloads.
        _join_error: Option<JoinError>,
        _spawn_panic: Option<Box<dyn Any + Send>>,
    },
}

enum ExpectedRecord {
    AwaitingHandle,
    Joining,
}

impl Record {
    fn is_pending(&self) -> bool {
        matches!(
            self,
            Self::AwaitingHandle | Self::Running(_) | Self::Joining
        )
    }
}

#[derive(Default)]
struct ScopeState {
    closed: bool,
    records: Vec<Record>,
    // An impossible attachment mismatch must retain custody and fail closed.
    unexpected: Vec<Record>,
}

impl ScopeState {
    fn observation(&self) -> CuratedCallbackObservation {
        let mut observation = CuratedCallbackObservation::default();
        for record in &self.records {
            match record {
                Record::AwaitingHandle | Record::Running(_) | Record::Joining => {
                    observation.pending += 1;
                }
                Record::Completed => observation.completed += 1,
                Record::Suppressed => observation.suppressed += 1,
                Record::Failed { .. } => observation.failed += 1,
            }
        }
        observation.failed += self.unexpected.len();
        observation.pending += self
            .unexpected
            .iter()
            .filter(|record| record.is_pending())
            .count();
        observation
    }
}

/// Counts exact registered actions; completion does not cover unrelated work
/// scheduled by the action's callees or best-effort failures they only log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CuratedCallbackObservation {
    pub pending: usize,
    pub completed: usize,
    pub suppressed: usize,
    pub failed: usize,
}

impl CuratedCallbackObservation {
    pub fn is_complete(&self) -> bool {
        self.pending == 0 && self.failed == 0
    }
}

/// Per-processor admission and primary ownership of curated callback actions.
/// Explicit host lifetime guards must call `begin_close`; dropping producer
/// clones or observers does not close admission or abandon registered work.
pub struct CuratedCallbackScope {
    runtime: Handle,
    registry: Weak<CallbackRegistry>,
    state: Mutex<ScopeState>,
}

impl CuratedCallbackScope {
    pub fn new(runtime: Handle) -> Arc<Self> {
        Self::with_registry(runtime, CALLBACK_SCOPES.get_or_init(Default::default))
    }

    fn with_registry(runtime: Handle, registry: &Arc<CallbackRegistry>) -> Arc<Self> {
        let scope = Arc::new(Self {
            runtime,
            registry: Arc::downgrade(registry),
            state: Mutex::new(ScopeState::default()),
        });
        lock(&registry.scopes).push(Arc::clone(&scope));
        scope
    }

    /// Build a payload-free notification. Its action must await its own work;
    /// nested detached spawns are outside this scope's completion guarantee.
    pub fn callback<F, W>(self: &Arc<Self>, work: W) -> CuratedSyncCallback
    where
        F: Future<Output = ()> + Send + 'static,
        W: Fn() -> F + Send + Sync + 'static,
    {
        CuratedSyncCallback {
            scope: Arc::clone(self),
            action: Arc::new(move || Box::pin(work())),
        }
    }

    /// Irreversibly reject new dispatches. A reserved task checks closure before
    /// action admission; an action already admitted may finish after this call.
    pub fn begin_close(&self) {
        lock(&self.state).closed = true;
        self.retire_if_clean();
    }

    /// Close and observe until the deadline, checked between observations.
    /// Pending and failed records retain primary custody after expiry or caller
    /// cancellation. Another observer must harvest their eventual join results.
    pub async fn wait_until(&self, deadline: Instant) -> CuratedCallbackObservation {
        self.wait_inner(Some(deadline)).await
    }

    /// Close and wait without adding a deadline to an enclosing host budget.
    pub async fn wait(&self) -> CuratedCallbackObservation {
        self.wait_inner(/*deadline*/ None).await
    }

    async fn wait_inner(&self, deadline: Option<Instant>) -> CuratedCallbackObservation {
        self.begin_close();
        loop {
            let observation = self.poll();
            if observation.pending == 0 || deadline.is_some_and(|end| Instant::now() >= end) {
                return observation;
            }
            let next = Instant::now() + Duration::from_millis(/*millis*/ 10);
            let wake = deadline.map_or(next, |end| end.min(next));
            tokio::time::sleep_until(wake.into()).await;
        }
    }

    fn reserve(&self) -> Option<usize> {
        let mut state = lock(&self.state);
        if state.closed
            || !state.unexpected.is_empty()
            || state
                .records
                .iter()
                .any(|record| matches!(record, Record::Failed { .. }))
        {
            return None;
        }
        let id = state.records.len();
        state.records.push(Record::AwaitingHandle);
        Some(id)
    }

    fn publish(&self, id: usize, next: Record, expected: ExpectedRecord) {
        let mut state = lock(&self.state);
        let matches_expected = state.records.get(id).is_some_and(|record| match expected {
            ExpectedRecord::AwaitingHandle => matches!(record, Record::AwaitingHandle),
            ExpectedRecord::Joining => matches!(record, Record::Joining),
        });
        if matches_expected {
            // Only empty placeholders are replaced while holding the mutex.
            state.records[id] = next;
        } else {
            state.unexpected.push(next);
        }
    }

    fn poll(&self) -> CuratedCallbackObservation {
        let finished = {
            let mut state = lock(&self.state);
            let mut finished = Vec::new();
            for (id, record) in state.records.iter_mut().enumerate() {
                if matches!(record, Record::Running(handle) if handle.is_finished())
                    && let Record::Running(handle) = std::mem::replace(record, Record::Joining)
                {
                    finished.push((id, handle));
                }
            }
            finished
        };
        for (id, mut handle) in finished {
            // Borrowed, one-shot polling outside the mutex. Even an unexpected
            // Pending restores this exact handle; no await can lose custody.
            let next = match (&mut handle).now_or_never() {
                Some(Ok(ActionOutcome::Completed)) => Record::Completed,
                Some(Ok(ActionOutcome::Suppressed)) => Record::Suppressed,
                Some(Err(error)) => Record::Failed {
                    _join_error: Some(error),
                    _spawn_panic: None,
                },
                None => Record::Running(handle),
            };
            self.publish(id, next, ExpectedRecord::Joining);
        }
        let observation = lock(&self.state).observation();
        self.retire_if_clean();
        observation
    }

    fn retire_if_clean(&self) {
        let retire = {
            let state = lock(&self.state);
            state.closed && state.observation().is_complete()
        };
        if retire && let Some(registry) = self.registry.upgrade() {
            // Closed-clean is monotonic. Never hold scope and registry mutexes
            // together, or drop removed ownership under either mutex.
            let retired = {
                let mut scopes = lock(&registry.scopes);
                scopes
                    .iter()
                    .position(|scope| std::ptr::eq(scope.as_ref(), self))
                    .map(|index| scopes.swap_remove(index))
            };
            drop(retired);
        }
    }
}

/// A curated notification carries no remote materialization or config payload.
#[derive(Clone)]
pub struct CuratedSyncCallback {
    scope: Arc<CuratedCallbackScope>,
    action: Arc<Action>,
}

impl CuratedSyncCallback {
    /// Synchronously reserve scope ownership before dispatching onto its captured
    /// runtime. Returns false after closure or an observed failure; true means
    /// registered, not successful. Safe to invoke from a native worker.
    pub fn dispatch(&self) -> bool {
        self.dispatch_with_spawn(|job| self.scope.runtime.spawn(job))
    }

    fn dispatch_with_spawn(
        &self,
        spawn: impl FnOnce(CallbackJob) -> JoinHandle<ActionOutcome>,
    ) -> bool {
        let Some(id) = self.scope.reserve() else {
            return false;
        };
        let scope = Arc::clone(&self.scope);
        let action = Arc::clone(&self.action);
        let job: CallbackJob = Box::pin(async move {
            let admitted = !lock(&scope.state).closed;
            if admitted {
                action().await;
                ActionOutcome::Completed
            } else {
                ActionOutcome::Suppressed
            }
        });
        let next = match catch_unwind(AssertUnwindSafe(|| spawn(job))) {
            Ok(handle) => Record::Running(handle),
            Err(payload) => Record::Failed {
                _join_error: None,
                _spawn_panic: Some(payload),
            },
        };
        self.scope.publish(id, next, ExpectedRecord::AwaitingHandle);
        true
    }
}

#[cfg(test)]
#[path = "callback_scope_tests.rs"]
mod tests;
