//! Async lifetime ownership for native sessions, including abandoned waiters.

use crate::FileSearchOptions;
use crate::FileSearchSession;
use crate::SessionReporter;
use crate::create_session;
use std::collections::HashMap;
use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio_util::task::TaskTracker;
use tokio_util::task::task_tracker::TaskTrackerToken;

const MAX_FAILURES: usize = 16;
const MAX_FAILURE_BYTES: usize = 2_048;
type Outcome = Result<(), String>;

/// Retains native startup and joined cleanup across cancellation of client waits.
///
/// Create sessions inside a Tokio runtime and explicitly await shutdown before
/// destroying that runtime. One capacity slot covers startup, use, and cleanup.
/// Clones share the lifetime owner; dropping its last clone requests shutdown.
pub struct FileSearchOwner {
    inner: Arc<OwnerInner>,
}

/// An observed startup failure after waiting for its owner-retained cleanup.
/// Pre-admission rejections have no accepted work and do not use this wrapper.
#[derive(Debug)]
pub struct FileSearchStartError {
    operation: anyhow::Error,
    cleanup: Option<anyhow::Error>,
}

impl FileSearchStartError {
    /// The original startup failure, including intentional release during startup.
    pub fn operation_error(&self) -> &anyhow::Error {
        &self.operation
    }

    /// Failure reported by the joined close receipt. `None` confirms successful
    /// cleanup even when the operation was cancelled before acknowledgement.
    /// Constructor failures retain their existing conservative close outcome.
    pub fn cleanup_error(&self) -> Option<&anyhow::Error> {
        self.cleanup.as_ref()
    }
}

impl std::fmt::Display for FileSearchStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:#}", self.operation)?;
        if let Some(cleanup) = &self.cleanup {
            write!(
                formatter,
                "; file-search startup cleanup failed: {cleanup:#}"
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for FileSearchStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.operation.as_ref())
    }
}

struct OwnerInner {
    state: Mutex<State>,
    tasks: TaskTracker,
    capacity: usize,
    owners: AtomicUsize,
}

#[derive(Default)]
struct State {
    stopping: bool,
    next_id: u64,
    entries: HashMap<u64, Entry>,
    failures: Vec<String>,
    omitted_failures: usize,
}

struct Entry {
    phase: Phase,
    runtime: Handle,
    ready: Arc<Completion>,
    closed: Arc<Completion>,
}

enum Phase {
    Preparing { released: bool },
    Ready(FileSearchSession),
    Closing,
}

#[derive(Default)]
struct Completion {
    result: Mutex<Option<Outcome>>,
    changed: Notify,
}

impl Completion {
    fn finish(&self, result: Outcome) {
        *self
            .result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(result);
        self.changed.notify_waiters();
    }

    async fn wait(&self) -> anyhow::Result<()> {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let result = self
                .result
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            if let Some(result) = result {
                return result.map_err(anyhow::Error::msg);
            }
            changed.await;
        }
    }
}

impl FileSearchOwner {
    pub fn new(max_sessions: NonZero<usize>) -> Self {
        Self {
            inner: Arc::new(OwnerInner {
                state: Mutex::new(State::default()),
                tasks: TaskTracker::new(),
                capacity: max_sessions.get(),
                owners: AtomicUsize::new(1),
            }),
        }
    }

