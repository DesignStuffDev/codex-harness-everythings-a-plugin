//! Executable-only aggregation; embedded replacement closes local scopes only.

use crate::featured_warmup::FeaturedWarmupDrain;
use crate::featured_warmup::FeaturedWarmupProcessObservation;
use crate::featured_warmup::begin_featured_warmup_process_close;
use crate::startup_sync::CuratedProcessShutdown;
use crate::startup_sync::CuratedProcessShutdownObservation;
use std::time::Instant;

/// Observe separately owned startup work without expanding the curated receipt.
/// Neither this object nor a successful task join proves pooled HTTP shutdown.
pub struct PluginStartupProcessShutdown {
    curated: CuratedProcessShutdown,
    featured: FeaturedWarmupDrain,
    deadline: Instant,
}

impl PluginStartupProcessShutdown {
    /// Executable composition supplies its existing, nonrenewable grace deadline.
    /// Never call this to replace an embedded processor in a continuing process.
    pub fn begin(deadline: Instant) -> Self {
        // Preserve existing native-stop onset before waiting for a scoped
        // publication critical section to release its cache lock.
        let curated = CuratedProcessShutdown::begin(deadline);
        let featured = begin_featured_warmup_process_close();
        Self {
            curated,
            featured,
            deadline,
        }
    }

    pub async fn wait_until(&self, deadline: Instant) -> PluginStartupProcessObservation {
        let deadline = deadline.min(self.deadline);
        let (curated, featured) = tokio::join!(
            self.curated.wait_until(deadline),
            self.featured.wait_until(deadline),
        );
        tracing::info!(
            target: "codex_core_plugins::lifecycle",
            event = "featured_warmup_process_shutdown_observed",
            receipt_version = 1_u64,
            task_pending = featured.pending,
            task_joined = featured.joined,
            task_panicked = featured.panicked,
            task_join_failed = featured.join_failed,
            optional_fetch_failed = featured.failed,
            task_ownership_clean = featured.is_complete(),
            "featured warmup task shutdown observed"
        );
        PluginStartupProcessObservation { curated, featured }
    }
}

/// Per-owner evidence; transport/native descendant cleanup has independent gates.
pub struct PluginStartupProcessObservation {
    pub curated: CuratedProcessShutdownObservation,
    pub featured: FeaturedWarmupProcessObservation,
}

impl PluginStartupProcessObservation {
    pub fn is_complete(&self) -> bool {
        self.curated.is_complete() && self.featured.is_complete()
    }
}
