//! Public-carrier tests: these exercise the real SDK drop/handoff code, not a
//! functioning backend. No fixture below claims to join workers or refund quota.
use codex_file_search_api::PendingSearchStart;
use codex_file_search_api::QueryAccepted;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchFuture;
use codex_file_search_api::SearchPoll;
use codex_file_search_api::SearchQuery;
use codex_file_search_api::SearchStartCancellationFuture;
use codex_file_search_api::SearchStartControl;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::SearchStartFuture;
use codex_file_search_api::StartCleanup;
use pretty_assertions::assert_eq;
use std::future::Future;
use std::future::pending;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;

#[derive(Default)]
struct CancellationProbe {
    calls: AtomicUsize,
}

impl SearchStartControl for CancellationProbe {
    fn request_cancel(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }

    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        // This probe never supplies cleanup proof.
        Box::pin(pending())
    }
}

fn ticket(
    probe: &Arc<CancellationProbe>,
    completion: SearchStartFuture<'static>,
) -> PendingSearchStart {
    PendingSearchStart::new(probe.clone(), completion)
}

fn poll_once<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

struct DropObservedCompletion {
    probe: Arc<CancellationProbe>,
    calls_at_drop: Arc<AtomicUsize>,
}

impl Future for DropObservedCompletion {
    type Output = Result<Arc<dyn SearchBackendSession>, SearchStartError>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

impl Drop for DropObservedCompletion {
    fn drop(&mut self) {
        self.calls_at_drop
            .store(self.probe.calls.load(Ordering::SeqCst), Ordering::SeqCst);
    }
}

#[test]
fn unpolled_ticket_requests_cancel_before_discarding_observer() {
    let probe = Arc::new(CancellationProbe::default());
    let calls_at_drop = Arc::new(AtomicUsize::new(0));
    let ticket = ticket(
        &probe,
        Box::pin(DropObservedCompletion {
            probe: probe.clone(),
            calls_at_drop: calls_at_drop.clone(),
        }),
    );
    let retained_control = ticket.control();
    drop(ticket);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
    assert_eq!(calls_at_drop.load(Ordering::SeqCst), 1);
    drop(retained_control);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn unpolled_finish_retains_armed_guard() {
    let probe = Arc::new(CancellationProbe::default());
    let finish = ticket(&probe, Box::pin(pending())).finish();
    drop(finish);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn polled_pending_finish_retains_armed_guard() {
    let probe = Arc::new(CancellationProbe::default());
    let mut finish = ticket(&probe, Box::pin(pending())).finish();
    assert!(poll_once(finish.as_mut()).is_pending());
    drop(finish);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn abandonment_invokes_only_the_tickets_control() {
    let first = Arc::new(CancellationProbe::default());
    let sibling = Arc::new(CancellationProbe::default());
    let first_ticket = ticket(&first, Box::pin(pending()));
    let sibling_ticket = ticket(&sibling, Box::pin(pending()));
    drop(first_ticket);
    assert_eq!(first.calls.load(Ordering::SeqCst), 1);
    assert_eq!(sibling.calls.load(Ordering::SeqCst), 0);
    drop(sibling_ticket);
    assert_eq!(sibling.calls.load(Ordering::SeqCst), 1);
}

// Only the identity of this returned trait object is used; no operation runs.
struct HandoffSession;

impl SearchBackendSession for HandoffSession {
    fn update_query(&self, _: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        unreachable!("handoff identity only")
    }

    fn next_snapshot(&self, _: u64, _: Duration) -> SearchFuture<'_, SearchPoll> {
        unreachable!("handoff identity only")
    }

    fn request_close(&self) {
        unreachable!("handoff identity only")
    }

    fn close(&self) -> SearchCloseFuture<'_> {
        unreachable!("handoff identity only")
    }
}

#[test]
fn ready_success_disarms_ticket_but_preserves_control() {
    let probe = Arc::new(CancellationProbe::default());
    let session: Arc<dyn SearchBackendSession> = Arc::new(HandoffSession);
    let expected = session.clone();
    let mut ticket = ticket(&probe, Box::pin(async move { Ok(session) }));
    let control = ticket.control();
    let Poll::Ready(Ok(actual)) = poll_once(Pin::new(&mut ticket)) else {
        panic!("expected session handoff");
    };
    drop(ticket);
    assert!(Arc::ptr_eq(&actual, &expected));
    assert_eq!(probe.calls.load(Ordering::SeqCst), 0);
    control.request_cancel();
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn ready_error_preserves_receipt_without_spurious_cancel() {
    for cleanup in [
        StartCleanup::NotAdmitted,
        StartCleanup::Confirmed,
        StartCleanup::Unconfirmed(SearchError::new(
            SearchErrorKind::TransportLost,
            "lost receipt",
        )),
    ] {
        let probe = Arc::new(CancellationProbe::default());
        let expected = SearchStartError {
            operation: SearchError::new(SearchErrorKind::SearchFailed, "start failed"),
            cleanup,
        };
        let error = expected.clone();
        let mut finish = ticket(&probe, Box::pin(async move { Err(error) })).finish();
        let Poll::Ready(Err(actual)) = poll_once(finish.as_mut()) else {
            panic!("expected startup failure");
        };
        drop(finish);
        assert_eq!(actual, expected);
        assert_eq!(probe.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn completion_poll_unwind_does_not_disarm_abandonment() {
    let probe = Arc::new(CancellationProbe::default());
    let mut ticket = ticket(
        &probe,
        Box::pin(async { panic!("completion observer failed while polling") }),
    );
    let poll = std::panic::catch_unwind(AssertUnwindSafe(|| {
        drop(poll_once(Pin::new(&mut ticket)));
    }));
    assert!(poll.is_err());
    assert_eq!(probe.calls.load(Ordering::SeqCst), 0);
    drop(ticket);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
}

struct PanickingControl(Arc<CancellationProbe>);

impl SearchStartControl for PanickingControl {
    fn request_cancel(&self) {
        self.0.request_cancel();
        panic!("cancellation hook violated its infallibility contract");
    }

    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        Box::pin(pending())
    }
}

#[test]
fn destructor_contains_hook_unwind_without_replacing_outer_unwind() {
    let probe = Arc::new(CancellationProbe::default());
    let calls_at_drop = Arc::new(AtomicUsize::new(0));
    let ticket = PendingSearchStart::new(
        Arc::new(PanickingControl(probe.clone())),
        Box::pin(DropObservedCompletion {
            probe: probe.clone(),
            calls_at_drop: calls_at_drop.clone(),
        }),
    );
    let drop_result = std::panic::catch_unwind(AssertUnwindSafe(move || {
        let _ticket = ticket;
        panic!("outer caller panic");
    }));
    let outer_panic = drop_result.expect_err("outer caller panic must remain observable");
    assert_eq!(
        outer_panic.downcast_ref::<&str>(),
        Some(&"outer caller panic")
    );
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
    assert_eq!(calls_at_drop.load(Ordering::SeqCst), 1);
}

// Admission probes test the public default observer contract only. They do not
// run a matcher or manufacture cleanup proof for accepted backend work.
struct AdmissionProbe {
    calls: AtomicUsize,
    control: Arc<CancellationProbe>,
    rejection: Option<SearchStartError>,
}
impl codex_file_search_api::SearchBackend for AdmissionProbe {
    fn begin_open(
        &self,
        request: codex_file_search_api::SearchOpen,
    ) -> Result<PendingSearchStart, SearchStartError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            request.roots,
            vec![std::path::PathBuf::from("explicit-root")]
        );
        match &self.rejection {
            Some(error) => Err(error.clone()),
            None => Ok(ticket(&self.control, Box::pin(pending()))),
        }
    }
    fn request_shutdown(&self) {
        unreachable!("admission probe only")
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        unreachable!("admission probe only")
    }
}
fn probe_request() -> codex_file_search_api::SearchOpen {
    let one = std::num::NonZeroUsize::MIN;
    codex_file_search_api::SearchOpen {
        roots: vec!["explicit-root".into()],
        options: codex_file_search_api::FileSearchOptions::default(),
        budget: codex_file_search_api::SearchBudget {
            max_index_entries: one,
            max_index_bytes: one,
            max_worker_threads: one,
        },
    }
}
#[test]
fn default_open_reserves_before_poll_and_unpolled_drop_reaches_its_control() {
    use codex_file_search_api::SearchBackend;
    let backend = AdmissionProbe {
        calls: AtomicUsize::new(0),
        control: Arc::new(CancellationProbe::default()),
        rejection: None,
    };
    let observer = backend.open(probe_request());
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    assert_eq!(backend.control.calls.load(Ordering::SeqCst), 0);
    drop(observer);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    assert_eq!(backend.control.calls.load(Ordering::SeqCst), 1);
}
#[test]
fn default_open_preserves_synchronous_rejection_without_cancel_or_readmission() {
    use codex_file_search_api::SearchBackend;
    let expected = SearchStartError {
        operation: SearchError::new(SearchErrorKind::ResourceExhausted, "admission full"),
        cleanup: StartCleanup::NotAdmitted,
    };
    let backend = AdmissionProbe {
        calls: AtomicUsize::new(0),
        control: Arc::new(CancellationProbe::default()),
        rejection: Some(expected.clone()),
    };
    let mut observer = backend.open(probe_request());
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    let Poll::Ready(Err(actual)) = poll_once(observer.as_mut()) else {
        panic!("expected immediate rejection");
    };
    assert_eq!(actual, expected);
    drop(observer);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    assert_eq!(backend.control.calls.load(Ordering::SeqCst), 0);
}
