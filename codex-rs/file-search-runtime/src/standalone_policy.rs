use std::num::NonZeroUsize;
use std::time::Duration;

use codex_file_search::NativeBackendLimits;
use codex_file_search_api::ProviderLimits;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchError;
use codex_file_search_component::ServiceLimits;

use crate::CliSearchPolicy;
use crate::RuntimePolicy;
use crate::SelectionPolicy;
use crate::policy::exhausted;

/// Initial bounded standalone profile; an operational ceiling, not a benchmark
/// optimum or RSS bound. 100k entries / 128 MiB charged index bytes were checked
/// against the actual 64-bit native allocation plan at two matcher threads:
/// fixed floor 9,605,104 bytes, or 9,738,224 with highlight scratch. Path/column
/// payloads are charged separately during traversal and can exhaust that cap.
///
/// `threads` keeps the existing CLI meaning. All dedicated native workers are
/// budgeted as 2*T+2, up to 64. A selected component may negotiate lower limits;
/// the explicit allocation then fails rather than silently changing options.
pub fn standalone_policy(threads: NonZeroUsize) -> Result<CliSearchPolicy, SearchError> {
    let workers = threads
        .get()
        .checked_mul(2)
        .and_then(|value| value.checked_add(2))
        .filter(|value| *value <= 64)
        .ok_or_else(|| {
            exhausted("file-search --threads requires more than the 64-worker standalone ceiling")
        })?;
    let resources = SearchBudget {
        max_index_entries: positive(100_000)?,
        max_index_bytes: positive(128 * 1024 * 1024)?,
        max_worker_threads: positive(64)?,
    };
    let max_query_bytes = positive(64 * 1024)?;
    let max_roots_options_bytes = positive(256 * 1024)?;
    let max_matches = positive(1024)?;
    let native_snapshot_bytes = positive(16 * 1024 * 1024)?;
    let receiver_frame_bytes = positive(16 * 1024 * 1024)?;
    let poll_wait = Duration::from_secs(1);
    let mut process = ServiceLimits::new(resources);
    process.max_leases = 1;
    Ok(CliSearchPolicy {
        selection: SelectionPolicy {
            runtime: RuntimePolicy {
                provider: ProviderLimits {
                    max_scopes: NonZeroUsize::MIN,
                    max_sessions: NonZeroUsize::MIN,
                    resources,
                },
                max_query_bytes,
                max_roots_options_bytes,
                max_matches,
                max_frame_retained_bytes: receiver_frame_bytes,
                poll_wait,
            },
            native: NativeBackendLimits {
                max_sessions: NonZeroUsize::MIN,
                resources,
                max_query_bytes,
                max_roots_options_bytes,
                max_matches,
                max_snapshot_bytes: native_snapshot_bytes,
                max_poll_wait: poll_wait,
            },
            process,
        },
        session_budget: SearchBudget {
            max_worker_threads: positive(workers)?,
            ..resources
        },
    })
}
fn positive(value: usize) -> Result<NonZeroUsize, SearchError> {
    NonZeroUsize::new(value)
        .ok_or_else(|| exhausted("file-search standalone policy contains a zero ceiling"))
}
