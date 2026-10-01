//! The actual native walker and matcher behind the versioned search service.
//!
//! This composition has no dependency on Core, App Server, presentation or the
//! callback runtime. Selection and per-session allocations belong to the host.

use std::num::NonZeroUsize;
use std::sync::Arc;

use codex_file_search::NativeSearchBackend;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use codex_file_search_component::BackendContext;
use codex_file_search_component::BackendFactoryFuture;
use codex_file_search_component::SearchBackendFactory;
use codex_file_search_component::ServiceLimits;

mod policy;

/// Initial worker admission ceilings, not implicit per-session allocations.
/// The host negotiates these downward and explicitly supplies every open budget.
pub fn service_ceilings() -> Result<ServiceLimits, SearchError> {
    Ok(ServiceLimits::new(SearchBudget {
        max_index_entries: positive(1_000_000)?,
        max_index_bytes: positive(512 * 1024 * 1024)?,
        max_worker_threads: positive(64)?,
    }))
}

/// Compose the persistent service with native Codex traversal/matching.
pub async fn run_stdio() -> anyhow::Result<()> {
    codex_file_search_component::run_stdio(Arc::new(NativeFactory), service_ceilings()?).await
}

struct NativeFactory;

impl SearchBackendFactory for NativeFactory {
    fn create(&self, context: BackendContext) -> BackendFactoryFuture<'_> {
        Box::pin(async move {
            let limits = policy::native_limits(&context.limits, context.plugin_config)
                .map_err(not_admitted)?;
            let backend =
                NativeSearchBackend::new(context.base_dir, limits).map_err(not_admitted)?;
            let backend: Arc<dyn SearchBackend> = Arc::new(backend);
            Ok(backend)
        })
    }
}

fn positive(value: usize) -> Result<NonZeroUsize, SearchError> {
    NonZeroUsize::new(value).ok_or_else(|| {
        SearchError::new(
            SearchErrorKind::InvalidInput,
            "native-search policy must use positive limits",
        )
    })
}

fn not_admitted(operation: SearchError) -> SearchStartError {
    SearchStartError {
        operation,
        cleanup: StartCleanup::NotAdmitted,
    }
}
