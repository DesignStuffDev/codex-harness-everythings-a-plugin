//! Service lifecycle checks using the production search facade and App Server
//! guards. The controlled backend admits no work and makes no native/process
//! extraction claim; its gate represents an outstanding shutdown receipt.

use std::future::Future;
use std::future::poll_fn;
use std::io;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::task::Poll;
use std::time::Duration;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::PendingSearchStart;
use codex_file_search_api::ProviderLimits;
use codex_file_search_api::ScopeLimits;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchStartError;
use codex_file_search_runtime::FileSearchProvider;
use codex_file_search_runtime::RuntimePolicy;
use pretty_assertions::assert_eq;
use tokio::sync::Semaphore;
use tokio::time::Instant;
use tokio::time::timeout;

use super::super::StoreShutdownGuard;
use super::super::finish_processor_and_services;
use super::CloseBehavior;
use super::ShutdownProbe;
use crate::file_search_services::SearchShutdownGuard;

struct SearchShutdownProbe {
    begun: AtomicBool,
    completed: AtomicBool,
    entered: Semaphore,
    release: Semaphore,
    outcome: SearchCloseOutcome,
}

impl SearchShutdownProbe {
    fn new(outcome: SearchCloseOutcome) -> Arc<Self> {
        Arc::new(Self {
            begun: AtomicBool::new(false),
            completed: AtomicBool::new(false),
            entered: Semaphore::new(0),
            release: Semaphore::new(0),
            outcome,
        })
    }
}

impl SearchBackend for SearchShutdownProbe {
    fn begin_open(&self, _: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        panic!("service lifecycle fixture must not admit search sessions");
    }

    fn request_shutdown(&self) {
        self.begun.store(true, Ordering::Release);
    }

    fn shutdown(&self) -> SearchCloseFuture<'_> {
        Box::pin(async move {
            self.entered.add_permits(1);
            self.release.acquire().await.expect("receipt gate").forget();
            self.completed.store(true, Ordering::Release);
            self.outcome.clone()
        })
    }
}

fn positive(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).expect("positive fixture bound")
}

fn search_guard(backend: &Arc<SearchShutdownProbe>) -> SearchShutdownGuard {
    let policy = RuntimePolicy {
        provider: ProviderLimits {
            max_scopes: positive(2),
            max_sessions: positive(1),
            resources: SearchBudget {
                max_index_entries: positive(1),
                max_index_bytes: positive(1024),
                max_worker_threads: positive(1),
            },
        },
        max_query_bytes: positive(64),
        max_roots_options_bytes: positive(1024),
        max_matches: positive(1),
        max_frame_retained_bytes: positive(4096),
        poll_wait: Duration::from_millis(10),
    };
    SearchShutdownGuard::new(
        FileSearchProvider::from_backend(backend.clone(), policy).expect("fixture provider"),
    )
}

fn joined() -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Joined,
    }
}

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    timeout(Duration::from_secs(2), future)
        .await
        .expect("controlled service transition must make progress")
}

#[tokio::test]
async fn dropping_either_search_guard_fences_surviving_owners_and_waits_for_the_receipt() {
    for drop_original in [false, true] {
        let backend = SearchShutdownProbe::new(joined());
        let original = search_guard(&backend);
        let fork = original.fork();
        let factory = original.factory();
        let scope = factory
            .new_scope(ScopeLimits {
                max_sessions: positive(1),
            })
            .expect("scope before shutdown");
        let surviving = if drop_original {
            drop(original);
            fork
        } else {
            drop(fork);
            original
        };

        assert!(backend.begun.load(Ordering::Acquire));
        let rejected = factory
            .new_scope(ScopeLimits {
                max_sessions: positive(1),
            })
            .err()
            .expect("a surviving explicit owner must not retain admission");
        assert_eq!(rejected.kind(), SearchErrorKind::ClosedLease);
        bounded(backend.entered.acquire()).await.unwrap().forget();
        assert_eq!(bounded(scope.shutdown()).await, joined());
        let finishing = surviving.finish();
        tokio::pin!(finishing);
        assert!(poll_fn(|cx| Poll::Ready(finishing.as_mut().poll(cx).is_pending())).await);
        assert!(!backend.completed.load(Ordering::Acquire));

        backend.release.add_permits(1);
        bounded(finishing).await.expect("joined search shutdown");
        assert!(backend.completed.load(Ordering::Acquire));
    }
}

