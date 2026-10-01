use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel::after;
use crossbeam_channel::never;
use crossbeam_channel::select;
use crossbeam_channel::unbounded;
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use nucleo::Config;
use nucleo::Injector;
use nucleo::Matcher;
use nucleo::Nucleo;
use nucleo::Utf32String;
use nucleo::pattern::CaseMatching;
use nucleo::pattern::Normalization;
use serde::Serialize;
use std::num::NonZero;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tokio::process::Command;

#[cfg(test)]
use nucleo::Utf32Str;
#[cfg(test)]
use nucleo::pattern::AtomKind;
#[cfg(test)]
use nucleo::pattern::Pattern;

mod cli;
mod async_owner;
mod lifecycle;

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod lifecycle_tests;

pub use cli::Cli;
pub use async_owner::FileSearchOwner;
pub use async_owner::FileSearchStartError;
pub use async_owner::ManagedFileSearchSession;

/// A single match result returned from the search.
///
/// * `score` – Relevance score returned by `nucleo`.
/// * `path`  – Path to the matched entry (file or directory), relative to the
///   search directory.
/// * `match_type` – Whether this match is a file or directory.
/// * `indices` – Optional list of character indices that matched the query.
///   These are only filled when the caller of [`run`] sets
///   `options.compute_indices` to `true`. The indices vector follows the
///   guidance from `nucleo::pattern::Pattern::indices`: they are
///   unique and sorted in ascending order so that callers can use
///   them directly for highlighting.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileMatch {
    pub score: u32,
    pub path: PathBuf,
    pub match_type: MatchType,
    pub root: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indices: Option<Vec<u32>>, // Sorted & deduplicated when present
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    File,
    Directory,
}

/// Carries the entry type observed by the walker so matched paths are not restatted.
struct IndexedEntry {
    full_path: Arc<str>,
    match_type: MatchType,
}

impl FileMatch {
    pub fn full_path(&self) -> PathBuf {
        self.root.join(&self.path)
    }
}

/// Returns the final path component for a matched path, falling back to the full path.
pub fn file_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

#[derive(Debug)]
pub struct FileSearchResults {
    pub matches: Vec<FileMatch>,
    pub total_match_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct FileSearchSnapshot {
    /// Monotonic session-local identity; zero denotes the initial idle state.
    pub query_id: u64,
    pub query: String,
    pub matches: Vec<FileMatch>,
    pub total_match_count: usize,
    pub scanned_file_count: usize,
    pub walk_complete: bool,
}

#[derive(Debug, Clone)]
pub struct FileSearchOptions {
    pub limit: NonZero<usize>,
    pub exclude: Vec<String>,
    pub threads: NonZero<usize>,
    pub compute_indices: bool,
    /// Toggle ignore-file processing in the walker.
    ///
    /// When enabled, `.gitignore` files are scoped by
    /// `WalkBuilder::require_git(true)`, so they are honored only when the
    /// traversed path is inside a git repository. When disabled, the walker
    /// turns off `.gitignore`, git-global/exclude rules, `.ignore`, and
    /// parent-directory ignore scanning.
    pub respect_gitignore: bool,
}

impl Default for FileSearchOptions {
    fn default() -> Self {
        Self {
            #[expect(clippy::unwrap_used)]
            limit: NonZero::new(20).unwrap(),
            exclude: Vec::new(),
            #[expect(clippy::unwrap_used)]
            threads: NonZero::new(2).unwrap(),
            compute_indices: false,
            respect_gitignore: true,
        }
    }
}

pub trait SessionReporter: Send + Sync + 'static {
    /// Called when the debounced top-N changes.
    fn on_update(&self, snapshot: &FileSearchSnapshot);

    /// Called when matching becomes idle or external cancellation is observed.
    /// Private closure fences callbacks and may discard pending queries.
    fn on_complete(&self);

    /// Associates completion with the latest query processed by the matcher.
    /// Existing reporters can ignore identity by implementing only `on_complete`.
    fn on_complete_tagged(&self, _query_id: u64) {
        self.on_complete();
    }
}

/// Owns a search supervisor. Dropping requests cleanup; [`Self::close`] joins it.
pub struct FileSearchSession {
    inner: Arc<SessionInner>,
    supervisor: Option<thread::JoinHandle<anyhow::Result<()>>>,
    finished: Receiver<()>,
}

impl FileSearchSession {
    /// Update with an automatically increasing identity; ignored after closure.
    pub fn update_query(&self, pattern_text: &str) {
        let _ = self.submit_query(pattern_text, /*query_id*/ None);
    }

    /// Admit an explicitly identified query. IDs must be positive and strictly
    /// increase within this session, including automatically assigned IDs.
    pub fn update_query_tagged(&self, pattern_text: &str, query_id: u64) -> anyhow::Result<()> {
        self.submit_query(pattern_text, Some(query_id))
    }

