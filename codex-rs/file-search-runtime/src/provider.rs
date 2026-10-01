use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::ScopeLimits;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchError;
use futures::FutureExt;
use tokio::sync::watch;
use tokio_util::task::TaskTracker;

use crate::FileSearchScope;
use crate::RuntimePolicy;
use crate::policy::exhausted;
use crate::policy::invalid;
use crate::state::*;

/// Unique public composition owner. Retained task/scoped state never owns this
/// wrapper, so dropping it fences every scope even while public leases survive.
pub struct FileSearchProvider {
    pub(crate) inner: Arc<Provider>,
}

/// Local scope capability, with no backend selection or shutdown authority.
/// Cloning this weak capability does not keep public provider ownership alive.
#[derive(Clone)]
pub struct FileSearchScopeFactory {
    inner: Weak<Provider>,
}

impl FileSearchProvider {
    /// The caller must keep a cleanup guard/Arc for an already-started backend
    /// until construction succeeds. Validation failure returns no join receipt;
    /// composition must request and observe that backend's shutdown separately.
    pub fn from_backend(
        backend: Arc<dyn SearchBackend>,
        policy: RuntimePolicy,
    ) -> Result<Self, SearchError> {
        policy.validate()?;
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| invalid("file-search runtime requires an active Tokio runtime"))?;
        Ok(Self {
            inner: Arc::new(Provider {
                backend,
                policy,
                runtime,
                state: Mutex::new(ProviderState::default()),
                tasks: TaskTracker::new(),
                completed: watch::channel(None).0,
            }),
        })
    }
    pub fn scope_factory(&self) -> FileSearchScopeFactory {
        FileSearchScopeFactory {
            inner: Arc::downgrade(&self.inner),
        }
    }
    pub fn request_shutdown(&self) {
        self.inner.request_shutdown();
    }
    /// Fences synchronously when called; the returned future only observes a
    /// retained owner and may be dropped without abandoning accepted cleanup.
    pub fn shutdown(&self) -> SearchCloseFuture<'static> {
        self.request_shutdown();
        Box::pin(observe(self.inner.completed.subscribe()))
    }
}
impl Drop for FileSearchProvider {
    fn drop(&mut self) {
        self.request_shutdown();
    }
}

impl FileSearchScopeFactory {
    pub fn new_scope(&self, limits: ScopeLimits) -> Result<FileSearchScope, SearchError> {
        let provider = self.inner.upgrade().ok_or_else(closed)?;
        limits.validate_within(&provider.policy.provider)?;
        let mut state = lock(&provider.state);
        if state.closing {
            return Err(closed());
        }
        if state.scopes.len() >= provider.policy.provider.max_scopes.get() {
            return Err(exhausted("file-search scope metadata capacity reached"));
        }
        let id = state
            .next_scope
            .checked_add(1)
            .ok_or_else(|| exhausted("file-search scope identities exhausted"))?;
        state.next_scope = id;
        let owner = Arc::new(Scope {
            id,
            limits,
            provider: Arc::downgrade(&provider),
            completed: watch::channel(None).0,
        });
        state.scopes.insert(
            id,
            ScopeEntry {
                owner: Arc::clone(&owner),
                closing: false,
                leases: Default::default(),
                first_error: None,
            },
        );
        Ok(FileSearchScope::new(owner))
    }
    /// Incarnation equality, not a home/path or connection-ID comparison.
    pub fn owns_scope(&self, scope: &FileSearchScope) -> bool {
        self.inner.ptr_eq(&scope.inner().provider)
    }
}

impl Provider {
    pub(crate) fn request_shutdown(self: &Arc<Self>) {
        let token = self.tasks.token();
        let scopes = {
            let mut state = lock(&self.state);
            if state.closing {
                return;
            }
            state.closing = true;
            state
                .scopes
                .values()
                .map(|entry| Arc::clone(&entry.owner))
                .collect::<Vec<_>>()
        };
        for scope in scopes {
            scope.request_shutdown();
        }
        if std::panic::catch_unwind(AssertUnwindSafe(|| self.backend.request_shutdown())).is_err() {
            lock(&self.state).first_error.get_or_insert_with(panicked);
        }
        // Scope/lease cleanup tokens are now registered. This coordinator is
        // outside the tracker it drains; the pre-fence token closes the gap.
        self.tasks.close();
        drop(token);
        let owner = Arc::clone(self);
        let guard = ProviderCompletionGuard(Some(Arc::clone(&owner)));
        self.runtime.spawn(async move {
            let mut guard = guard;
            let mut outcome = match AssertUnwindSafe(async { owner.backend.shutdown().await })
                .catch_unwind()
                .await
            {
                Ok(outcome) => outcome,
                Err(_) => uncertain(panicked()),
            };
            let unconfirmed = matches!(outcome.cleanup, CloseCleanup::Unconfirmed(_));
            // Owned backend shutdown can wake accepted operations even if its
            // nonblocking hook panicked. Joined still requires local callbacks
            // and receipts to drain; uncertainty may be reported before that.
            if !unconfirmed {
                owner.tasks.wait().await;
            }
            {
                let mut state = lock(&owner.state);
                if let Some(error) = &state.first_error {
                    outcome.operation = Err(error.clone());
                }
                if outcome.cleanup == CloseCleanup::Joined {
                    // Separate whole-provider receipt proves quarantined work
                    // drained. Earlier lease/scope receipts remain immutable.
                    state.scopes.clear();
                    state.sessions = 0;
                    state.used = [0; 3];
                }
            }
            complete(&owner.completed, outcome);
            if unconfirmed {
                owner.tasks.wait().await;
            }
            guard.0.take();
        });
    }
}
struct ProviderCompletionGuard(Option<Arc<Provider>>);
impl Drop for ProviderCompletionGuard {
    fn drop(&mut self) {
        if let Some(owner) = self.0.take() {
            complete(&owner.completed, uncertain(lost()));
        }
    }
}
