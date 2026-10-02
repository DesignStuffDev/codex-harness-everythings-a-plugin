//! Per-processor featured task custody, independent of curated worker ownership.

use codex_core_plugins::FeaturedWarmupScope;
use std::io;
use std::sync::Arc;

pub(crate) struct FeaturedWarmupGuard {
    scope: Arc<FeaturedWarmupScope>,
}

impl FeaturedWarmupGuard {
    pub(crate) fn new() -> Self {
        Self::from_scope(FeaturedWarmupScope::new())
    }

    pub(crate) fn from_scope(scope: Arc<FeaturedWarmupScope>) -> Self {
        Self { scope }
    }

    pub(crate) fn fork(&self) -> Self {
        Self::from_scope(self.scope())
    }

    pub(crate) fn scope(&self) -> Arc<FeaturedWarmupScope> {
        Arc::clone(&self.scope)
    }

    pub(crate) fn begin_close(&self) {
        self.scope.begin_close();
    }

    /// Observe only until the caller's already-established teardown deadline.
    /// Expiry and cancelled waits retain primary custody; this adds no grace.
    pub(crate) async fn finish<T>(
        &self,
        result: io::Result<T>,
        deadline: tokio::time::Instant,
    ) -> io::Result<T> {
        self.begin_close();
        let observation = self.scope.wait_until(deadline.into_std()).await;
        tracing::info!(
            target: "codex_core_plugins::lifecycle",
            event = "featured_warmup_scope_shutdown_observed",
            receipt_version = 1_u64,
            task_ownership = ?observation.ownership,
            optional_fetch_outcome = ?observation.outcome,
            task_ownership_clean = observation.is_complete(),
            "featured warmup scope shutdown observed"
        );
        let cleanup = if observation.is_complete() {
            Ok(())
        } else {
            Err(io::Error::other("featured warmup task cleanup unconfirmed"))
        };
        crate::file_search_services::combine(result, cleanup)
    }
}

impl Drop for FeaturedWarmupGuard {
    fn drop(&mut self) {
        self.begin_close();
    }
}
