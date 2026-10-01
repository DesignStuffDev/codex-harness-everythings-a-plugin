use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use codex_file_search::NativeBackendLimits;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchError;
use codex_file_search_component::ServiceLimits;

use crate::RuntimePolicy;
use crate::policy::invalid;

/// Explicit immutable context. Relative query roots keep their original lexical
/// form and are interpreted against `base_dir`; selection never changes cwd.
#[derive(Debug, Clone)]
pub struct SelectionContext {
    pub codex_home: PathBuf,
    pub base_dir: PathBuf,
}

/// Composition policy, without universal defaults or an RSS claim. Native
/// generation, receiver-retained frames and serialized process frames use
/// different accounting; none is silently substituted for another.
#[derive(Debug, Clone)]
pub struct SelectionPolicy {
    pub runtime: RuntimePolicy,
    pub native: NativeBackendLimits,
    pub process: ServiceLimits,
}

impl SelectionContext {
    pub(crate) fn validate(&self) -> Result<(), SearchError> {
        if !self.codex_home.is_absolute() || !self.base_dir.is_absolute() {
            return Err(invalid(
                "file-search selection requires absolute home and base directories",
            ));
        }
        let current = std::env::current_dir()
            .map_err(|_| invalid("file-search working directory is unavailable"))?;
        if self.base_dir.as_os_str() != current.as_os_str() {
            return Err(invalid(
                "file-search base must equal the immutable caller working directory",
            ));
        }
        Ok(())
    }
}

impl SelectionPolicy {
    pub(crate) fn validate(&self) -> Result<(), SearchError> {
        self.runtime.validate()?;
        self.process.validate()?;
        if self.native.max_poll_wait.is_zero()
            || self.native.max_poll_wait > Duration::from_secs(60)
        {
            return Err(invalid(
                "file-search native poll ceiling must be positive and <=60 seconds",
            ));
        }
        Ok(())
    }

    pub(crate) fn for_native(&self) -> Result<RuntimePolicy, SearchError> {
        let mut policy = self.runtime;
        policy.provider.max_sessions = policy.provider.max_sessions.min(self.native.max_sessions);
        policy.provider.resources =
            minimum_budget(policy.provider.resources, self.native.resources);
        policy.max_query_bytes = policy.max_query_bytes.min(self.native.max_query_bytes);
        // Both of these are native logical payload plus Vec-element headers.
        policy.max_roots_options_bytes = policy
            .max_roots_options_bytes
            .min(self.native.max_roots_options_bytes);
        policy.max_matches = policy.max_matches.min(self.native.max_matches);
        policy.poll_wait = policy.poll_wait.min(self.native.max_poll_wait);
        policy.validate()?;
        Ok(policy)
    }

    pub(crate) fn for_process(&self, limits: &ServiceLimits) -> Result<RuntimePolicy, SearchError> {
        limits.validate_negotiated(&self.process)?;
        let mut policy = self.runtime;
        policy.provider.max_sessions = policy
            .provider
            .max_sessions
            .min(nonzero(limits.max_leases)?);
        policy.provider.resources =
            minimum_budget(policy.provider.resources, limits.resources.into_native()?);
        policy.max_query_bytes = policy
            .max_query_bytes
            .min(nonzero(limits.max_query_utf8_bytes)?);
        policy.max_matches = policy.max_matches.min(nonzero(limits.max_matches)?);
        policy.poll_wait = policy
            .poll_wait
            .min(Duration::from_millis(u64::from(limits.max_poll_wait_ms)));
        // Process roots/options and frames are encoded-byte limits. Keep the
        // receiver's logical/backing-allocation caps independently enforced.
        policy.validate()?;
        Ok(policy)
    }
}

fn minimum_budget(left: SearchBudget, right: SearchBudget) -> SearchBudget {
    SearchBudget {
        max_index_entries: left.max_index_entries.min(right.max_index_entries),
        max_index_bytes: left.max_index_bytes.min(right.max_index_bytes),
        max_worker_threads: left.max_worker_threads.min(right.max_worker_threads),
    }
}
fn nonzero(value: u32) -> Result<NonZeroUsize, SearchError> {
    usize::try_from(value)
        .ok()
        .and_then(NonZeroUsize::new)
        .ok_or_else(|| invalid("file-search negotiated ceiling is not representable"))
}
