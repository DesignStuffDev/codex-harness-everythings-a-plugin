//! Per-processor curated callbacks, independent of process-wide worker stop.

use codex_core_plugins::startup_sync::CuratedCallbackScope;
use std::io;
use std::sync::Arc;

pub(crate) struct CuratedCallbackGuard {
    scope: Arc<CuratedCallbackScope>,
}

impl CuratedCallbackGuard {
    pub(crate) fn new() -> Self {
        Self::from_scope(CuratedCallbackScope::new(tokio::runtime::Handle::current()))
    }

    pub(crate) fn from_scope(scope: Arc<CuratedCallbackScope>) -> Self {
        Self { scope }
    }

    pub(crate) fn fork(&self) -> Self {
        Self::from_scope(self.scope())
    }

    pub(crate) fn scope(&self) -> Arc<CuratedCallbackScope> {
        Arc::clone(&self.scope)
    }

    pub(crate) fn begin_close(&self) {
        self.scope.begin_close();
    }

    /// Existing processor/runtime timeouts can cancel this observation without
    /// discarding primary task custody. Final shared deadlines belong to the
    /// explicit host owner; this method adds no independent timeout budget.
    pub(crate) async fn finish<T>(&self, result: io::Result<T>) -> io::Result<T> {
        let observation = self.scope.wait().await;
        let cleanup = if observation.is_complete() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "curated callback cleanup unconfirmed: {} pending, {} failed",
                observation.pending, observation.failed,
            )))
        };
        crate::file_search_services::combine(result, cleanup)
    }
}

impl Drop for CuratedCallbackGuard {
    fn drop(&mut self) {
        self.begin_close();
    }
}

#[cfg(test)]
#[path = "curated_callback_lifecycle_tests.rs"]
mod tests;