#[tokio::test(start_paused = true)]
async fn stalled_search_and_storage_share_the_service_cleanup_budget() {
    let store = ShutdownProbe::new(CloseBehavior::Stall);
    let storage = StoreShutdownGuard::new(store.clone());
    let backend = SearchShutdownProbe::new(joined());
    let search = search_guard(&backend);
    let mut processor = tokio::spawn(std::future::pending::<io::Result<()>>());
    let started = Instant::now();

    let error = timeout(
        Duration::from_secs(200),
        finish_processor_and_services(&mut processor, &storage, &search),
    )
    .await
    .expect("processor plus concurrent service budgets must fit the runtime deadline")
    .expect_err("every stalled shutdown stage is reported");

    assert!(started.elapsed() >= Duration::from_secs(165));
    assert!(started.elapsed() < Duration::from_secs(200));
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    let message = error.to_string();
    assert!(message.contains("request processor shutdown timed out"));
    assert!(message.contains("thread-store shutdown timed out"));
    assert!(message.contains("file-search provider shutdown timed out"));
    assert!(processor.is_finished());
    assert!(store.begun.load(Ordering::Acquire));
    assert!(!store.completed.load(Ordering::Acquire));
    assert!(backend.begun.load(Ordering::Acquire));
    assert!(!backend.completed.load(Ordering::Acquire));

    // The observer timeout does not abandon the facade's retained cleanup.
    backend.release.add_permits(1);
    bounded(search.finish())
        .await
        .expect("later joined receipt");
    assert!(backend.completed.load(Ordering::Acquire));
}

#[tokio::test]
async fn storage_completes_while_search_cleanup_is_pending_and_then_fails() {
    let store = ShutdownProbe::new(CloseBehavior::Complete);
    let storage = StoreShutdownGuard::new(store.clone());
    let backend = SearchShutdownProbe::new(SearchCloseOutcome {
        operation: Err(SearchError::new(
            SearchErrorKind::SearchFailed,
            "search worker failed",
        )),
        cleanup: CloseCleanup::Joined,
    });
    let search = search_guard(&backend);
    let mut processor = tokio::spawn(async { Ok(()) });
    let finishing = tokio::spawn(async move {
        finish_processor_and_services(&mut processor, &storage, &search).await
    });
    bounded(backend.entered.acquire()).await.unwrap().forget();
    bounded(async {
        while !store.completed.load(Ordering::Acquire) {
            tokio::task::yield_now().await;
        }
    })
    .await;

    assert!(store.begun.load(Ordering::Acquire));
    assert!(!finishing.is_finished());
    assert!(!backend.completed.load(Ordering::Acquire));
    backend.release.add_permits(1);
    let error = bounded(finishing)
        .await
        .expect("helper task")
        .expect_err("search failure reaches the caller after storage drain");
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(error.to_string().contains("search worker failed"));
    assert!(backend.completed.load(Ordering::Acquire));
}

#[tokio::test]
async fn processor_error_keeps_its_kind_and_both_service_failures() {
    let store = ShutdownProbe::new(CloseBehavior::Fail);
    let storage = StoreShutdownGuard::new(store.clone());
    let backend = SearchShutdownProbe::new(SearchCloseOutcome {
        operation: Err(SearchError::new(
            SearchErrorKind::SearchFailed,
            "search worker failed",
        )),
        cleanup: CloseCleanup::Unconfirmed(SearchError::new(
            SearchErrorKind::TransportLost,
            "search worker join was not confirmed",
        )),
    });
    backend.release.add_permits(1);
    let search = search_guard(&backend);
    let mut processor = tokio::spawn(async {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "processor authorization failed",
        ))
    });

    let error = bounded(finish_processor_and_services(
        &mut processor,
        &storage,
        &search,
    ))
    .await
    .expect_err("all shutdown failures must survive");

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    let message = error.to_string();
    assert!(message.contains("processor authorization failed"));
    assert!(message.contains("thread-store shutdown failed"));
    assert!(message.contains("accepted write outcomes may be unknown"));
    assert!(message.contains("search worker failed"));
    assert!(message.contains("file-search provider cleanup unconfirmed"));
    assert!(message.contains("search worker join was not confirmed"));
    assert!(!message.contains("private-token"));
    assert!(store.begun.load(Ordering::Acquire));
    assert!(backend.completed.load(Ordering::Acquire));
}
