//! Controlled constructor ownership used by the real App Server request paths.
//! This proves consumer wiring, not native/provider-process construction behavior.
use super::*;
use codex_file_search_api::*;
use codex_file_search_runtime::FileSearchProvider;
use codex_file_search_runtime::RuntimePolicy;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tokio_util::task::task_tracker::TaskTrackerToken;

pub(super) struct Start {
    pub entered: Semaphore,
    pub cancelled: CancellationToken,
    pub release: Semaphore,
    pub operation: Result<(), SearchError>,
    admitted: AtomicBool,
    checked_connection: Mutex<Option<std::sync::Weak<SearchConnectionState>>>,
    pub hook_under_fence: AtomicBool,
    receipt: watch::Sender<Option<SearchStartCancellationOutcome>>,
}
impl Start {
    pub fn new(operation: Result<(), SearchError>) -> Arc<Self> {
        Arc::new(Self {
            entered: Semaphore::new(0),
            cancelled: CancellationToken::new(),
            release: Semaphore::new(0),
            operation,
            admitted: AtomicBool::new(false),
            receipt: watch::channel(None).0,
            checked_connection: Mutex::new(None),
            hook_under_fence: AtomicBool::new(false),
        })
    }
    pub fn check_connection(&self, connection: &Arc<SearchConnectionState>) {
        *self.checked_connection.lock().expect("test setup") = Some(Arc::downgrade(connection));
    }
    fn publish(&self, outcome: SearchStartCancellationOutcome) {
        self.receipt.send_if_modified(|retained| {
            if retained.is_some() {
                return false;
            }
            *retained = Some(outcome);
            true
        });
    }
}
impl SearchStartControl for Start {
    fn request_cancel(&self) {
        if let Some(connection) = self
            .checked_connection
            .lock()
            .expect("test hook")
            .as_ref()
            .and_then(std::sync::Weak::upgrade)
            && connection.state.try_lock().is_err()
        {
            self.hook_under_fence.store(true, Ordering::Release);
        }
        self.cancelled.cancel();
    }
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        Box::pin(observe(self.receipt.subscribe()))
    }
}
async fn observe(
    mut receipt: watch::Receiver<Option<SearchStartCancellationOutcome>>,
) -> SearchStartCancellationOutcome {
    loop {
        if let Some(outcome) = receipt.borrow_and_update().clone() {
            return outcome;
        }
        if receipt.changed().await.is_err() {
            let error = SearchError::new(SearchErrorKind::ForcedShutdown, "fixture owner lost");
            return SearchStartCancellationOutcome {
                operation: Err(error.clone()),
                cleanup: StartCleanup::Unconfirmed(error),
            };
        }
    }
}
struct Guard {
    start: Option<Arc<Start>>,
    _task: TaskTrackerToken,
}
impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(start) = &self.start {
            start.request_cancel();
            let error = SearchError::new(
                SearchErrorKind::ForcedShutdown,
                "fixture constructor owner lost",
            );
            start.publish(SearchStartCancellationOutcome {
                operation: Err(start
                    .operation
                    .clone()
                    .err()
                    .unwrap_or_else(|| error.clone())),
                cleanup: StartCleanup::Unconfirmed(error),
            });
        }
    }
}
struct Backend {
    pending: Mutex<VecDeque<Arc<Start>>>,
    all: Vec<Arc<Start>>,
    closed: AtomicBool,
    tasks: TaskTracker,
}
impl SearchBackend for Backend {
    fn begin_open(&self, _: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        let mut pending = self.pending.lock().expect("admission fence");
        if self.closed.load(Ordering::Acquire) {
            return Err(SearchStartError {
                operation: SearchError::new(SearchErrorKind::ClosedLease, "fixture closed"),
                cleanup: StartCleanup::NotAdmitted,
            });
        }
        let start = pending.pop_front().ok_or_else(|| SearchStartError {
            operation: SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "fixture starts exhausted",
            ),
            cleanup: StartCleanup::NotAdmitted,
        })?;
        start.admitted.store(true, Ordering::Release);
        let owned = start.clone();
        let guard = Guard {
            start: Some(start.clone()),
            _task: self.tasks.token(),
        };
        let (reply, response) = oneshot::channel();
        tokio::spawn(async move {
            let mut guard = guard;
            owned.entered.add_permits(1);
            owned.cancelled.cancelled().await;
            owned
                .release
                .acquire()
                .await
                .expect("actual fixture cleanup gate")
                .forget();
            let outcome = SearchStartCancellationOutcome {
                operation: owned.operation.clone(),
                cleanup: StartCleanup::Confirmed,
            };
            let _ = reply.send(Err(SearchStartError {
                operation: outcome.operation.clone().err().unwrap_or_else(|| {
                    SearchError::new(
                        SearchErrorKind::ClosedLease,
                        "cancelled fixture construction",
                    )
                }),
                cleanup: outcome.cleanup.clone(),
            }));
            owned.publish(outcome);
            drop(guard.start.take());
        });
        let observer = start.clone();
        Ok(PendingSearchStart::new(
            start,
            Box::pin(async move {
                match response.await {
                    Ok(result) => result,
                    Err(_) => {
                        let outcome = observer.cancel_and_wait().await;
                        Err(SearchStartError {
                            operation: outcome.operation.err().unwrap_or_else(|| {
                                SearchError::new(
                                    SearchErrorKind::ForcedShutdown,
                                    "fixture response lost",
                                )
                            }),
                            cleanup: outcome.cleanup,
                        })
                    }
                }
            }),
        ))
    }
    fn request_shutdown(&self) {
        {
            let _fence = self.pending.lock().expect("admission fence");
            self.closed.store(true, Ordering::Release);
            self.tasks.close();
        }
        for start in &self.all {
            start.request_cancel();
        }
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.request_shutdown();
        Box::pin(async move {
            let mut result = SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Joined,
            };
            for start in &self.all {
                if start.admitted.load(Ordering::Acquire) {
                    let outcome = start.cancel_and_wait().await;
                    if result.operation.is_ok() {
                        result.operation = outcome.operation;
                    }
                    if let StartCleanup::Unconfirmed(error) = outcome.cleanup {
                        result.cleanup = CloseCleanup::Unconfirmed(error);
                    }
                }
            }
            self.tasks.wait().await;
            result
        })
    }
}

