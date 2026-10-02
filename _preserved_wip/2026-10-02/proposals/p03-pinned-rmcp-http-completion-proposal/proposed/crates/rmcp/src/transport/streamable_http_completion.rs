//! Local task joins and optional remote session deletion have separate outcomes.

use std::error::Error;
use std::sync::{Arc, OnceLock};

use super::{SseTaskResult, StreamableHttpError};

/// Deletion concerns the last known remote session only. A missing session ID
/// does not prove that an interrupted request created no remote state.
#[derive(Debug)]
#[non_exhaustive]
pub enum HttpRemoteDeleteOutcome<E: Error + Send + Sync + 'static> {
    NoKnownSession,
    Deleted,
    Unsupported,
    Failed(StreamableHttpError<E>),
    TimedOut,
}

/// Published only after application requests have been dropped and all owned
/// SSE task handles joined. Pair this with `WorkerTransport::close_observed`:
/// seeing this receipt alone does not establish that the worker itself joined.
/// It makes no universal claim about an HTTP backend's private tasks or DNS.
#[derive(Debug)]
pub struct HttpCleanupReceipt<E: Error + Send + Sync + 'static> {
    pub joined_sse_tasks: u64,
    pub cancelled_sse_tasks: u64,
    pub unexpected_sse_task_failures: u64,
    pub first_sse_task_failure: Option<tokio::task::JoinError>,
    pub remote_delete: HttpRemoteDeleteOutcome<E>,
}

impl<E: Error + Send + Sync + 'static> HttpCleanupReceipt<E> {
    /// All owned application tasks were joined without an unexpected task error.
    /// Optional request and DELETE errors are deliberately not this predicate.
    pub fn local_cleanup_complete(&self) -> bool {
        self.unexpected_sse_task_failures == 0
    }
}

/// Read-only observation; only the worker can publish the immutable receipt.
#[derive(Debug)]
pub struct HttpCleanupObserver<E: Error + Send + Sync + 'static> {
    receipt: Arc<OnceLock<Arc<HttpCleanupReceipt<E>>>>,
}

impl<E: Error + Send + Sync + 'static> Clone for HttpCleanupObserver<E> {
    fn clone(&self) -> Self {
        Self { receipt: Arc::clone(&self.receipt) }
    }
}

impl<E: Error + Send + Sync + 'static> Default for HttpCleanupObserver<E> {
    fn default() -> Self {
        Self { receipt: Arc::new(OnceLock::new()) }
    }
}

impl<E: Error + Send + Sync + 'static> HttpCleanupObserver<E> {
    /// None is pending or unobserved, never a successful cleanup result.
    pub fn snapshot(&self) -> Option<Arc<HttpCleanupReceipt<E>>> {
        self.receipt.get().map(Arc::clone)
    }

    pub(super) fn publish(&self, joins: SseJoins, remote_delete: HttpRemoteDeleteOutcome<E>) {
        let _ = self.receipt.set(Arc::new(HttpCleanupReceipt {
            joined_sse_tasks: joins.joined,
            cancelled_sse_tasks: joins.cancelled,
            unexpected_sse_task_failures: joins.failures,
            first_sse_task_failure: joins.first_failure,
            remote_delete,
        }));
    }
}

/// Keep a bounded failure summary and the first original JoinError, rather
/// than accumulating one error allocation per failed stream over a long session.
#[derive(Default)]
pub(super) struct SseJoins {
    joined: u64,
    cancelled: u64,
    failures: u64,
    first_failure: Option<tokio::task::JoinError>,
}

impl SseJoins {
    pub(super) fn joined(&mut self) {
        self.joined = self.joined.saturating_add(1);
    }

    pub(super) fn unexpected(&mut self, error: tokio::task::JoinError) {
        self.joined();
        self.failures = self.failures.saturating_add(1);
        self.first_failure.get_or_insert(error);
    }

    /// Aborting requests is part of shutdown/recovery. Each original handle is
    /// still joined; only those expected cancellations are non-failing joins.
    pub(super) async fn abort_and_drain<E: Error + Send + Sync + 'static>(
        &mut self,
        streams: &mut tokio::task::JoinSet<SseTaskResult<E>>,
    ) {
        streams.abort_all();
        while let Some(result) = streams.join_next().await {
            match result {
                Ok(_) => self.joined(),
                Err(error) if error.is_cancelled() => {
                    self.joined();
                    self.cancelled = self.cancelled.saturating_add(1);
                }
                Err(error) => self.unexpected(error),
            }
        }
    }
}
