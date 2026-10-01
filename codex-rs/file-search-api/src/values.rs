use crate::SearchError;
use serde::Serialize;
use std::num::NonZero;
use std::path::PathBuf;

/// A single match result returned from the search.
///
/// `path` is relative to `root`; `score` is the backend's relevance score.
/// Optional character indices are sorted and deduplicated for highlighting when
/// `FileSearchOptions::compute_indices` is enabled. The serialization of these
/// values preserves the existing native search API, including omitted indices.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileMatch {
    pub score: u32,
    pub path: PathBuf,
    pub match_type: MatchType,
    pub root: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indices: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    File,
    Directory,
}

impl FileMatch {
    pub fn full_path(&self) -> PathBuf {
        self.root.join(&self.path)
    }
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

/// Receives bounded current-query observations from a client runtime facade.
///
/// Implementations must return promptly and must not synchronously join their
/// own session from a callback. A facade fences obsolete generations before
/// delivery and retains its callback owner until joined cleanup. Native legacy
/// completion can include cancellation; a backend adapter must distinguish that
/// from successful query idle instead of forwarding it as successful completion.
pub trait SessionReporter: Send + Sync + 'static {
    /// Called when the debounced top-N changes.
    fn on_update(&self, snapshot: &FileSearchSnapshot);

    /// Legacy completion callback; see this trait's cancellation distinction.
    fn on_complete(&self);

    /// Associates completion with the latest query processed by the matcher.
    /// Existing reporters can ignore identity by implementing only `on_complete`.
    fn on_complete_tagged(&self, _query_id: u64) {
        self.on_complete();
    }

    /// Reports failure through the current generation's presentation fence.
    /// The runtime also retains this error for close/shutdown observers.
    fn on_error(&self, _error: &SearchError) {}
}
