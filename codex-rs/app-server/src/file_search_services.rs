//! One selected search composition shared by connection and local picker scopes.
//! Provider ownership stays outside abortable processors; factories are weak.

use std::io;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::ScopeLimits;
use codex_file_search_api::SearchBudget;
use codex_file_search_runtime::FileSearchProvider;
use codex_file_search_runtime::FileSearchScopeFactory;
use codex_file_search_runtime::SelectionContext;
use codex_file_search_runtime::interactive_allocation;
use codex_file_search_runtime::interactive_policy;
use codex_file_search_runtime::select_provider;

/// Immutable consumer policy, not another backend/public provider owner.
#[derive(Clone)]
pub(crate) struct SearchContext {
    pub(crate) factory: FileSearchScopeFactory,
    /// Deliberate composition shutdown, published before backend closure.
    pub(crate) shutdown_requested: CancellationToken,
    pub(crate) scope_limits: ScopeLimits,
    pub(crate) budget: SearchBudget,
    pub(crate) options: FileSearchOptions,
    pub(crate) max_query_bytes: NonZeroUsize,
}

pub(crate) async fn start(codex_home: &Path) -> io::Result<(SearchContext, SearchShutdownGuard)> {
    let options = crate::fuzzy_file_search::options();
    let budget = interactive_allocation(options.threads).map_err(io::Error::other)?;
    let policy = interactive_policy().map_err(io::Error::other)?;
    let selected = select_provider(
        SelectionContext {
            codex_home: codex_home.to_owned(),
            // Keep original caller roots lexical. Never use a thread/workspace cwd
            // here or change global cwd on behalf of an independent embedding.
            base_dir: std::env::current_dir()?,
        },
        policy,
    )
    .await
    .map_err(io::Error::other)?;
    let effective = selected.effective_policy;
    let lifecycle = SearchShutdownGuard::new(selected.provider);
    let context = SearchContext {
        factory: lifecycle.factory(),
        shutdown_requested: lifecycle.shutdown_signal(),
        scope_limits: ScopeLimits {
            max_sessions: effective.provider.max_sessions,
        },
        budget,
        options,
        max_query_bytes: effective.max_query_bytes,
    };
    Ok((context, lifecycle))
}

/// Dropping any explicit owner fences the provider even when the runtime or
/// detached requests retain other owners. Normal exits separately await proof.
pub(crate) struct SearchShutdownGuard {
    provider: Arc<FileSearchProvider>,
    shutdown_requested: CancellationToken,
}
impl SearchShutdownGuard {
    pub(crate) fn new(provider: FileSearchProvider) -> Self {
        Self {
            provider: Arc::new(provider),
            shutdown_requested: CancellationToken::new(),
        }
    }
    /// A second explicit lifetime owner; avoid incidental Clone/temporary guards.
    pub(crate) fn fork(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider),
            shutdown_requested: self.shutdown_requested.clone(),
        }
    }
    pub(crate) fn factory(&self) -> FileSearchScopeFactory {
        self.provider.scope_factory()
    }
    pub(crate) fn shutdown_signal(&self) -> CancellationToken {
        self.shutdown_requested.clone()
    }
    pub(crate) fn begin_shutdown(&self) {
        // Presentation must observe deliberate shutdown even when its processor
        // is blocked or has not yet run its per-connection cancellation fence.
        self.shutdown_requested.cancel();
        self.provider.request_shutdown();
    }
    pub(crate) async fn finish(&self) -> io::Result<()> {
        self.begin_shutdown();
        let outcome = tokio::time::timeout(Duration::from_secs(120), self.provider.shutdown())
            .await
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "file-search provider shutdown timed out; owned cleanup is unconfirmed",
                )
            })?;
        let operation = outcome
            .operation
            .err()
            .map(|error| format!("file-search provider failed: {error}"));
        let cleanup = match outcome.cleanup {
            CloseCleanup::Joined => None,
            CloseCleanup::Unconfirmed(error) => {
                Some(format!("file-search provider cleanup unconfirmed: {error}"))
            }
        };
        match (operation, cleanup) {
            (None, None) => Ok(()),
            (Some(error), None) | (None, Some(error)) => Err(io::Error::other(error)),
            (Some(operation), Some(cleanup)) => {
                Err(io::Error::other(format!("{operation}; {cleanup}")))
            }
        }
    }
}
impl Drop for SearchShutdownGuard {
    fn drop(&mut self) {
        self.begin_shutdown();
    }
}

/// Preserve every cleanup failure without returning early and skipping a service.
pub(crate) fn combine<T>(result: io::Result<T>, cleanup: io::Result<()>) -> io::Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(primary), Err(cleanup)) => Err(io::Error::new(
            primary.kind(),
            format!("{primary}; {cleanup}"),
        )),
    }
}
