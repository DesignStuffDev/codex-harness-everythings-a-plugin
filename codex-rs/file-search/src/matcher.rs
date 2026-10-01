use crate::FileMatch;
use crate::FileSearchSnapshot;
use crate::IndexedEntry;
use crate::SessionInner;
use crate::get_file_path;
use crate::native_index::NativeMatcher;
use crossbeam_channel::Receiver;
use crossbeam_channel::after;
use crossbeam_channel::never;
use crossbeam_channel::select;
use nucleo::Config;
use nucleo::Matcher;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub(super) fn matcher_worker(
    inner: Arc<SessionInner>,
    work_rx: Receiver<()>,
    nucleo: &mut dyn NativeMatcher,
) -> anyhow::Result<()> {
    const TICK_TIMEOUT_MS: u64 = 10;
    let config = Config::DEFAULT.match_paths();
    let mut indices_matcher = if inner.compute_indices {
        if inner.budget.is_some() {
            Some(Matcher::try_new(config).ok_or_else(|| {
                codex_file_search_api::SearchError::new(
                    codex_file_search_api::SearchErrorKind::ResourceExhausted,
                    "native highlighting scratch allocation failed",
                )
            })?)
        } else {
            Some(Matcher::new(config))
        }
    } else {
        None
    };
    let cancel_requested = || inner.cancelled.load(Ordering::Acquire);
    let shutdown_requested = || inner.shutdown.load(Ordering::Acquire);

    let mut last_query = String::new();
    let mut last_query_id = 0;
    let mut next_notify = never();
    let mut will_notify = false;
    let mut walk_complete = false;

    loop {
        if cancel_requested() || shutdown_requested() {
            break;
        }
        select! {
            recv(work_rx) -> wake => {
                if wake.is_err() {
                    break;
                }
                let pending = inner.work_tx.take();
                if pending.shutdown || shutdown_requested() {
                    break;
                }
                if let Some((query, query_id)) = pending.query {
                    let append = query.starts_with(&last_query);
                    nucleo.reparse(&query, append);
                    last_query = query;
                    last_query_id = query_id;
                    will_notify = true;
                    next_notify = after(Duration::ZERO);
                }
                if pending.walk_complete {
                    walk_complete = true;
                    will_notify = true;
                    next_notify = after(Duration::ZERO);
                } else if pending.notified && !will_notify {
                    will_notify = true;
                    next_notify = after(Duration::from_millis(TICK_TIMEOUT_MS));
                }
            }
            recv(next_notify) -> _ => {
                will_notify = false;
                let status = nucleo.tick(TICK_TIMEOUT_MS);
                let complete = !status.running && walk_complete;
                // During reparse, Nucleo can expose the previous snapshot while
                // replacement computation runs. Never relabel its old matches.
                let coherent = nucleo.snapshot().pattern().column_pattern(0).atoms
                    == nucleo.pattern().column_pattern(0).atoms;
                if cancel_requested() || shutdown_requested() {
                    break;
                }
                if complete && !coherent {
                    anyhow::bail!("file-search became idle with a stale pattern snapshot");
                }
                // Walking can finish after matching became idle. That final
                // tick need not change Nucleo's snapshot, but must still publish
                // the current identity and walk_complete=true before completion.
                if coherent && (status.changed || complete) {
                    let snapshot = snapshot(
                        &inner,
                        nucleo.snapshot(),
                        &mut indices_matcher,
                        last_query_id,
                        &last_query,
                        walk_complete,
                    );
                    if !cancel_requested() && !shutdown_requested() {
                        inner.reporter.on_update(&snapshot);
                    }
                }
                if complete && !cancel_requested() && !shutdown_requested() {
                    inner.reporter.on_complete_tagged(last_query_id);
                }
            }
            default(Duration::from_millis(100)) => {
                // Occasionally check the legacy external cancellation flag.
            }
        }
    }

    // Legacy external cancellation retains its completion notification. Native
    // backend adapters must use private close instead of supplying this flag.
    if !shutdown_requested() {
        inner.reporter.on_complete_tagged(last_query_id);
    }
    Ok(())
}

fn snapshot(
    inner: &SessionInner,
    snapshot: &nucleo::Snapshot<IndexedEntry>,
    indices_matcher: &mut Option<Matcher>,
    query_id: u64,
    query: &str,
    walk_complete: bool,
) -> FileSearchSnapshot {
    let limit = inner.limit.min(snapshot.matched_item_count() as usize);
    let pattern = snapshot.pattern().column_pattern(0);
    let matches = snapshot
        .matches()
        .iter()
        .take(limit)
        .filter_map(|match_| {
            let item = snapshot.get_item(match_.idx)?;
            let (root_idx, relative_path) = get_file_path(
                Path::new(item.data.full_path.as_ref()),
                &inner.search_directories,
            )?;
            let indices = if let Some(indices_matcher) = indices_matcher.as_mut() {
                let mut indices = Vec::new();
                let haystack = item.matcher_columns[0].slice(..);
                let _ = pattern.indices(haystack, indices_matcher, &mut indices);
                indices.sort_unstable();
                indices.dedup();
                Some(indices)
            } else {
                None
            };
            Some(FileMatch {
                score: match_.score,
                path: PathBuf::from(relative_path),
                match_type: item.data.match_type,
                root: inner.search_directories[root_idx].clone(),
                indices,
            })
        })
        .collect();
    FileSearchSnapshot {
        query_id,
        query: query.to_owned(),
        matches,
        total_match_count: snapshot.matched_item_count() as usize,
        scanned_file_count: snapshot.item_count() as usize,
        walk_complete,
    }
}
