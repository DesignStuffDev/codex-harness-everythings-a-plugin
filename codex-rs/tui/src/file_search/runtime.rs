//! Public lifetime ownership is separate from the retained coordinator/drain.
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use std::time::Duration;

use crate::app_event::AppEvent;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::ScopeLimits;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use codex_file_search_runtime::FileSearchProvider;
use codex_file_search_runtime::FileSearchScope;
use codex_file_search_runtime::FileSearchScopeFactory;
use codex_file_search_runtime::SelectionContext;
use codex_file_search_runtime::interactive_allocation;
use codex_file_search_runtime::interactive_policy;
use codex_file_search_runtime::select_provider;
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::Semaphore;
use tokio::sync::mpsc::WeakUnboundedSender;
use tokio::sync::watch;
use tokio_util::task::TaskTracker;

use super::coordinator;
use super::reporter;
use super::state::Intent;
use super::state::SearchState;
use super::state::lock;

pub(crate) enum FileSearchSource {
    Embedded(FileSearchScopeFactory),
    Local(SelectionContext),
}
#[derive(Clone)]
pub(crate) struct FileSearchRuntime(Arc<PublicOwner>);
struct PublicOwner {
    inner: Arc<RuntimeInner>,
    local: Option<Arc<FileSearchProvider>>,
    executor: tokio::runtime::Handle,
}
impl Drop for PublicOwner {
    fn drop(&mut self) {
        begin_shutdown(&self.inner, self.local.clone(), &self.executor);
    }
}

pub(super) struct RuntimeInner {
    pub scope: FileSearchScope,
    pub options: FileSearchOptions,
    pub budget: SearchBudget,
    pub max_query_bytes: usize,
    pub state: Mutex<RuntimeState>,
    pub signal: watch::Sender<()>,
    pub tasks: TaskTracker,
    pub wakes: Mutex<WakeState>,
    pub wake_capacity: Arc<Semaphore>,
    completed: watch::Sender<Option<SearchCloseOutcome>>,
}
#[derive(Default)]
pub(super) struct RuntimeState {
    pub closing: bool,
    drain_started: bool,
    pub active: Weak<Mutex<SearchState>>,
    pub intent: Option<Intent>,
    pub latest_revision: u64,
    pub first_error: Option<SearchError>,
    cleanup_error: Option<SearchError>,
}
#[derive(Default)]
pub(super) struct WakeState {
    sender: Option<WeakUnboundedSender<AppEvent>>,
    pub channel: Arc<()>,
    pub queued: Weak<WakeCredit>,
}
#[derive(Debug)]
pub(super) struct WakeCredit {
    pub _permit: OwnedSemaphorePermit,
}
impl RuntimeInner {
    pub fn failed(&self, error: SearchError) {
        lock(&self.state).first_error.get_or_insert(error);
    }
    pub fn unconfirmed(&self, error: SearchError) {
        lock(&self.state).cleanup_error.get_or_insert(error);
    }
    pub fn uncertain(&self, cleanup: SearchError) -> SearchCloseOutcome {
        let operation = lock(&self.state)
            .first_error
            .clone()
            .unwrap_or_else(|| cleanup.clone());
        SearchCloseOutcome {
            operation: Err(operation),
            cleanup: CloseCleanup::Unconfirmed(cleanup),
        }
    }
    fn publish_once(&self, outcome: SearchCloseOutcome) {
        self.completed.send_if_modified(|current| {
            if current.is_some() {
                return false;
            }
            *current = Some(outcome);
            true
        });
    }
    pub fn closing(&self) -> bool {
        lock(&self.state).closing
    }
    pub fn coordinator_lost(&self, error: SearchError) {
        let active = {
            let mut state = lock(&self.state);
            state.closing = true;
            state.intent = None;
            state.first_error.get_or_insert_with(|| error.clone());
            state.active.upgrade()
        };
        self.scope.request_shutdown();
        self.signal.send_replace(());
        self.publish_once(self.uncertain(error.clone()));
        if let Some(active) = active {
            let generation = lock(&active).request();
            reporter::fail(&active, &generation, self, error);
        }
    }

    pub fn take_intent(&self) -> Option<Intent> {
        lock(&self.state).intent.take()
    }
    pub fn current_manager(&self, intent: &Intent) -> bool {
        let state = lock(&self.state);
        !state.closing && Weak::ptr_eq(&state.active, &intent.state)
    }
    pub fn requeue(&self, intent: Intent) {
        let mut state = lock(&self.state);
        if !state.closing
            && Weak::ptr_eq(&state.active, &intent.state)
            && state.latest_revision == intent.request.query_id
            && state.intent.is_none()
        {
            state.intent = Some(intent);
        }
    }
}