    fn submit_query(&self, pattern_text: &str, query_id: Option<u64>) -> anyhow::Result<()> {
        let mut previous = self.inner.last_query_id.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.inner.shutdown.load(Ordering::Acquire) {
            anyhow::bail!("file-search session is closed");
        }
        let query_id = match query_id {
            Some(id) => id,
            None => previous.checked_add(1).ok_or_else(|| anyhow::anyhow!("file-search query identities exhausted"))?,
        };
        if query_id == 0 || query_id <= *previous {
            anyhow::bail!("file-search query identity must be positive and strictly increasing");
        }
        self.inner.work_tx.send(WorkSignal::QueryUpdated {
            query: pattern_text.to_owned(),
            query_id,
        }).map_err(|_| anyhow::anyhow!("file-search worker has stopped"))?;
        *previous = query_id;
        Ok(())
    }

    /// Stop accepting work and request cleanup without waiting for workers.
    /// This never changes a cancellation flag shared with another session.
    pub fn request_close(&self) {
        let _admission = self.inner.last_query_id.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !self.inner.shutdown.swap(true, Ordering::AcqRel) {
            let _ = self.inner.work_tx.send(WorkSignal::Shutdown);
        }
    }

    /// Request closure and join traversal, matching and every private pool thread.
    ///
    /// Call from a blocking worker in asynchronous applications. Filesystem calls
    /// and reporter callbacks must return before closure can finish. Calling this
    /// from this session's reporter is rejected; use `request_close` there instead.
    pub fn close(mut self) -> anyhow::Result<()> {
        self.request_close();
        let Some(supervisor) = self.supervisor.take() else {
            return Ok(());
        };
        if supervisor.thread().id() == thread::current().id() {
            anyhow::bail!("cannot join file search from its own reporter callback");
        }
        supervisor
            .join()
            .map_err(|_| anyhow::anyhow!("file-search supervisor panicked"))?
    }
}

impl Drop for FileSearchSession {
    fn drop(&mut self) {
        // The supervisor retains all child ownership even when its waiter drops.
        // Only a successful explicit close acknowledges joined cleanup.
        self.request_close();
    }
}