    /// Reserve ownership before starting native work. Cancelling this wait
    /// releases the reservation but leaves accepted cleanup with this owner.
    pub async fn create(
        &self,
        roots: Vec<PathBuf>,
        options: FileSearchOptions,
        reporter: Arc<dyn SessionReporter>,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> anyhow::Result<ManagedFileSearchSession> {
        let runtime = Handle::try_current()?;
        let (id, ready, closed, token) = {
            let mut state = self
                .inner
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.stopping {
                anyhow::bail!("file-search owner is shutting down");
            }
            if state.entries.len() >= self.inner.capacity {
                anyhow::bail!("file-search owner session capacity reached");
            }
            let id = state
                .next_id
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("file-search session identities exhausted"))?;
            state.next_id = id;
            let ready = Arc::new(Completion::default());
            let closed = Arc::new(Completion::default());
            // Register under the admission lock, before shutdown can observe
            // an empty tracker and before any work is spawned or awaited.
            let token = self.inner.tasks.token();
            state.entries.insert(
                id,
                Entry {
                    phase: Phase::Preparing { released: false },
                    runtime: runtime.clone(),
                    ready: ready.clone(),
                    closed: closed.clone(),
                },
            );
            (id, ready, closed, token)
        };
        let session = ManagedFileSearchSession {
            lease: Arc::new(Lease {
                owner: self.inner.clone(),
                id,
                closed,
            }),
        };
        let startup =
            runtime.spawn_blocking(move || create_session(roots, options, reporter, cancel_flag));
        let owner = self.inner.clone();
        runtime.spawn(async move {
            let _token = token;
            let result = startup
                .await
                .map_err(|error| anyhow::anyhow!("file-search startup task failed: {error}"))
                .and_then(std::convert::identity);
            owner.started(id, result);
        });
        if let Err(start_error) = ready.wait().await {
            // An observed rejection acknowledges cleanup too. If this wait is
            // abandoned, the lease still leaves accepted work with the owner.
            let cleanup = session.close().await.err();
            return Err(FileSearchStartError {
                operation: start_error,
                cleanup,
            }
            .into());
        }
        Ok(session)
    }

    /// Atomically fence new sessions and request closure of every accepted one.
    pub fn request_shutdown(&self) {
        self.inner.request_shutdown();
    }

    /// Drain all accepted work. Cancelling this wait does not cancel cleanup;
    /// repeated observers receive the retained, bounded failure report.
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.request_shutdown();
        self.inner.tasks.wait().await;
        let state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut failures = state.failures.clone();
        if !state.entries.is_empty() {
            failures.push("file-search cleanup ended with retained sessions".into());
        }
        if state.omitted_failures != 0 {
            failures.push(format!(
                "{} additional search cleanup failures omitted",
                state.omitted_failures
            ));
        }
        if failures.is_empty() {
            Ok(())
        } else {
            anyhow::bail!(failures.join("; "))
        }
    }
}

impl Clone for FileSearchOwner {
    fn clone(&self) -> Self {
        self.inner.owners.fetch_add(1, Ordering::Relaxed);
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Drop for FileSearchOwner {
    fn drop(&mut self) {
        if self.inner.owners.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.inner.request_shutdown();
        }
    }
}

/// Cloneable access to one owner-retained native session. Dropping the final
/// clone requests closure; explicit close on any clone closes the shared session.
#[derive(Clone)]
pub struct ManagedFileSearchSession {
    lease: Arc<Lease>,
}

struct Lease {
    owner: Arc<OwnerInner>,
    id: u64,
    closed: Arc<Completion>,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.owner.release(self.id);
    }
}

impl ManagedFileSearchSession {
    pub fn update_query(&self, query: &str) {
        let state = self
            .lease
            .owner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(Entry {
            phase: Phase::Ready(session),
            ..
        }) = state.entries.get(&self.lease.id)
        {
            session.update_query(query);
        }
    }

    pub fn update_query_tagged(&self, query: &str, query_id: u64) -> anyhow::Result<()> {
        let state = self
            .lease
            .owner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match state.entries.get(&self.lease.id) {
            Some(Entry {
                phase: Phase::Ready(session),
                ..
            }) => session.update_query_tagged(query, query_id),
            _ => anyhow::bail!("file-search session is closed"),
        }
    }

    pub fn request_close(&self) {
        self.lease.owner.release(self.lease.id);
    }

    /// Detects native termination, including failure without a completion callback.
    /// A pending close is finished only after the native join has completed.
    pub fn is_finished(&self) -> bool {
        let state = self
            .lease
            .owner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match state.entries.get(&self.lease.id) {
            Some(Entry {
                phase: Phase::Ready(session),
                ..
            }) => session
                .supervisor
                .as_ref()
                .is_none_or(std::thread::JoinHandle::is_finished),
            Some(_) => false,
            None => true,
        }
    }