impl FileSearchRuntime {
    pub(crate) async fn start(source: FileSearchSource) -> Result<Self, SearchStartError> {
        tokio::runtime::Handle::try_current().map_err(|_| {
            not_admitted(SearchError::new(
                SearchErrorKind::InvalidInput,
                "TUI file search requires an active Tokio runtime",
            ))
        })?;
        let (factory, local) = match source {
            FileSearchSource::Embedded(factory) => (factory, None),
            FileSearchSource::Local(context) => {
                let policy = interactive_policy().map_err(not_admitted)?;
                let selected = select_provider(context, policy).await?;
                let provider = Arc::new(selected.provider);
                (provider.scope_factory(), Some(provider))
            }
        };
        match Self::assemble(factory, local.clone()) {
            Ok(runtime) => Ok(runtime),
            Err(mut operation) => {
                let cleanup = match local {
                    Some(provider) => {
                        let outcome = provider.shutdown().await;
                        if let Err(error) = outcome.operation {
                            operation = SearchError::new(
                                operation.kind(),
                                format!(
                                    "{operation}; selected provider cleanup operation also failed: {error}"
                                ),
                            );
                        }
                        outcome.cleanup.into()
                    }
                    None => StartCleanup::NotAdmitted,
                };
                Err(SearchStartError { operation, cleanup })
            }
        }
    }
    fn assemble(
        factory: FileSearchScopeFactory,
        local: Option<Arc<FileSearchProvider>>,
    ) -> Result<Self, SearchError> {
        let executor = tokio::runtime::Handle::try_current().map_err(|_| {
            SearchError::new(
                SearchErrorKind::InvalidInput,
                "TUI file search requires an active Tokio runtime",
            )
        })?;
        let effective = factory.effective_policy()?;
        let options = FileSearchOptions {
            compute_indices: true,
            ..Default::default()
        };
        let budget = interactive_allocation(options.threads)?;
        budget.validate_within(&effective.provider.resources)?;
        if options.limit > effective.max_matches {
            return Err(SearchError::new(
                SearchErrorKind::UnsupportedOption,
                "selected file-search provider cannot supply the TUI result limit",
            ));
        }
        let max_sessions = NonZeroUsize::new(effective.provider.max_sessions.get().min(8))
            .ok_or_else(|| {
                SearchError::new(
                    SearchErrorKind::ResourceExhausted,
                    "TUI search scope has zero capacity",
                )
            })?;
        let scope = factory.new_scope(ScopeLimits { max_sessions })?;
        let inner = Arc::new(RuntimeInner {
            scope,
            options,
            budget,
            max_query_bytes: effective.max_query_bytes.get(),
            state: Mutex::new(RuntimeState::default()),
            signal: watch::channel(()).0,
            tasks: TaskTracker::new(),
            wakes: Mutex::new(WakeState::default()),
            wake_capacity: Arc::new(Semaphore::new(8)),
            completed: watch::channel(None).0,
        });
        let runtime = Self(Arc::new(PublicOwner {
            inner: Arc::clone(&inner),
            local,
            executor,
        }));
        coordinator::start(&inner);
        Ok(runtime)
    }
    pub(super) fn inner(&self) -> Arc<RuntimeInner> {
        Arc::clone(&self.0.inner)
    }
    pub(super) fn max_query_bytes(&self) -> usize {
        self.0.inner.max_query_bytes
    }
    pub(super) fn wake(&self) {
        self.0.inner.signal.send_replace(());
    }
    pub(super) fn register(&self, manager: &Arc<Mutex<SearchState>>) {
        {
            let mut manager = lock(manager);
            let mut wakes = lock(&self.0.inner.wakes);
            let same_channel = wakes
                .sender
                .as_ref()
                .and_then(WeakUnboundedSender::upgrade)
                .is_some_and(|sender| sender.same_channel(&manager.tx.app_event_tx));
            if !same_channel {
                wakes.channel = Arc::new(());
                wakes.queued = Weak::new();
                wakes.sender = Some(manager.tx.app_event_tx.downgrade());
            }
            manager.channel = Arc::clone(&wakes.channel);
        }
        let previous = {
            let mut state = lock(&self.0.inner.state);
            let previous = state.active.upgrade();
            state.active = Arc::downgrade(manager);
            state.intent = None;
            state.latest_revision = 0;
            previous
        };
        if let Some(previous) = previous {
            let session = lock(&previous).invalidate();
            if let Some(session) = session {
                session.request_close();
            }
        }
        self.wake();
    }
    pub(super) fn enqueue(&self, intent: Intent) {
        let result = {
            let mut state = lock(&self.0.inner.state);
            if state.closing {
                Some(SearchError::new(
                    SearchErrorKind::ClosedLease,
                    "file search is shutting down",
                ))
            } else if !Weak::ptr_eq(&state.active, &intent.state)
                || intent.request.query_id < state.latest_revision
            {
                return;
            } else {
                state.latest_revision = intent.request.query_id;
                state.intent = Some(intent.clone());
                None
            }
        };
        self.wake();
        if let Some(error) = result
            && let Some(state) = intent.state.upgrade()
        {
            reporter::fail(&state, &intent.request, &self.0.inner, error);
        }
    }
    pub(crate) fn request_shutdown(&self) {
        begin_shutdown(&self.0.inner, self.0.local.clone(), &self.0.executor);
    }
    pub(crate) async fn shutdown(&self) -> anyhow::Result<()> {
        self.request_shutdown();
        let mut completion = self.0.inner.completed.subscribe();
        loop {
            if let Some(outcome) = completion.borrow_and_update().clone() {
                return close_result(outcome);
            }
            completion.changed().await.map_err(|_| {
                anyhow::anyhow!("TUI search cleanup owner lost; cleanup unconfirmed")
            })?;
        }
    }
    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        let policy = interactive_policy().expect("test policy");
        let backend = codex_file_search::NativeSearchBackend::new(
            std::env::current_dir().expect("test cwd"),
            policy.native,
        )
        .expect("test native backend");
        let provider = Arc::new(
            FileSearchProvider::from_backend(Arc::new(backend), policy.runtime)
                .expect("test provider"),
        );
        Self::assemble(provider.scope_factory(), Some(provider)).expect("test runtime")
    }
}

