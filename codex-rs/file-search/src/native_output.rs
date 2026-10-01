//! Bounds one native callback snapshot's requested backing allocations.
//!
//! This is not a wire-size or RSS limit. Index storage, parsed patterns, queued
//! query copies, and client-owned snapshot copies have separate lifetime bounds.
//! Accounting uses the pinned Rust Global allocator's exact fresh capacities.

use crate::FileMatch;
use crate::FileSearchSnapshot;
use crate::IndexedEntry;
use crate::SessionInner;
use crate::get_file_path;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use nucleo::Matcher;
use std::alloc::Layout;
use std::ffi::OsString;
use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;

/// Output policy supplied only by the native backend's bounded composition.
#[derive(Clone, Copy)]
pub(crate) struct NativeOutputLimits {
    pub max_query_bytes: NonZeroUsize,
    pub max_snapshot_bytes: NonZeroUsize,
}

impl NativeOutputLimits {
    pub(crate) fn validate_query(self, query: &str) -> Result<(), SearchError> {
        if query.len() > self.max_query_bytes.get() {
            return Err(resource("native file-search query exceeds its byte limit"));
        }
        Ok(())
    }
}

pub(super) fn snapshot(
    inner: &SessionInner,
    snapshot: &nucleo::Snapshot<IndexedEntry>,
    indices_matcher: &mut Option<Matcher>,
    query_id: u64,
    query: &str,
    walk_complete: bool,
    limits: NativeOutputLimits,
) -> Result<FileSearchSnapshot, SearchError> {
    limits.validate_query(query)?;
    let limit = inner.limit.min(snapshot.matched_item_count() as usize);
    let pattern = snapshot.pattern().column_pattern(0);
    // Each positive atom appends at most its needle's grapheme count. Negative
    // atoms append nothing. Dedup retains this prepaid capacity afterward.
    let indices_capacity = if indices_matcher.is_some() {
        pattern
            .atoms
            .iter()
            .filter(|atom| !atom.negative)
            .try_fold(0usize, |count, atom| {
                count
                    .checked_add(atom.needle_text().len())
                    .ok_or_else(|| resource("native file-search highlight size overflow"))
            })?
    } else {
        0
    };
    let indices_bytes = Layout::array::<u32>(indices_capacity)
        .map_err(|_| resource("native file-search highlight layout overflow"))?
        .size();
    let mut count = 0usize;
    let mut payload_bytes = query.len();
    // Preflight the same immutable top-N view before cloning any result. Do not
    // allocate an auxiliary plan vector whose size would need another charge.
    for matched in snapshot.matches().iter().take(limit) {
        let Some(item) = snapshot.get_item(matched.idx) else {
            continue;
        };
        let Some((root_idx, relative_path)) = get_file_path(
            Path::new(item.data.full_path.as_ref()),
            &inner.search_directories,
        ) else {
            continue;
        };
        count = count
            .checked_add(1)
            .ok_or_else(|| resource("native file-search result count overflow"))?;
        for bytes in [
            Path::new(relative_path)
                .as_os_str()
                .as_encoded_bytes()
                .len(),
            inner.search_directories[root_idx]
                .as_os_str()
                .as_encoded_bytes()
                .len(),
            indices_bytes,
        ] {
            payload_bytes = payload_bytes
                .checked_add(bytes)
                .ok_or_else(|| resource("native file-search snapshot size overflow"))?;
        }
        if payload_bytes > limits.max_snapshot_bytes.get() {
            return Err(resource(
                "native file-search snapshot exceeds its allocation limit",
            ));
        }
    }
    let match_bytes = Layout::array::<FileMatch>(count)
        .map_err(|_| resource("native file-search result layout overflow"))?
        .size();
    let charged_bytes = payload_bytes
        .checked_add(match_bytes)
        .ok_or_else(|| resource("native file-search snapshot size overflow"))?;
    if charged_bytes > limits.max_snapshot_bytes.get() {
        return Err(resource(
            "native file-search snapshot exceeds its allocation limit",
        ));
    }

    let mut matches = exact_vec(count)?;
    let query = exact_string(query)?;
    for matched in snapshot.matches().iter().take(limit) {
        let Some(item) = snapshot.get_item(matched.idx) else {
            continue;
        };
        let Some((root_idx, relative_path)) = get_file_path(
            Path::new(item.data.full_path.as_ref()),
            &inner.search_directories,
        ) else {
            continue;
        };
        let indices = if let Some(matcher) = indices_matcher.as_mut() {
            let mut indices = exact_vec(indices_capacity)?;
            let _ = pattern.indices(item.matcher_columns[0].slice(..), matcher, &mut indices);
            if indices.capacity() != indices_capacity || indices.len() > indices_capacity {
                return Err(resource(
                    "native file-search highlights exceeded their prepaid capacity",
                ));
            }
            indices.sort_unstable();
            indices.dedup();
            Some(indices)
        } else {
            None
        };
        if matches.len() == count {
            return Err(resource(
                "native file-search snapshot changed during construction",
            ));
        }
        matches.push(FileMatch {
            score: matched.score,
            path: exact_path(Path::new(relative_path))?,
            match_type: item.data.match_type,
            root: exact_path(&inner.search_directories[root_idx])?,
            indices,
        });
    }
    if matches.len() != count {
        return Err(resource(
            "native file-search snapshot changed during construction",
        ));
    }
    Ok(FileSearchSnapshot {
        query_id,
        query,
        matches,
        total_match_count: snapshot.matched_item_count() as usize,
        scanned_file_count: snapshot.item_count() as usize,
        walk_complete,
    })
}

pub(super) fn exact_vec<T>(count: usize) -> Result<Vec<T>, SearchError> {
    let mut value = Vec::new();
    value
        .try_reserve_exact(count)
        .map_err(|_| resource("native file-search output allocation failed"))?;
    if std::mem::size_of::<T>() != 0 && value.capacity() != count {
        return Err(resource(
            "native file-search output allocator differs from its exact charge",
        ));
    }
    Ok(value)
}

pub(super) fn exact_string(source: &str) -> Result<String, SearchError> {
    let mut value = String::new();
    value
        .try_reserve_exact(source.len())
        .map_err(|_| resource("native file-search query snapshot allocation failed"))?;
    if value.capacity() != source.len() {
        return Err(resource(
            "native file-search query allocator differs from its exact charge",
        ));
    }
    value.push_str(source);
    Ok(value)
}

pub(super) fn exact_path(source: &Path) -> Result<PathBuf, SearchError> {
    let source = source.as_os_str();
    let bytes = source.as_encoded_bytes().len();
    let mut value = OsString::new();
    value
        .try_reserve_exact(bytes)
        .map_err(|_| resource("native file-search path snapshot allocation failed"))?;
    if value.capacity() != bytes {
        return Err(resource(
            "native file-search path allocator differs from its exact charge",
        ));
    }
    // Copy a single complete OS string into an empty buffer; no encoding changes
    // or path normalization occur, including native non-UTF8/WTF8 strings.
    value.push(source);
    if value.len() != bytes || value.capacity() != bytes {
        return Err(resource(
            "native file-search path differs from its exact charge",
        ));
    }
    Ok(PathBuf::from(value))
}

fn resource(message: &'static str) -> SearchError {
    SearchError::new(SearchErrorKind::ResourceExhausted, message)
}

#[cfg(test)]
#[path = "native_output_tests.rs"]
mod tests;
