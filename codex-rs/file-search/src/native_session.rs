//! Native admission and joined receipts, with legacy anyhow wrappers retained.

use crate::FileSearchOptions;
use crate::SessionInner;
use crate::SessionReporter;
use crate::WorkSender;
use crate::WorkSignal;
use crate::build_override_matcher;
use crate::lifecycle;
use crate::native_budget::NativeBudget;
use crate::native_index::NativeAllocation;
use crate::native_output::NativeOutputLimits;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use crossbeam_channel::Receiver;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;

pub(crate) enum NativePolicy {
    Legacy,
    Bounded(SearchBudget),
    Backend {
        budget: SearchBudget,
        output: NativeOutputLimits,
    },
}

/// Owns every native worker. Drop requests release; consuming close joins them.
pub struct FileSearchSession {
    pub(super) inner: Arc<SessionInner>,
    pub(super) supervisor: Option<thread::JoinHandle<anyhow::Result<()>>>,
    pub(super) finished: Receiver<()>,
}

impl FileSearchSession {
    pub fn update_query(&self, pattern_text: &str) {
        let _ = self.submit_query(pattern_text, /*query_id*/ None);
    }

    pub fn update_query_tagged(&self, pattern_text: &str, query_id: u64) -> anyhow::Result<()> {
        self.submit_query(pattern_text, Some(query_id))
    }

    fn submit_query(&self, pattern_text: &str, query_id: Option<u64>) -> anyhow::Result<()> {
        let mut previous = self
            .inner
            .last_query_id
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.inner.shutdown.load(Ordering::Acquire) {
            anyhow::bail!("file-search session is closed");
        }
        if let Some(output) = self.inner.output {
            output.validate_query(pattern_text)?;
        }
        let query_id = match query_id {
            Some(id) => id,
            None => previous
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("file-search query identities exhausted"))?,
        };
        if query_id == 0 || query_id <= *previous {
            anyhow::bail!("file-search query identity must be positive and strictly increasing");
        }
        self.inner
            .work_tx
            .send(WorkSignal::QueryUpdated {
                query: pattern_text.to_owned(),
                query_id,
            })
            .map_err(|_| anyhow::anyhow!("file-search worker has stopped"))?;
        *previous = query_id;
        Ok(())
    }

    /// Nonblocking private closure; a shared external flag is never changed.
    pub fn request_close(&self) {
        let _admission = self
            .inner
            .last_query_id
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !self.inner.shutdown.swap(true, Ordering::AcqRel) {
            let _ = self.inner.work_tx.send(WorkSignal::Shutdown);
        }
    }

    /// Legacy wrapper over the explicit native operation and cleanup receipts.
    /// Run on a blocking worker. Callbacks and filesystem operations must return.
    pub fn close(self) -> anyhow::Result<()> {
        let outcome = self.close_outcome();
        match outcome.cleanup {
            CloseCleanup::Joined => outcome.operation.map_err(anyhow::Error::new),
            CloseCleanup::Unconfirmed(error) => Err(anyhow::Error::new(error)),
        }
    }

    /// Joins the supervisor and its RAII-owned walker/pool threads. A worker
    /// panic or exhausted index can therefore be operation Err with Joined
    /// cleanup. Calling from this session's own callback cannot prove a join.
    pub fn close_outcome(mut self) -> SearchCloseOutcome {
        self.request_close();
        let Some(supervisor) = self.supervisor.take() else {
            return SearchCloseOutcome {
                operation: self.inner.first_failure().map_or(Ok(()), Err),
                cleanup: CloseCleanup::Joined,
            };
        };
        if supervisor.thread().id() == thread::current().id() {
            let error = SearchError::new(
                SearchErrorKind::SearchFailed,
                "cannot join file search from its own reporter callback",
            );
            return SearchCloseOutcome {
                operation: Err(error.clone()),
                cleanup: CloseCleanup::Unconfirmed(error),
            };
        }
        let result = supervisor
            .join()
            .map_err(|_| anyhow::anyhow!("file-search supervisor panicked"))
            .and_then(std::convert::identity);
        let operation = match self.inner.first_failure() {
            Some(error) => Err(error),
            None => result.map_err(|error| {
                error
                    .downcast_ref::<SearchError>()
                    .cloned()
                    .unwrap_or_else(|| {
                        SearchError::new(SearchErrorKind::SearchFailed, error.to_string())
                    })
            }),
        };
        SearchCloseOutcome {
            operation,
            cleanup: CloseCleanup::Joined,
        }
    }
}