fn not_admitted(operation: SearchError) -> SearchStartError {
    SearchStartError {
        operation,
        cleanup: StartCleanup::NotAdmitted,
    }
}

fn begin_shutdown(
    inner: &Arc<RuntimeInner>,
    local: Option<Arc<FileSearchProvider>>,
    executor: &tokio::runtime::Handle,
) {
    let active = {
        let mut state = lock(&inner.state);
        if state.drain_started {
            return;
        }
        state.drain_started = true;
        state.closing = true;
        state.intent = None;
        state.active.upgrade()
    };
    if let Some(active) = active {
        let session = lock(&active).invalidate();
        if let Some(session) = session {
            session.request_close();
        }
    }
    inner.scope.request_shutdown();
    if let Some(provider) = &local {
        provider.request_shutdown();
    }
    inner.signal.send_replace(());
    inner.tasks.close();
    let retained = Arc::clone(inner);
    let guard = DrainGuard {
        inner: Arc::clone(inner),
        finished: false,
    };
    executor.spawn(async move {
        let mut guard = guard;
        let cleanup = async {
            let mut outcome = retained.scope.shutdown().await;
            retained.tasks.wait().await;
            if let Some(provider) = local {
                merge(&mut outcome, provider.shutdown().await);
            }
            {
                let state = lock(&retained.state);
                if let Some(error) = &state.first_error { outcome.operation = Err(error.clone()); }
                if let Some(error) = &state.cleanup_error { outcome.cleanup = CloseCleanup::Unconfirmed(error.clone()); }
            }
            outcome
        };
        tokio::pin!(cleanup);
        let outcome = tokio::select! {
            outcome = &mut cleanup => outcome,
            _ = tokio::time::sleep(Duration::from_secs(120)) => {
                let error = SearchError::new(SearchErrorKind::ForcedShutdown, "TUI file-search shutdown deadline expired; cleanup unconfirmed");
                retained.publish_once(retained.uncertain(error));
                // The public receipt remains uncertainty; keep owning the drain.
                let _ = cleanup.await;
                guard.finished = true;
                return;
            }
        };
        retained.publish_once(outcome);
        guard.finished = true;
    });
}
fn merge(target: &mut SearchCloseOutcome, other: SearchCloseOutcome) {
    if target.operation.is_ok() {
        target.operation = other.operation;
    }
    if let CloseCleanup::Unconfirmed(error) = other.cleanup {
        target.cleanup = CloseCleanup::Unconfirmed(error);
    }
}
pub(super) fn close_result(outcome: SearchCloseOutcome) -> anyhow::Result<()> {
    let operation = outcome.operation.err().map(|error| error.to_string());
    let cleanup = match outcome.cleanup {
        CloseCleanup::Joined => None,
        CloseCleanup::Unconfirmed(error) => {
            Some(format!("file search cleanup unconfirmed: {error}"))
        }
    };
    match (operation, cleanup) {
        (None, None) => Ok(()),
        (Some(error), None) | (None, Some(error)) => Err(anyhow::anyhow!(error)),
        (Some(operation), Some(cleanup)) => Err(anyhow::anyhow!("{operation}; {cleanup}")),
    }
}

struct DrainGuard {
    inner: Arc<RuntimeInner>,
    finished: bool,
}
impl Drop for DrainGuard {
    fn drop(&mut self) {
        if !self.finished {
            let error = SearchError::new(
                SearchErrorKind::TransportLost,
                "TUI search drain owner was lost; cleanup unconfirmed",
            );
            self.inner.failed(error.clone());
            self.inner.publish_once(self.inner.uncertain(error));
        }
    }
}