pub(super) fn fixture(
    starts: Vec<Arc<Start>>,
) -> (
    SearchRequestProcessor,
    FileSearchProvider,
    tokio::sync::mpsc::Receiver<crate::outgoing_message::OutgoingEnvelope>,
) {
    let nz = |value| NonZeroUsize::new(value).expect("positive test budget");
    let resources = SearchBudget {
        max_index_entries: nz(16),
        max_index_bytes: nz(1024 * 1024),
        max_worker_threads: nz(16),
    };
    let policy = RuntimePolicy {
        provider: ProviderLimits {
            max_scopes: nz(4),
            max_sessions: nz(16),
            resources,
        },
        max_query_bytes: nz(1024),
        max_roots_options_bytes: nz(1024),
        max_matches: nz(50),
        max_frame_retained_bytes: nz(64 * 1024),
        poll_wait: Duration::from_millis(20),
    };
    let backend = Arc::new(Backend {
        pending: Mutex::new(starts.iter().cloned().collect()),
        all: starts,
        closed: AtomicBool::new(false),
        tasks: TaskTracker::new(),
    });
    let provider = FileSearchProvider::from_backend(backend, policy).expect("real runtime facade");
    let mut options = crate::fuzzy_file_search::options();
    options.threads = NonZeroUsize::MIN;
    let context = SearchContext {
        shutdown_requested: CancellationToken::new(),
        factory: provider.scope_factory(),
        scope_limits: ScopeLimits {
            max_sessions: nz(16),
        },
        budget: SearchBudget {
            max_index_entries: NonZeroUsize::MIN,
            max_index_bytes: nz(1024),
            max_worker_threads: NonZeroUsize::MIN,
        },
        options,
        max_query_bytes: policy.max_query_bytes,
    };
    let (sender, receiver) = tokio::sync::mpsc::channel(16);
    let outgoing = Arc::new(OutgoingMessageSender::new(
        sender,
        codex_analytics::AnalyticsEventsClient::disabled(),
    ));
    (
        SearchRequestProcessor::new(outgoing, context),
        provider,
        receiver,
    )
}
pub(super) async fn entered(start: &Start) {
    tokio::time::timeout(Duration::from_secs(2), start.entered.acquire())
        .await
        .expect("startup admission")
        .expect("admission gate")
        .forget();
}
pub(super) async fn cancelled(start: &Start) {
    tokio::time::timeout(Duration::from_secs(2), start.cancelled.cancelled())
        .await
        .expect("actual pending control cancelled without releasing constructor gate");
}
