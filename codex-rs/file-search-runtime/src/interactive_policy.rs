use std::num::NonZeroUsize;
use std::time::Duration;

use codex_file_search::NativeBackendLimits;
use codex_file_search_api::ProviderLimits;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchError;
use codex_file_search_component::ServiceLimits;

use crate::RuntimePolicy;
use crate::SelectionPolicy;
use crate::policy::exhausted;

/// One interactive composition's accounting ceilings, shared across App Server
/// connections and a local TUI picker. This is not an OS-wide or RSS quota, and
/// independent embeddings remain independent even when their Codex homes match.
/// The worker cap may constrain concurrency before the 16-session metadata cap.
pub fn interactive_policy() -> Result<SelectionPolicy, SearchError> {
    let resources = SearchBudget {
        max_index_entries: positive(1_600_000)?,
        max_index_bytes: positive(2 * 1024 * 1024 * 1024)?,
        max_worker_threads: positive(64)?,
    };
    let max_sessions = positive(16)?;
    let max_query_bytes = positive(64 * 1024)?;
    let max_roots_options_bytes = positive(256 * 1024)?;
    let max_matches = positive(1024)?;
    let native_snapshot_bytes = positive(16 * 1024 * 1024)?;
    let receiver_frame_bytes = positive(16 * 1024 * 1024)?;
    let poll_wait = Duration::from_secs(1);
    Ok(SelectionPolicy {
        runtime: RuntimePolicy {
            provider: ProviderLimits {
                max_scopes: positive(64)?,
                max_sessions,
                resources,
            },
            max_query_bytes,
            max_roots_options_bytes,
            max_matches,
            max_frame_retained_bytes: receiver_frame_bytes,
            poll_wait,
        },
        native: NativeBackendLimits {
            max_sessions,
            resources,
            max_query_bytes,
            max_roots_options_bytes,
            max_matches,
            max_snapshot_bytes: native_snapshot_bytes,
            max_poll_wait: poll_wait,
        },
        process: ServiceLimits::new(resources),
    })
}

/// Explicit per-lease allocation; unsupported selected-provider budgets fail
/// admission rather than changing roots, thread options or the index silently.
/// Native worker ownership includes supervisor, walker, ignore and matcher work.
pub fn interactive_allocation(threads: NonZeroUsize) -> Result<SearchBudget, SearchError> {
    let workers = threads
        .get()
        .checked_mul(2)
        .and_then(|value| value.checked_add(2))
        .filter(|value| *value <= 64)
        .ok_or_else(|| {
            exhausted("file-search interactive allocation exceeds its 64-worker ceiling")
        })?;
    Ok(SearchBudget {
        max_index_entries: positive(100_000)?,
        max_index_bytes: positive(128 * 1024 * 1024)?,
        max_worker_threads: positive(workers)?,
    })
}
fn positive(value: usize) -> Result<NonZeroUsize, SearchError> {
    NonZeroUsize::new(value)
        .ok_or_else(|| exhausted("file-search interactive policy contains a zero ceiling"))
}
