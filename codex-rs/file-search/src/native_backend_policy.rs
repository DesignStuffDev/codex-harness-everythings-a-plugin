//! Explicit native admission policy; byte charges are not serialized wire sizes.

use std::num::NonZeroUsize;
use std::path::Path;
use std::time::Duration;

use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;

use crate::build_override_matcher;
use crate::native_budget::NativeBudget;
use crate::native_output::NativeOutputLimits;
use crate::native_output::exact_path;
use crate::native_output::exact_string;
use crate::native_output::exact_vec;

/// No implicit resource defaults. Composition supplies one policy per provider.
#[derive(Debug, Clone, Copy)]
pub struct NativeBackendLimits {
    pub max_sessions: NonZeroUsize,
    pub resources: SearchBudget,
    pub max_query_bytes: NonZeroUsize,
    pub max_roots_options_bytes: NonZeroUsize,
    pub max_matches: NonZeroUsize,
    /// Maximum charged native snapshot allocation, including capacities and
    /// path/root/query/indices payload. This is not wire-frame size or process RSS.
    pub max_snapshot_bytes: NonZeroUsize,
    pub max_poll_wait: Duration,
}

impl NativeBackendLimits {
    pub(super) fn output(self) -> NativeOutputLimits {
        NativeOutputLimits {
            max_query_bytes: self.max_query_bytes,
            max_snapshot_bytes: self.max_snapshot_bytes,
        }
    }

    pub(super) fn validate(self) -> Result<(), SearchError> {
        if self.max_poll_wait.is_zero() || self.max_poll_wait > Duration::from_secs(60) {
            return Err(invalid(
                "native file-search poll ceiling must be positive and at most 60 seconds",
            ));
        }
        Ok(())
    }

    pub(super) fn prepare_open(
        self,
        mut request: SearchOpen,
        base: &Path,
    ) -> Result<SearchOpen, SearchError> {
        self.validate_open(&request, base)?;
        // Caller-owned spare capacity is not part of a logical input limit.
        // Compact into exact fresh storage before retaining provider-owned work.
        let mut roots = exact_vec(request.roots.len())?;
        for root in &request.roots {
            roots.push(exact_path(root)?);
        }
        let mut excludes = exact_vec(request.options.exclude.len())?;
        for exclude in &request.options.exclude {
            excludes.push(exact_string(exclude)?);
        }
        request.roots = roots;
        request.options.exclude = excludes;
        Ok(request)
    }

    pub(super) fn validate_open(
        self,
        request: &SearchOpen,
        base: &Path,
    ) -> Result<(), SearchError> {
        if std::env::current_dir()
            .map_err(|_| invalid("native file-search working directory is unavailable"))?
            .as_os_str()
            != base.as_os_str()
        {
            return Err(invalid(
                "native file-search working directory differs from its immutable context",
            ));
        }
        let Some(primary) = request.roots.first() else {
            return Err(invalid("at least one search directory is required"));
        };
        if request.options.limit > self.max_matches {
            return Err(exhausted(
                "native file-search match limit exceeds provider policy",
            ));
        }
        request.budget.validate_within(&self.resources)?;
        let mut bytes = request
            .roots
            .len()
            .checked_mul(std::mem::size_of::<std::path::PathBuf>())
            .and_then(|bytes| {
                request
                    .options
                    .exclude
                    .len()
                    .checked_mul(std::mem::size_of::<String>())
                    .and_then(|excludes| bytes.checked_add(excludes))
            })
            .ok_or_else(|| exhausted("native file-search input allocation size overflow"))?;
        for size in request
            .roots
            .iter()
            .map(|path| path.as_os_str().as_encoded_bytes().len())
            .chain(request.options.exclude.iter().map(String::len))
        {
            bytes = bytes
                .checked_add(size)
                .ok_or_else(|| exhausted("native file-search input allocation size overflow"))?;
        }
        if bytes > self.max_roots_options_bytes.get() {
            return Err(exhausted(
                "native file-search roots/options exceed provider policy",
            ));
        }
        build_override_matcher(primary, &request.options.exclude)
            .map_err(|_| invalid("invalid file-search exclusion pattern"))?;
        // Clean policy errors are rejected before owner admission and therefore
        // cannot poison its retained shutdown history. Actual allocation remains
        // fallible during admitted startup and keeps its typed cleanup receipt.
        NativeBudget::prepare(&request.options, request.budget)?;
        Ok(())
    }
}

pub(super) fn invalid(message: &'static str) -> SearchError {
    SearchError::new(SearchErrorKind::InvalidInput, message)
}
pub(super) fn exhausted(message: &'static str) -> SearchError {
    SearchError::new(SearchErrorKind::ResourceExhausted, message)
}
pub(super) fn closed() -> SearchError {
    SearchError::new(
        SearchErrorKind::ClosedLease,
        "native file-search admission is closed",
    )
}
pub(super) fn failed(message: &'static str) -> SearchError {
    SearchError::new(SearchErrorKind::SearchFailed, message)
}