pub fn create_session(
    search_directories: Vec<PathBuf>,
    options: FileSearchOptions,
    reporter: Arc<dyn SessionReporter>,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> anyhow::Result<FileSearchSession> {
    let FileSearchOptions {
        limit,
        exclude,
        threads,
        compute_indices,
        respect_gitignore,
    } = options;
    let Some(primary_search_directory) = search_directories.first() else {
        anyhow::bail!("at least one search directory is required");
    };
    let override_matcher = build_override_matcher(primary_search_directory, &exclude)?;
    let (work_tx, work_rx) = unbounded();
    let inner = Arc::new(SessionInner {
        search_directories,
        limit: limit.get(),
        threads: threads.get(),
        compute_indices,
        respect_gitignore,
        cancelled: cancel_flag.unwrap_or_else(|| Arc::new(AtomicBool::new(false))),
        shutdown: Arc::new(AtomicBool::new(false)),
        last_query_id: Mutex::new(0),
        reporter,
        work_tx,
    });
    lifecycle::start(inner, work_rx, override_matcher)
}

pub trait Reporter {
    fn report_match(&self, file_match: &FileMatch);
    fn warn_matches_truncated(&self, total_match_count: usize, shown_match_count: usize);
    fn warn_no_search_pattern(&self, search_directory: &Path);
}

pub async fn run_main<T: Reporter>(
    Cli {
        pattern,
        limit,
        cwd,
        compute_indices,
        json: _,
        exclude,
        threads,
    }: Cli,
    reporter: T,
) -> anyhow::Result<()> {
    let search_directory = match cwd {
        Some(dir) => dir,
        None => std::env::current_dir()?,
    };
    let pattern_text = match pattern {
        Some(pattern) => pattern,
        None => {
            reporter.warn_no_search_pattern(&search_directory);
            #[cfg(unix)]
            Command::new("ls")
                .arg("-al")
                .current_dir(search_directory)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .await?;
            #[cfg(windows)]
            {
                Command::new("cmd")
                    .arg("/c")
                    .arg(search_directory)
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .status()
                    .await?;
            }
            return Ok(());
        }
    };

    let FileSearchResults {
        total_match_count,
        matches,
    } = run(
        &pattern_text,
        vec![search_directory.to_path_buf()],
        FileSearchOptions {
            limit,
            exclude,
            threads,
            compute_indices,
            respect_gitignore: true,
        },
        /*cancel_flag*/ None,
    )?;
    let match_count = matches.len();
    let matches_truncated = total_match_count > match_count;

    for file_match in matches {
        reporter.report_match(&file_match);
    }
    if matches_truncated {
        reporter.warn_matches_truncated(total_match_count, match_count);
    }

    Ok(())
}

/// The worker threads will periodically check `cancel_flag` to see if they
/// should stop processing files.
pub fn run(
    pattern_text: &str,
    roots: Vec<PathBuf>,
    options: FileSearchOptions,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> anyhow::Result<FileSearchResults> {
    let reporter = Arc::new(RunReporter {
        expected_query: pattern_text.to_owned(),
        ..RunReporter::default()
    });
    let session = create_session(roots, options, reporter.clone(), cancel_flag)?;

    session.update_query(pattern_text);

    let snapshot = reporter.wait_for_complete(&session);
    session.close()?;
    Ok(FileSearchResults {
        matches: snapshot.matches,
        total_match_count: snapshot.total_match_count,
    })
}

/// Sort matches in-place by descending score, then ascending path.
#[cfg(test)]
fn sort_matches(matches: &mut [(u32, String)]) {
    matches.sort_by(cmp_by_score_desc_then_path_asc::<(u32, String), _, _>(
        |t| t.0,
        |t| t.1.as_str(),
    ));
}

/// Returns a comparator closure suitable for `slice.sort_by(...)` that orders
/// items by descending score and then ascending path using the provided accessors.
pub fn cmp_by_score_desc_then_path_asc<T, FScore, FPath>(
    score_of: FScore,
    path_of: FPath,
) -> impl FnMut(&T, &T) -> std::cmp::Ordering
where
    FScore: Fn(&T) -> u32,
    FPath: Fn(&T) -> &str,
{
    use std::cmp::Ordering;
    move |a, b| match score_of(b).cmp(&score_of(a)) {
        Ordering::Equal => path_of(a).cmp(path_of(b)),
        other => other,
    }
}

#[cfg(test)]
fn create_pattern(pattern: &str) -> Pattern {
    Pattern::new(
        pattern,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    )
}

struct SessionInner {
    search_directories: Vec<PathBuf>,
    limit: usize,
    threads: usize,
    compute_indices: bool,
    respect_gitignore: bool,
    cancelled: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    last_query_id: Mutex<u64>,
    reporter: Arc<dyn SessionReporter>,
    work_tx: Sender<WorkSignal>,
}

enum WorkSignal {
    QueryUpdated { query: String, query_id: u64 },
    NucleoNotify,
    WalkComplete,
    Shutdown,
}

fn build_override_matcher(
    search_directory: &Path,
    exclude: &[String],
) -> anyhow::Result<Option<ignore::overrides::Override>> {
    if exclude.is_empty() {
        return Ok(None);
    }
    let mut override_builder = OverrideBuilder::new(search_directory);
    for exclude in exclude {
        let exclude_pattern = format!("!{exclude}");
        override_builder.add(&exclude_pattern)?;
    }
    let matcher = override_builder.build()?;
    Ok(Some(matcher))
}

fn get_file_path<'a>(path: &'a Path, search_directories: &[PathBuf]) -> Option<(usize, &'a str)> {
    let mut best_match: Option<(usize, &Path)> = None;
    for (idx, root) in search_directories.iter().enumerate() {
        if let Ok(rel_path) = path.strip_prefix(root) {
            let root_depth = root.components().count();
            match best_match {
                Some((best_idx, _))
                    if search_directories[best_idx].components().count() >= root_depth => {}
                _ => {
                    best_match = Some((idx, rel_path));
                }
            }
        }
    }

    let (root_idx, rel_path) = best_match?;
    rel_path.to_str().map(|p| (root_idx, p))
}

/// Walks the search directories and feeds discovered paths into `nucleo`
/// via the injector.
///
/// The walker uses `require_git(true)` to match git's own ignore semantics:
/// git never reads `.gitignore` files from directories above the repository
/// root. Without this flag, the `ignore` crate reads `.gitignore` files from
/// *all* ancestor directories—a deliberate divergence from git intended for
/// non-git use cases—allowing a broad parent ignore (e.g. `~/.gitignore`
/// containing `*`) to silently suppress every file in the walk.
///
/// When `respect_gitignore` is `false`, all git-related ignore processing is
/// disabled regardless of this flag.
fn walker_worker(
    inner: Arc<SessionInner>,
    override_matcher: Option<ignore::overrides::Override>,
    injector: Injector<IndexedEntry>,
) {
    let Some(first_root) = inner.search_directories.first() else {
        let _ = inner.work_tx.send(WorkSignal::WalkComplete);
        return;
    };

    if inner.cancelled.load(Ordering::Acquire) || inner.shutdown.load(Ordering::Acquire) {
        return;
    }
    let mut walk_builder = WalkBuilder::new(first_root);
    for root in inner.search_directories.iter().skip(1) {
        walk_builder.add(root);
    }
    walk_builder
        .threads(inner.threads)
        // Allow hidden entries.
        .hidden(false)
        // Follow symlinks to search their contents.
        .follow_links(true)
        // Keep ignore behavior aligned with git repositories: only apply
        // gitignore rules when a git context exists.
        .require_git(true);
    if !inner.respect_gitignore {
        walk_builder
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .ignore(false)
            .parents(false);
    }
    if let Some(override_matcher) = override_matcher {
        walk_builder.overrides(override_matcher);
    }

    let walker = walk_builder.build_parallel();

    walker.run(|| {
        let search_directories = inner.search_directories.clone();
        let injector = injector.clone();
        let cancelled = inner.cancelled.clone();
        let shutdown = inner.shutdown.clone();

        Box::new(move |entry| {
            if cancelled.load(Ordering::Acquire) || shutdown.load(Ordering::Acquire) {
                return ignore::WalkState::Quit;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => return ignore::WalkState::Continue,
            };
            let path = entry.path();
            let Some(full_path) = path.to_str() else {
                return ignore::WalkState::Continue;
            };
            if let Some((_, relative_path)) = get_file_path(path, &search_directories) {
                let match_type = match entry.file_type() {
                    Some(file_type) if file_type.is_dir() => MatchType::Directory,
                    _ => MatchType::File,
                };
                injector.push(
                    IndexedEntry {
                        full_path: Arc::from(full_path),
                        match_type,
                    },
                    |_, cols| {
                        cols[0] = Utf32String::from(relative_path);
                    },
                );
            }
            ignore::WalkState::Continue
        })
    });
    let _ = inner.work_tx.send(WorkSignal::WalkComplete);
}

fn matcher_worker(
    inner: Arc<SessionInner>,
    work_rx: Receiver<WorkSignal>,
    nucleo: &mut Nucleo<IndexedEntry>,
) -> anyhow::Result<()> {
    const TICK_TIMEOUT_MS: u64 = 10;
    let config = Config::DEFAULT.match_paths();
    let mut indices_matcher = inner.compute_indices.then(|| Matcher::new(config.clone()));
    let cancel_requested = || inner.cancelled.load(Ordering::Relaxed);
    let shutdown_requested = || inner.shutdown.load(Ordering::Relaxed);

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
            recv(work_rx) -> signal => {
                let Ok(signal) = signal else {
                    break;
                };
                match signal {
                    WorkSignal::QueryUpdated { query, query_id } => {
                        if shutdown_requested() {
                            break;
                        }
                        let append = query.starts_with(&last_query);
                        nucleo.pattern.reparse(
                            0,
                            &query,
                            CaseMatching::Ignore,
                            Normalization::Smart,
                            append,
                        );
                        last_query = query;
                        last_query_id = query_id;
                        will_notify = true;
                        next_notify = after(Duration::from_millis(0));
                    }
                    WorkSignal::NucleoNotify => {
                        if !will_notify {
                            will_notify = true;
                            next_notify = after(Duration::from_millis(TICK_TIMEOUT_MS));
                        }
                    }
                    WorkSignal::WalkComplete => {
                        walk_complete = true;
                        if !will_notify {
                            will_notify = true;
                            next_notify = after(Duration::from_millis(0));
                        }
                    }
                    WorkSignal::Shutdown => {
                        break;
                    }
                }
            }
            recv(next_notify) -> _ => {
                will_notify = false;
                let status = nucleo.tick(TICK_TIMEOUT_MS);
                // During reparse, Nucleo can expose the previous snapshot while
                // the replacement computation is still running. Never label a
                // different pattern's matches with the latest query identity.
                if status.changed
                    && nucleo.snapshot().pattern().column_pattern(0).atoms
                        == nucleo.pattern.column_pattern(0).atoms
                {
                    let snapshot = nucleo.snapshot();
                    let limit = inner.limit.min(snapshot.matched_item_count() as usize);
                    let pattern = snapshot.pattern().column_pattern(0);
                    let matches: Vec<_> = snapshot
                        .matches()
                        .iter()
                        .take(limit)
                        .filter_map(|match_| {
                            let item = snapshot.get_item(match_.idx)?;
                            let full_path = item.data.full_path.as_ref();
                            let (root_idx, relative_path) = get_file_path(Path::new(full_path), &inner.search_directories)?;
                            let indices = if let Some(indices_matcher) = indices_matcher.as_mut() {
                                let mut idx_vec = Vec::<u32>::new();
                                let haystack = item.matcher_columns[0].slice(..);
                                let _ = pattern.indices(haystack, indices_matcher, &mut idx_vec);
                                idx_vec.sort_unstable();
                                idx_vec.dedup();
                                Some(idx_vec)
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

                    let snapshot = FileSearchSnapshot {
                        query_id: last_query_id,
                        query: last_query.clone(),
                        matches,
                        total_match_count: snapshot.matched_item_count() as usize,
                        scanned_file_count: snapshot.item_count() as usize,
                        walk_complete,
                    };
                    if !shutdown_requested() {
                        inner.reporter.on_update(&snapshot);
                    }
                }
                if !status.running && walk_complete && !shutdown_requested() {
                    inner.reporter.on_complete_tagged(last_query_id);
                }
            }
            default(Duration::from_millis(100)) => {
                // Occasionally check the cancel flag.
            }
        }

        if cancel_requested() || shutdown_requested() {
            break;
        }
    }

    // External cancellation retains its completion notification. Private close
    // fences callbacks; explicit close waits for any callback already in flight.
    if !shutdown_requested() {
        inner.reporter.on_complete_tagged(last_query_id);
    }

    Ok(())
}

struct RunReporter {
    expected_query: String,
    snapshot: RwLock<FileSearchSnapshot>,
    completed_tx: Sender<()>,
    completed_rx: Receiver<()>,
}

impl Default for RunReporter {
    fn default() -> Self {
        let (completed_tx, completed_rx) = crossbeam_channel::bounded(1);
        Self {
            expected_query: String::new(),
            snapshot: RwLock::default(),
            completed_tx,
            completed_rx,
        }
    }
}

impl SessionReporter for RunReporter {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        let mut guard = self.snapshot.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = snapshot.clone();
    }

    fn on_complete_tagged(&self, query_id: u64) {
        if query_id != 0 {
            self.on_complete();
        }
    }

    fn on_complete(&self) {
        // Startup can become idle before the caller submits its first query.
        // That completion must not finish a different one-shot query.
        let snapshot = self.snapshot.read().unwrap_or_else(std::sync::PoisonError::into_inner);
        if snapshot.query == self.expected_query {
            let _ = self.completed_tx.try_send(());
        }
    }
}

impl RunReporter {
    fn wait_for_complete(&self, session: &FileSearchSession) -> FileSearchSnapshot {
        select! {
            recv(self.completed_rx) -> _ => {},
            recv(session.finished) -> _ => {},
        }
        self.snapshot.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::sync::Arc;
    use std::sync::Condvar;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use std::time::Duration;
    use std::time::Instant;
    use tempfile::TempDir;

    #[test]
    fn verify_score_is_none_for_non_match() {
        let mut utf32buf = Vec::<char>::new();
        let line = "hello";
        let mut matcher = Matcher::new(Config::DEFAULT);
        let haystack: Utf32Str<'_> = Utf32Str::new(line, &mut utf32buf);
        let pattern = create_pattern("zzz");
        let score = pattern.score(haystack, &mut matcher);
        assert_eq!(score, None);
    }

    #[test]
    fn tie_breakers_sort_by_path_when_scores_equal() {
        let mut matches = vec![
            (100, "b_path".to_string()),
            (100, "a_path".to_string()),
            (90, "zzz".to_string()),
        ];

        sort_matches(&mut matches);

        // Highest score first; ties broken alphabetically.
        let expected = vec![
            (100, "a_path".to_string()),
            (100, "b_path".to_string()),
            (90, "zzz".to_string()),
        ];

        assert_eq!(matches, expected);
    }

    #[test]
    fn file_name_from_path_uses_basename() {
        assert_eq!(file_name_from_path("foo/bar.txt"), "bar.txt");
    }

    #[test]
    fn file_name_from_path_falls_back_to_full_path() {
        assert_eq!(file_name_from_path(""), "");
    }

    #[derive(Default)]
    struct RecordingReporter {
        updates: Mutex<Vec<FileSearchSnapshot>>,
        complete_times: Mutex<Vec<Instant>>,
        complete_cv: Condvar,
        update_cv: Condvar,
    }

    impl RecordingReporter {
        fn wait_until<T, F>(
            &self,
            mutex: &Mutex<T>,
            cv: &Condvar,
            timeout: Duration,
            mut predicate: F,
        ) -> bool
        where
            F: FnMut(&T) -> bool,
        {
            let deadline = Instant::now() + timeout;
            let mut state = mutex.lock().unwrap();
            loop {
                if predicate(&state) {
                    return true;
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return false;
                }
                let (next_state, wait_result) = cv.wait_timeout(state, remaining).unwrap();
                state = next_state;
                if wait_result.timed_out() {
                    return predicate(&state);
                }
            }
        }

        fn wait_for_complete(&self, timeout: Duration) -> bool {
            self.wait_until(
                &self.complete_times,
                &self.complete_cv,
                timeout,
                |completes| !completes.is_empty(),
            )
        }
        fn clear(&self) {
            self.updates.lock().unwrap().clear();
            self.complete_times.lock().unwrap().clear();
        }

        fn updates(&self) -> Vec<FileSearchSnapshot> {
            self.updates.lock().unwrap().clone()
        }

        fn wait_for_updates_at_least(&self, min_len: usize, timeout: Duration) -> bool {
            self.wait_until(&self.updates, &self.update_cv, timeout, |updates| {
                updates.len() >= min_len
            })
        }

        fn snapshot(&self) -> FileSearchSnapshot {
            self.updates
                .lock()
                .unwrap()
                .last()
                .cloned()
                .unwrap_or_default()
        }
    }

    impl SessionReporter for RecordingReporter {
        fn on_update(&self, snapshot: &FileSearchSnapshot) {
            let mut updates = self.updates.lock().unwrap();
            updates.push(snapshot.clone());
            self.update_cv.notify_all();
        }

        fn on_complete(&self) {
            {
                let mut complete_times = self.complete_times.lock().unwrap();
                complete_times.push(Instant::now());
            }
            self.complete_cv.notify_all();
        }
    }

    fn create_temp_tree(file_count: usize) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..file_count {
            let path = dir.path().join(format!("file-{i:04}.txt"));
            fs::write(path, format!("contents {i}")).unwrap();
        }
        dir
    }

    #[test]
    fn session_scanned_file_count_is_monotonic_across_queries() {
        let dir = create_temp_tree(/*file_count*/ 200);
        let reporter = Arc::new(RecordingReporter::default());
        let session = create_session(
            vec![dir.path().to_path_buf()],
            FileSearchOptions::default(),
            reporter.clone(),
            /*cancel_flag*/ None,
        )
        .expect("session");

        session.update_query("file-00");
        thread::sleep(Duration::from_millis(20));
        let first_snapshot = reporter.snapshot();
        session.update_query("file-01");
        thread::sleep(Duration::from_millis(20));
        let second_snapshot = reporter.snapshot();
        let _ = reporter.wait_for_complete(Duration::from_secs(5));
        let completed_snapshot = reporter.snapshot();

        assert!(second_snapshot.scanned_file_count >= first_snapshot.scanned_file_count);
        assert!(completed_snapshot.scanned_file_count >= second_snapshot.scanned_file_count);
    }

    #[test]
    fn session_streams_updates_before_walk_complete() {
        let dir = create_temp_tree(/*file_count*/ 600);
        let reporter = Arc::new(RecordingReporter::default());
        let session = create_session(
            vec![dir.path().to_path_buf()],
            FileSearchOptions::default(),
            reporter.clone(),
            /*cancel_flag*/ None,
        )
        .expect("session");

        session.update_query("file-0");
        let completed = reporter.wait_for_complete(Duration::from_secs(5));

        assert!(completed);
        let updates = reporter.updates();
        assert!(updates.iter().any(|snapshot| !snapshot.walk_complete));
    }

    #[test]
    fn session_accepts_query_updates_after_walk_complete() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("alpha.txt"), "alpha").unwrap();
        fs::write(dir.path().join("beta.txt"), "beta").unwrap();
        let reporter = Arc::new(RecordingReporter::default());
        let session = create_session(
            vec![dir.path().to_path_buf()],
            FileSearchOptions::default(),
            reporter.clone(),
            /*cancel_flag*/ None,
        )
        .expect("session");

        session.update_query("alpha");
        assert!(reporter.wait_for_complete(Duration::from_secs(5)));
        let updates_before = reporter.updates().len();

        session.update_query("beta");
        assert!(reporter.wait_for_updates_at_least(updates_before + 1, Duration::from_secs(5),));

        let updates = reporter.updates();
        let last_update = updates.last().cloned().expect("update");
        assert!(
            last_update
                .matches
                .iter()
                .any(|file_match| file_match.path.to_string_lossy().contains("beta.txt"))
        );
    }

    #[test]
    fn session_emits_complete_when_query_changes_with_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("alpha.txt"), "alpha").unwrap();
        fs::write(dir.path().join("beta.txt"), "beta").unwrap();
        let reporter = Arc::new(RecordingReporter::default());
        let session = create_session(
            vec![dir.path().to_path_buf()],
            FileSearchOptions::default(),
            reporter.clone(),
            /*cancel_flag*/ None,
        )
        .expect("session");

        session.update_query("asdf");
        assert!(reporter.wait_for_complete(Duration::from_secs(5)));

        let completed_snapshot = reporter.snapshot();
        assert_eq!(completed_snapshot.matches, Vec::new());
        assert_eq!(completed_snapshot.total_match_count, 0);

        reporter.clear();

        session.update_query("asdfa");
        assert!(reporter.wait_for_complete(Duration::from_secs(5)));
        assert!(!reporter.updates().is_empty());
    }

    #[test]
    fn dropping_session_does_not_cancel_siblings_with_shared_cancel_flag() {
        let root_a = create_temp_tree(/*file_count*/ 200);
        let root_b = create_temp_tree(/*file_count*/ 4_000);
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let reporter_a = Arc::new(RecordingReporter::default());
        let session_a = create_session(
            vec![root_a.path().to_path_buf()],
            FileSearchOptions::default(),
            reporter_a,
            Some(cancel_flag.clone()),
        )
        .expect("session_a");

        let reporter_b = Arc::new(RecordingReporter::default());
        let session_b = create_session(
            vec![root_b.path().to_path_buf()],
            FileSearchOptions::default(),
            reporter_b.clone(),
            Some(cancel_flag),
        )
        .expect("session_b");

        session_a.update_query("file-0");
        session_b.update_query("file-1");

        thread::sleep(Duration::from_millis(5));
        drop(session_a);

        let completed = reporter_b.wait_for_complete(Duration::from_secs(5));
        assert_eq!(completed, true);
    }

    #[test]
    fn session_emits_updates_when_query_changes() {
        let dir = create_temp_tree(/*file_count*/ 200);
        let reporter = Arc::new(RecordingReporter::default());
        let session = create_session(
            vec![dir.path().to_path_buf()],
            FileSearchOptions::default(),
            reporter.clone(),
            /*cancel_flag*/ None,
        )
        .expect("session");

        session.update_query("zzzzzzzz");
        let completed = reporter.wait_for_complete(Duration::from_secs(5));
        assert!(completed);

        reporter.clear();

        session.update_query("zzzzzzzzq");
        let completed = reporter.wait_for_complete(Duration::from_secs(5));
        assert!(completed);

        let updates = reporter.updates();
        assert_eq!(updates.len(), 1);
    }

    #[test]
    fn run_returns_matches_for_query() {
        let dir = create_temp_tree(/*file_count*/ 40);
        let options = FileSearchOptions {
            limit: NonZero::new(20).unwrap(),
            exclude: Vec::new(),
            threads: NonZero::new(2).unwrap(),
            compute_indices: false,
            respect_gitignore: true,
        };
        let results = run(
            "file-000",
            vec![dir.path().to_path_buf()],
            options,
            /*cancel_flag*/ None,
        )
        .expect("run ok");

        assert!(!results.matches.is_empty());
        assert!(results.total_match_count >= results.matches.len());
        assert!(
            results
                .matches
                .iter()
                .any(|m| m.path.to_string_lossy().contains("file-0000.txt"))
        );
    }

    #[test]
    fn run_returns_directory_matches_for_query() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/guides")).unwrap();
        fs::write(dir.path().join("docs/guides/intro.md"), "intro").unwrap();
        fs::write(dir.path().join("docs/readme.md"), "readme").unwrap();

        let results = run(
            "guides",
            vec![dir.path().to_path_buf()],
            FileSearchOptions {
                limit: NonZero::new(20).unwrap(),
                exclude: Vec::new(),
                threads: NonZero::new(2).unwrap(),
                compute_indices: false,
                respect_gitignore: true,
            },
            /*cancel_flag*/ None,
        )
        .expect("run ok");

        assert!(results.matches.iter().any(|m| {
            m.path == std::path::Path::new("docs").join("guides")
                && m.match_type == MatchType::Directory
        }));
    }

    #[cfg(unix)]
    #[test]
    fn run_classifies_followed_directory_symlink_as_directory() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("guides")).unwrap();
        symlink(dir.path().join("guides"), dir.path().join("guides-link")).unwrap();

        let results = run(
            "guides-link",
            vec![dir.path().to_path_buf()],
            FileSearchOptions::default(),
            /*cancel_flag*/ None,
        )
        .expect("run ok");

        assert!(results.matches.iter().any(|file_match| {
            file_match.path == Path::new("guides-link")
                && file_match.match_type == MatchType::Directory
        }));
    }

    #[test]
    fn cancel_exits_run() {
        let dir = create_temp_tree(/*file_count*/ 200);
        let cancel_flag = Arc::new(AtomicBool::new(true));
        let search_dir = dir.path().to_path_buf();
        let options = FileSearchOptions {
            compute_indices: false,
            ..Default::default()
        };
        let (tx, rx) = std::sync::mpsc::channel();

        let handle = thread::spawn(move || {
            let result = run("file-", vec![search_dir], options, Some(cancel_flag));
            let _ = tx.send(result);
        });

        let result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("run should exit after cancellation");
        handle.join().unwrap();

        let results = result.expect("run ok");
        assert_eq!(results.matches, Vec::new());
        assert_eq!(results.total_match_count, 0);
    }

    /// Regression test for #3493: a parent directory's `.gitignore` with `*`
    /// must not suppress files discovered inside a child "repo" directory.
    ///
    /// The fixture intentionally omits `git init` so that no `.git` directory
    /// exists. With `require_git(true)`, the walker skips all gitignore
    /// processing, making the parent's broad ignore harmless.
    #[test]
    fn parent_gitignore_outside_repo_does_not_hide_repo_files() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("home");
        let repo = parent.join("repo");
        fs::create_dir_all(repo.join(".vscode")).unwrap();

        fs::write(parent.join(".gitignore"), "*\n!.gitignore\n").unwrap();
        fs::write(
            repo.join(".gitignore"),
            ".vscode/*\n!.vscode/\n!.vscode/settings.json\n!package.json\n",
        )
        .unwrap();
        fs::write(repo.join("package.json"), "{ \"name\": \"demo\" }\n").unwrap();
        fs::write(repo.join(".vscode/settings.json"), "{ \"editor\": true }\n").unwrap();

        let respect_results = run(
            "package",
            vec![repo.clone()],
            FileSearchOptions {
                limit: NonZero::new(20).unwrap(),
                exclude: Vec::new(),
                threads: NonZero::new(2).unwrap(),
                compute_indices: false,
                respect_gitignore: true,
            },
            /*cancel_flag*/ None,
        )
        .expect("run ok");
        assert!(
            respect_results
                .matches
                .iter()
                .any(|m| m.path.as_path() == Path::new("package.json"))
        );

        let nested_file_results = run(
            "settings",
            vec![repo],
            FileSearchOptions {
                limit: NonZero::new(20).unwrap(),
                exclude: Vec::new(),
                threads: NonZero::new(2).unwrap(),
                compute_indices: false,
                respect_gitignore: true,
            },
            /*cancel_flag*/ None,
        )
        .expect("run ok");
        assert!(
            nested_file_results
                .matches
                .iter()
                .any(|m| m.path.as_path() == Path::new(".vscode/settings.json"))
        );
    }

    #[test]
    fn git_repo_still_respects_local_gitignore_when_enabled() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("home");
        let repo = parent.join("repo");
        fs::create_dir_all(repo.join(".vscode")).unwrap();

        fs::write(parent.join(".gitignore"), "*\n!.gitignore\n").unwrap();
        fs::write(
            repo.join(".gitignore"),
            ".vscode/*\n!.vscode/\n!.vscode/settings.json\n!package.json\n",
        )
        .unwrap();
        fs::write(repo.join("package.json"), "{ \"name\": \"demo\" }\n").unwrap();
        fs::write(repo.join(".vscode/settings.json"), "{ \"editor\": true }\n").unwrap();
        fs::write(
            repo.join(".vscode/extensions.json"),
            "{ \"extensions\": [] }\n",
        )
        .unwrap();

        fs::create_dir_all(repo.join(".git")).unwrap();

        let package_results = run(
            "package",
            vec![repo.clone()],
            FileSearchOptions {
                limit: NonZero::new(20).unwrap(),
                exclude: Vec::new(),
                threads: NonZero::new(2).unwrap(),
                compute_indices: false,
                respect_gitignore: true,
            },
            /*cancel_flag*/ None,
        )
        .expect("run ok");
        assert!(
            package_results
                .matches
                .iter()
                .any(|m| m.path.as_path() == Path::new("package.json"))
        );

        let ignored_results = run(
            "extensions.json",
            vec![repo.clone()],
            FileSearchOptions {
                limit: NonZero::new(20).unwrap(),
                exclude: Vec::new(),
                threads: NonZero::new(2).unwrap(),
                compute_indices: false,
                respect_gitignore: true,
            },
            /*cancel_flag*/ None,
        )
        .expect("run ok");
        assert!(
            !ignored_results
                .matches
                .iter()
                .any(|m| m.path.as_path() == Path::new(".vscode/extensions.json"))
        );

        let whitelisted_results = run(
            "settings.json",
            vec![repo],
            FileSearchOptions {
                limit: NonZero::new(20).unwrap(),
                exclude: Vec::new(),
                threads: NonZero::new(2).unwrap(),
                compute_indices: false,
                respect_gitignore: true,
            },
            /*cancel_flag*/ None,
        )
        .expect("run ok");
        assert!(
            whitelisted_results
                .matches
                .iter()
                .any(|m| m.path.as_path() == Path::new(".vscode/settings.json"))
        );
    }
}
