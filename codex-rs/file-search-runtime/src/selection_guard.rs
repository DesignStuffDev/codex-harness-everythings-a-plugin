use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use futures::FutureExt;

/// Armed before the first post-start fallible operation. A cancelled observer
/// cannot become the sole owner of an already-started backend's drain.
pub(super) struct BackendGuard {
    backend: Option<Arc<dyn SearchBackend>>,
    runtime: tokio::runtime::Handle,
}
impl BackendGuard {
    pub(super) fn new(backend: Arc<dyn SearchBackend>) -> Self {
        Self {
            backend: Some(backend),
            runtime: tokio::runtime::Handle::current(),
        }
    }
    pub(super) fn disarm(mut self) {
        self.backend.take();
    }
    pub(super) async fn shutdown(mut self) -> SearchCloseOutcome {
        let Some(backend) = self.backend.as_ref() else {
            unreachable!("armed selection cleanup owner")
        };
        let result = drain(Arc::clone(backend)).await;
        self.backend.take();
        result
    }
}
impl Drop for BackendGuard {
    fn drop(&mut self) {
        if let Some(backend) = self.backend.take() {
            // request_shutdown is advisory; the independently owned shutdown
            // future remains the authoritative drain even if the hook panics.
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| backend.request_shutdown()));
            self.runtime.spawn(async move {
                let _ = drain(backend).await;
            });
        }
    }
}
async fn drain(backend: Arc<dyn SearchBackend>) -> SearchCloseOutcome {
    match AssertUnwindSafe(async { backend.shutdown().await })
        .catch_unwind()
        .await
    {
        Ok(outcome) => outcome,
        Err(_) => {
            let error = SearchError::new(
                SearchErrorKind::SearchFailed,
                "file-search selected backend panicked during cleanup",
            );
            SearchCloseOutcome {
                operation: Err(error.clone()),
                cleanup: CloseCleanup::Unconfirmed(error),
            }
        }
    }
}
