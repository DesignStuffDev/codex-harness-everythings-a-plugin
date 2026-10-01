use std::sync::Arc;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::SessionReporter;

use crate::FileSearchSession;
use crate::PendingFileSearchStart;
use crate::state::*;

/// Cloneable public scope. Internal tasks hold Scope, never ScopeHandle, so last
/// public clone drop fences this scope without shutting down sibling scopes.
#[derive(Clone)]
pub struct FileSearchScope {
    handle: Arc<ScopeHandle>,
}
struct ScopeHandle(Arc<Scope>);
impl Drop for ScopeHandle {
    fn drop(&mut self) {
        self.0.request_shutdown();
    }
}

impl FileSearchScope {
    pub(crate) fn new(inner: Arc<Scope>) -> Self {
        Self {
            handle: Arc::new(ScopeHandle(inner)),
        }
    }
    pub(crate) fn inner(&self) -> &Arc<Scope> {
        &self.handle.0
    }
    /// Reserve one start synchronously and expose its exact cancellation control.
    pub fn begin_open(
        &self,
        request: SearchOpen,
        reporter: Arc<dyn SessionReporter>,
    ) -> Result<PendingFileSearchStart, SearchStartError> {
        crate::startup::begin_open(Arc::clone(self.inner()), request, reporter)
    }
    pub async fn open(
        &self,
        request: SearchOpen,
        reporter: Arc<dyn SessionReporter>,
    ) -> Result<FileSearchSession, SearchStartError> {
        self.begin_open(request, reporter)?.finish().await
    }
    pub fn request_shutdown(&self) {
        self.inner().request_shutdown();
    }
    pub fn shutdown(&self) -> SearchCloseFuture<'static> {
        self.request_shutdown();
        Box::pin(observe(self.inner().completed.subscribe()))
    }
}

impl Scope {
    pub(crate) fn request_shutdown(self: &Arc<Self>) {
        let Some(provider) = self.provider.upgrade() else {
            if self.completed.borrow().is_none() {
                complete(&self.completed, uncertain(lost()));
            }
            return;
        };
        let token = provider.tasks.token();
        let leases = {
            let mut state = lock(&provider.state);
            let Some(scope) = state.scopes.get_mut(&self.id) else {
                return;
            };
            if scope.closing {
                return;
            }
            scope.closing = true;
            scope.leases.values().cloned().collect::<Vec<_>>()
        };
        for lease in &leases {
            lease.request_close();
        }
        let owner = Arc::clone(self);
        let runtime = provider.runtime.clone();
        let guard = ScopeCompletionGuard {
            owner: Some(Arc::clone(&owner)),
            _token: token,
        };
        runtime.spawn(async move {
            let mut guard = guard;
            let mut outcome = SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Joined,
            };
            for lease in leases {
                let closed = observe(lease.completed.subscribe()).await;
                if outcome.operation.is_ok() {
                    outcome.operation = closed.operation;
                }
                if let CloseCleanup::Unconfirmed(error) = closed.cleanup {
                    outcome.cleanup = CloseCleanup::Unconfirmed(error);
                }
            }
            {
                let mut state = lock(&provider.state);
                if let Some(scope) = state.scopes.get(&owner.id)
                    && let Some(error) = &scope.first_error
                {
                    outcome.operation = Err(error.clone());
                }
                if let Err(error) = &outcome.operation {
                    state.first_error.get_or_insert_with(|| error.clone());
                }
                if outcome.cleanup == CloseCleanup::Joined {
                    state.scopes.remove(&owner.id);
                }
            }
            complete(&owner.completed, outcome);
            guard.owner.take();
        });
    }
}
struct ScopeCompletionGuard {
    owner: Option<Arc<Scope>>,
    _token: tokio_util::task::task_tracker::TaskTrackerToken,
}
impl Drop for ScopeCompletionGuard {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            if let Some(provider) = owner.provider.upgrade() {
                lock(&provider.state).first_error.get_or_insert_with(lost);
            }
            complete(&owner.completed, uncertain(lost()));
        }
    }
}
