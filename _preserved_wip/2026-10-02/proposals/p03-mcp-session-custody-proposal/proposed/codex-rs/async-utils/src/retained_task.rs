//! Primary task custody which outlives cancelled shutdown observers.

use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::PoisonError;
use std::sync::Weak;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinError;
use tokio::task::JoinHandle;

/// Classifies a joined result for registry retirement. Implementations must
/// return true only for successful completion; this runs outside owner locks.
pub trait RetainedTaskOutcome: Send + Sync + 'static {
    fn succeeded(&self) -> bool;
}

impl RetainedTaskOutcome for () {
    fn succeeded(&self) -> bool { true }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> RetainedTaskOutcome for Result<T, E> {
    fn succeeded(&self) -> bool { self.is_ok() }
}

/// Bounded public classification; original failures stay in primary custody.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskJoinFailure {
    Panicked,
    Cancelled,
    SpawnPanicked,
}

impl std::fmt::Display for TaskJoinFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Panicked => "owned task panicked",
            Self::Cancelled => "owned task was cancelled",
            Self::SpawnPanicked => "owned task spawn panicked",
        })
    }
}

impl std::error::Error for TaskJoinFailure {}

#[derive(Default)]
struct Registry {
    tasks: Mutex<Vec<Arc<dyn Any + Send + Sync>>>,
}

static TASKS: OnceLock<Arc<Registry>> = OnceLock::new();

struct Completion<T> {
    result: Result<Arc<T>, TaskJoinFailure>,
    _join_error: Option<JoinError>,
    _spawn_panic: Option<Box<dyn Any + Send>>,
}

enum State<T> {
    Reserved,
    Running(JoinHandle<T>),
    Joining,
    Complete(Completion<T>),
}

/// A pre-registered owner of one exact Tokio task and its immutable result.
/// Waiting never transfers its handle into an abortable observer. Successful
/// joined tasks retire; failed and unobserved tasks remain process-reachable.
/// This does not stop the task or prove ownership of work it detaches itself.
pub struct RetainedTask<T: RetainedTaskOutcome> {
    registry: Weak<Registry>,
    state: Mutex<State<T>>,
}

impl<T: RetainedTaskOutcome> RetainedTask<T> {
    pub fn spawn(runtime: &Handle, future: impl Future<Output = T> + Send + 'static) -> Arc<Self> {
        Self::spawn_in(runtime, future, TASKS.get_or_init(Default::default), |_| {})
    }

    /// Publish the reserved owner into its enclosing lifecycle before the task
    /// can run. The publisher must not retain a lock when it returns.
    pub fn spawn_published(
        runtime: &Handle,
        future: impl Future<Output = T> + Send + 'static,
        publish: impl FnOnce(Arc<Self>),
    ) -> Arc<Self> {
        Self::spawn_in(runtime, future, TASKS.get_or_init(Default::default), publish)
    }

    fn spawn_in(
        runtime: &Handle,
        future: impl Future<Output = T> + Send + 'static,
        registry: &Arc<Registry>,
        publish: impl FnOnce(Arc<Self>),
    ) -> Arc<Self> {
        let owner = Arc::new(Self {
            registry: Arc::downgrade(registry),
            state: Mutex::new(State::Reserved),
        });
        registry.tasks.lock().unwrap_or_else(PoisonError::into_inner).push(owner.clone());
        // Neither spawning nor destruction of arbitrary future captures may run
        // under either registry or owner mutex.
        let state = match catch_unwind(AssertUnwindSafe(|| {
            publish(Arc::clone(&owner));
            runtime.spawn(future)
        })) {
            Ok(handle) => State::Running(handle),
            Err(payload) => State::Complete(Completion {
                result: Err(TaskJoinFailure::SpawnPanicked),
                _join_error: None,
                _spawn_panic: Some(payload),
            }),
        };
        *owner.state.lock().unwrap_or_else(PoisonError::into_inner) = state;
        owner
    }

    /// The caller supplies any deadline around this observation. Expiry or
    /// cancellation does not abort, detach, or discard the primary task.
    pub async fn wait(&self) -> Result<Arc<T>, TaskJoinFailure> {
        loop {
            if let Some(result) = self.poll_join() {
                if result.as_ref().is_ok_and(|outcome| outcome.succeeded()) {
                    self.retire();
                }
                return result;
            }
            tokio::time::sleep(Duration::from_millis(/*millis*/ 10)).await;
        }
    }

    fn poll_join(&self) -> Option<Result<Arc<T>, TaskJoinFailure>> {
        let mut handle = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            match &*state {
                State::Complete(completion) => return Some(completion.result.clone()),
                State::Running(handle) if handle.is_finished() => {},
                State::Reserved | State::Running(_) | State::Joining => return None,
            }
            let State::Running(handle) = std::mem::replace(&mut *state, State::Joining) else {
                return None;
            };
            handle
        };
        // No await between taking and publishing. Concurrent observers see
        // Joining; an unexpected Pending restores this same handle.
        let state = match Pin::new(&mut handle).poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(Ok(output)) => State::Complete(Completion {
                result: Ok(Arc::new(output)),
                _join_error: None,
                _spawn_panic: None,
            }),
            Poll::Ready(Err(error)) => State::Complete(Completion {
                result: Err(if error.is_panic() { TaskJoinFailure::Panicked } else { TaskJoinFailure::Cancelled }),
                _join_error: Some(error),
                _spawn_panic: None,
            }),
            Poll::Pending => State::Running(handle),
        };
        let result = match &state {
            State::Complete(completion) => Some(completion.result.clone()),
            State::Reserved | State::Running(_) | State::Joining => None,
        };
        *self.state.lock().unwrap_or_else(PoisonError::into_inner) = state;
        result
    }

    fn retire(&self) {
        if let Some(registry) = self.registry.upgrade() {
            let retired = {
                let mut tasks = registry.tasks.lock().unwrap_or_else(PoisonError::into_inner);
                tasks.iter().position(|task| task.downcast_ref::<Self>().is_some_and(|task| std::ptr::eq(task, self)))
                    .map(|index| tasks.swap_remove(index))
            };
            drop(retired);
        }
    }
}

#[cfg(test)]
#[path = "retained_task_tests.rs"]
mod tests;