    pub async fn close(self) -> anyhow::Result<()> {
        self.request_close();
        self.lease.closed.wait().await
    }
}

struct CloseJob {
    id: u64,
    native: FileSearchSession,
    runtime: Handle,
    token: TaskTrackerToken,
}

impl OwnerInner {
    fn started(self: &Arc<Self>, id: u64, result: anyhow::Result<FileSearchSession>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Startup owns the only transition out of Preparing, so its entry
        // cannot have been removed by a dropped waiter or shutdown.
        let Some(entry) = state.entries.get_mut(&id) else {
            return;
        };
        let ready = entry.ready.clone();
        match result {
            Ok(native) => {
                if matches!(entry.phase, Phase::Preparing { released: false }) {
                    entry.phase = Phase::Ready(native);
                    ready.finish(Ok(()));
                } else {
                    native.request_close();
                    entry.phase = Phase::Closing;
                    let job = CloseJob {
                        id,
                        native,
                        runtime: entry.runtime.clone(),
                        token: self.tasks.token(),
                    };
                    drop(state);
                    ready.finish(Err("file-search session closed during startup".into()));
                    self.launch_close(job);
                }
            }
            Err(error) => {
                let error = bounded_failure(error);
                let closed = entry.closed.clone();
                state.entries.remove(&id);
                record_failure(&mut state, error.clone());
                drop(state);
                ready.finish(Err(error.clone()));
                closed.finish(Err(error));
            }
        }
    }

    fn release(self: &Arc<Self>, id: u64) {
        let job = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state
                .entries
                .get_mut(&id)
                .and_then(|entry| self.prepare_close(id, entry))
        };
        if let Some(job) = job {
            self.launch_close(job);
        }
    }

    fn prepare_close(&self, id: u64, entry: &mut Entry) -> Option<CloseJob> {
        match std::mem::replace(&mut entry.phase, Phase::Closing) {
            Phase::Preparing { .. } => {
                entry.phase = Phase::Preparing { released: true };
                None
            }
            Phase::Closing => None,
            Phase::Ready(native) => {
                native.request_close();
                Some(CloseJob {
                    id,
                    native,
                    runtime: entry.runtime.clone(),
                    token: self.tasks.token(),
                })
            }
        }
    }

    fn request_shutdown(self: &Arc<Self>) {
        let jobs = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.stopping = true;
            let jobs = state
                .entries
                .iter_mut()
                .filter_map(|(&id, entry)| self.prepare_close(id, entry))
                .collect::<Vec<_>>();
            self.tasks.close();
            jobs
        };
        for job in jobs {
            self.launch_close(job);
        }
    }

    fn launch_close(self: &Arc<Self>, job: CloseJob) {
        let CloseJob {
            id,
            native,
            runtime,
            token,
        } = job;
        let close = runtime.spawn_blocking(move || native.close());
        let owner = self.clone();
        runtime.spawn(async move {
            let _token = token;
            let result = close
                .await
                .map_err(|error| anyhow::anyhow!("file-search close task failed: {error}"))
                .and_then(std::convert::identity)
                .map_err(bounded_failure);
            let mut state = owner
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entry) = state.entries.remove(&id) {
                if let Err(error) = &result {
                    record_failure(&mut state, error.clone());
                }
                drop(state);
                entry.closed.finish(result);
            }
        });
    }
}

fn bounded_failure(error: anyhow::Error) -> String {
    let mut message = error.to_string();
    if message.len() > MAX_FAILURE_BYTES {
        let mut end = MAX_FAILURE_BYTES;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
    }
    message
}

fn record_failure(state: &mut State, error: String) {
    if state.failures.len() < MAX_FAILURES {
        state.failures.push(error);
    } else {
        state.omitted_failures = state.omitted_failures.saturating_add(1);
    }
}

#[cfg(test)]
#[path = "async_owner_tests.rs"]
mod tests;
