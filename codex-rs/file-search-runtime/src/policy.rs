use std::num::NonZeroUsize;
use std::path::Component;
use std::time::Duration;

use codex_file_search_api::FileMatch;
use codex_file_search_api::ProviderLimits;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchFrame;
use codex_file_search_api::SearchOpen;

/// Explicit receiver/composition policy. None of these bounds claims total RSS.
#[derive(Clone, Copy, Debug)]
pub struct RuntimePolicy {
    pub provider: ProviderLimits,
    /// Raw UTF-8 bytes of a submitted query, checked before retained copying.
    pub max_query_bytes: NonZeroUsize,
    /// Logical roots/options payload plus retained Vec element headers. Incoming
    /// caller spare capacity is not counted; admitted copies are compacted.
    pub max_roots_options_bytes: NonZeroUsize,
    pub max_matches: NonZeroUsize,
    /// Actual backing capacities in a received frame: frame query String plus
    /// snapshot query String, matches Vec, path/root PathBuf and indices Vec.
    /// This receiver charge differs from native generation allocations and exact
    /// serialized JSON bytes; callback-owned copies are outside this owner's cap.
    pub max_frame_retained_bytes: NonZeroUsize,
    /// Positive whole milliseconds, <= 1000ms and the selected backend ceiling.
    pub poll_wait: Duration,
}

impl RuntimePolicy {
    pub(crate) fn validate(&self) -> Result<(), SearchError> {
        if self.poll_wait.is_zero()
            || self.poll_wait > Duration::from_secs(1)
            || !self.poll_wait.subsec_nanos().is_multiple_of(1_000_000)
        {
            return Err(invalid(
                "file-search runtime poll wait must be positive whole milliseconds <=1000",
            ));
        }
        Ok(())
    }

    pub(crate) fn compact_open(&self, request: SearchOpen) -> Result<SearchOpen, SearchError> {
        request.budget.validate_within(&self.provider.resources)?;
        if request.options.limit > self.max_matches {
            return Err(exhausted(
                "file-search requested match count exceeds receiver policy",
            ));
        }
        let mut charge = request
            .roots
            .len()
            .checked_mul(std::mem::size_of::<std::path::PathBuf>())
            .and_then(|v| {
                request
                    .options
                    .exclude
                    .len()
                    .checked_mul(std::mem::size_of::<String>())
                    .and_then(|n| v.checked_add(n))
            })
            .ok_or_else(|| exhausted("file-search input charge overflow"))?;
        for bytes in request
            .roots
            .iter()
            .map(|p| p.as_os_str().as_encoded_bytes().len())
            .chain(request.options.exclude.iter().map(String::len))
        {
            charge = charge
                .checked_add(bytes)
                .ok_or_else(|| exhausted("file-search input charge overflow"))?;
            if charge > self.max_roots_options_bytes.get() {
                return Err(exhausted(
                    "file-search roots/options exceed receiver policy",
                ));
            }
        }
        if charge > self.max_roots_options_bytes.get() {
            return Err(exhausted(
                "file-search roots/options exceed receiver policy",
            ));
        }
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(request.roots.len())
            .map_err(|_| exhausted("file-search roots allocation failed"))?;
        for source in request.roots {
            let mut path = std::path::PathBuf::new();
            path.try_reserve_exact(source.as_os_str().as_encoded_bytes().len())
                .map_err(|_| exhausted("file-search root allocation failed"))?;
            path.push(source);
            roots.push(path);
        }
        let mut exclude = Vec::new();
        exclude
            .try_reserve_exact(request.options.exclude.len())
            .map_err(|_| exhausted("file-search exclusions allocation failed"))?;
        for source in request.options.exclude {
            exclude.push(copy_query(&source)?);
        }
        Ok(SearchOpen {
            roots,
            options: codex_file_search_api::FileSearchOptions {
                exclude,
                ..request.options
            },
            budget: request.budget,
        })
    }

    pub(crate) fn validate_frame(
        &self,
        frame: &SearchFrame,
        roots: &[std::path::PathBuf],
        match_limit: usize,
    ) -> Result<(), SearchError> {
        frame.validate()?;
        if frame.query.len() > self.max_query_bytes.get() {
            return Err(exhausted("file-search frame query exceeds receiver policy"));
        }
        let mut charge = frame.query.capacity();
        if let Some(snapshot) = &frame.snapshot {
            if snapshot.matches.len() > match_limit
                || snapshot.matches.len() > self.max_matches.get()
            {
                return Err(exhausted(
                    "file-search frame match count exceeds receiver policy",
                ));
            }
            charge = snapshot
                .matches
                .capacity()
                .checked_mul(std::mem::size_of::<FileMatch>())
                .and_then(|bytes| charge.checked_add(bytes))
                .and_then(|bytes| bytes.checked_add(snapshot.query.capacity()))
                .ok_or_else(|| exhausted("file-search frame charge overflow"))?;
            for matched in &snapshot.matches {
                if !roots
                    .iter()
                    .any(|root| root.as_os_str() == matched.root.as_os_str())
                    || matched.path.components().any(|part| {
                        matches!(
                            part,
                            Component::RootDir | Component::Prefix(_) | Component::ParentDir
                        )
                    })
                    || matched
                        .indices
                        .as_ref()
                        .is_some_and(|indices| indices.windows(2).any(|pair| pair[0] >= pair[1]))
                {
                    return Err(invalid(
                        "file-search frame path or highlight indices are invalid",
                    ));
                }
                charge = charge
                    .checked_add(matched.path.capacity())
                    .and_then(|bytes| bytes.checked_add(matched.root.capacity()))
                    .and_then(|bytes| {
                        matched.indices.as_ref().map_or(Some(bytes), |indices| {
                            indices
                                .capacity()
                                .checked_mul(std::mem::size_of::<u32>())
                                .and_then(|value| bytes.checked_add(value))
                        })
                    })
                    .ok_or_else(|| exhausted("file-search frame charge overflow"))?;
                if charge > self.max_frame_retained_bytes.get() {
                    return Err(exhausted(
                        "file-search frame backing allocations exceed receiver policy",
                    ));
                }
            }
        }
        if charge > self.max_frame_retained_bytes.get() {
            return Err(exhausted(
                "file-search frame backing allocations exceed receiver policy",
            ));
        }
        Ok(())
    }
}

pub(crate) fn copy_query(source: &str) -> Result<String, SearchError> {
    let mut result = String::new();
    result
        .try_reserve_exact(source.len())
        .map_err(|_| exhausted("file-search text allocation failed"))?;
    result.push_str(source);
    Ok(result)
}
pub(crate) fn invalid(message: &'static str) -> SearchError {
    SearchError::new(SearchErrorKind::InvalidInput, message)
}
pub(crate) fn exhausted(message: &'static str) -> SearchError {
    SearchError::new(SearchErrorKind::ResourceExhausted, message)
}