impl Drop for FileSearchSession {
    fn drop(&mut self) {
        self.request_close();
    }
}

impl SessionInner {
    pub(super) fn first_failure(&self) -> Option<SearchError> {
        self.failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub(super) fn fail(&self, error: SearchError) {
        let mut failure = self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if failure.is_none() {
            *failure = Some(error);
        }
        drop(failure);
        self.shutdown.store(true, Ordering::Release);
        let _ = self.work_tx.send(WorkSignal::Shutdown);
    }
}

pub fn create_session(
    roots: Vec<PathBuf>,
    options: FileSearchOptions,
    reporter: Arc<dyn SessionReporter>,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> anyhow::Result<FileSearchSession> {
    create_session_with_receipt(roots, options, reporter, cancel_flag, NativePolicy::Legacy)
        .map_err(anyhow::Error::new)
}

/// Enforces index entries, charged index bytes and dedicated OS-worker budget.
/// Query/result/poll limits and aggregate provider quotas belong to the backend.
/// Private cancellation is used; this API cannot cancel a sibling's shared flag.
pub fn create_bounded_session(
    roots: Vec<PathBuf>,
    options: FileSearchOptions,
    budget: SearchBudget,
    reporter: Arc<dyn SessionReporter>,
) -> Result<FileSearchSession, SearchStartError> {
    create_session_with_receipt(
        roots,
        options,
        reporter,
        /*cancel_flag*/ None,
        NativePolicy::Bounded(budget),
    )
}

pub(crate) fn create_session_with_receipt(
    roots: Vec<PathBuf>,
    options: FileSearchOptions,
    reporter: Arc<dyn SessionReporter>,
    cancel_flag: Option<Arc<AtomicBool>>,
    policy: NativePolicy,
) -> Result<FileSearchSession, SearchStartError> {
    let not_admitted = |operation| SearchStartError {
        operation,
        cleanup: StartCleanup::NotAdmitted,
    };
    let primary = roots.first().ok_or_else(|| {
        not_admitted(SearchError::new(
            SearchErrorKind::InvalidInput,
            "at least one search directory is required",
        ))
    })?;
    let overrides = build_override_matcher(primary, &options.exclude).map_err(|error| {
        let diagnostic = match &policy {
            NativePolicy::Legacy => error.to_string(),
            NativePolicy::Bounded(_) | NativePolicy::Backend { .. } => {
                "invalid file-search exclusion pattern".to_owned()
            }
        };
        not_admitted(SearchError::new(SearchErrorKind::InvalidInput, diagnostic))
    })?;
    let output = match &policy {
        NativePolicy::Backend { output, .. } => Some(*output),
        NativePolicy::Legacy | NativePolicy::Bounded(_) => None,
    };
    let (allocation, budget) = match policy {
        NativePolicy::Legacy => (NativeAllocation::Legacy, None),
        NativePolicy::Bounded(budget) | NativePolicy::Backend { budget, .. } => {
            let (plan, budget) = NativeBudget::prepare(&options, budget).map_err(not_admitted)?;
            (NativeAllocation::Bounded(plan), Some(budget))
        }
    };
    let (work_tx, work_rx) = WorkSender::channel();
    let inner = Arc::new(SessionInner {
        search_directories: roots,
        limit: options.limit.get(),
        threads: options.threads.get(),
        compute_indices: options.compute_indices,
        respect_gitignore: options.respect_gitignore,
        cancelled: cancel_flag.unwrap_or_else(|| Arc::new(AtomicBool::new(false))),
        shutdown: Arc::new(AtomicBool::new(false)),
        last_query_id: Mutex::new(0),
        reporter,
        work_tx,
        budget,
        output,
        failure: Arc::new(Mutex::new(None)),
    });
    lifecycle::start(inner, work_rx, overrides, allocation)
}
