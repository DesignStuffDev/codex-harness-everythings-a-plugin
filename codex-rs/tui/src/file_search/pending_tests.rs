//! Preparing-phase ownership tests; controlled backend, not installed-plugin proof.
use super::AppEventSender;
use super::FileSearchDelivery;
use super::FileSearchManager;
use super::FileSearchRuntime;
use super::state::lock;
use super::test_support::*;
use crate::app_event::AppEvent;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchPhase;
use codex_file_search_api::SearchQuery;
use codex_file_search_api::SearchStartCancellationOutcome;
use codex_file_search_api::SearchStartControl;
use codex_file_search_api::StartCleanup;
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::timeout;

fn manager(runtime: &FileSearchRuntime) -> (FileSearchManager, UnboundedReceiver<AppEvent>) {
    let (tx, rx) = unbounded_channel();
    (
        FileSearchManager::new("/fixture".into(), AppEventSender::new(tx), runtime.clone()),
        rx,
    )
}

fn pending(manager: &FileSearchManager) -> Arc<dyn SearchStartControl> {
    Arc::clone(
        lock(&manager.state)
            .pending_start
            .as_ref()
            .expect("published start"),
    )
}

async fn delivered(
    manager: &FileSearchManager,
    rx: &mut UnboundedReceiver<AppEvent>,
) -> FileSearchDelivery {
    timeout(Duration::from_secs(2), async {
        loop {
            let event = rx.recv().await.expect("live delivery channel");
            if let AppEvent::FileSearchReady { wake } = event
                && let Some(delivery) = manager.take_delivery(&wake)
                && manager.accepts_delivery(&delivery)
            {
                return delivery;
            }
        }
    })
    .await
    .expect("current delivery deadline")
}

#[derive(Clone, Copy, Debug)]
enum Retirement {
    Empty,
    Root,
    Restart,
    Drop,
}

#[tokio::test]
async fn each_manager_retirement_cancels_preparing_before_open_and_retains_join() {
    for retirement in [
        Retirement::Empty,
        Retirement::Root,
        Retirement::Restart,
        Retirement::Drop,
    ] {
        let session = Session::new(/*acknowledgements*/ 1);
        session.close_ready.send_replace(false);
        let (runtime, provider, backend) = fixture(vec![Arc::clone(&session)]).await;
        backend.open_ready.send_replace(false);
        let (manager, rx) = manager(&runtime);
        let mut manager = Some(manager);
        manager
            .as_ref()
            .expect("manager")
            .on_user_query("A")
            .expect("query");
        permit(&backend.open_entered).await;
        let control = pending(manager.as_ref().expect("manager"));
        let old_state = Arc::clone(&manager.as_ref().expect("manager").state);
        match retirement {
            Retirement::Empty => manager
                .as_ref()
                .expect("manager")
                .on_user_query("")
                .expect("clear"),
            Retirement::Root => manager
                .as_mut()
                .expect("manager")
                .update_search_dir("/new-root".into()),
            Retirement::Restart => {
                let sender = lock(&old_state).tx.clone();
                manager
                    .as_mut()
                    .expect("manager")
                    .restart("/new-root".into(), sender);
            }
            Retirement::Drop => drop(manager.take()),
        }
        permit(&session.close_entered).await;
        assert!(session.closing.is_cancelled(), "{retirement:?}");
        assert!(
            !*backend.open_ready.borrow(),
            "fixture startup gate remains held"
        );
        assert!(lock(&session.updates).is_empty());
        assert!(lock(&old_state).pending_start.is_none());
        assert!(!lock(&old_state).preparing);
        assert_eq!(lock(&backend.opened).len(), 1);
        {
            let mut observer = control.cancel_and_wait();
            assert!(futures::poll!(&mut observer).is_pending());
        }
        // Abandon both passive observers. Neither owns the coordinator or close.
        {
            let observer = runtime.shutdown();
            tokio::pin!(observer);
            assert!(futures::poll!(&mut observer).is_pending());
        }
        assert_eq!(runtime.inner().tasks.len(), 1);
        session.close_ready.send_replace(true);
        timeout(Duration::from_secs(2), runtime.shutdown())
            .await
            .expect("retained drain deadline")
            .expect("joined retirement");
        let expected = SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::Confirmed,
        };
        for _ in 0..2 {
            assert_eq!(
                timeout(Duration::from_secs(2), control.cancel_and_wait())
                    .await
                    .expect("receipt"),
                expected,
            );
        }
        assert!(runtime.inner().tasks.is_empty());
        assert!(rx.is_empty(), "retired start must not deliver A");
        assert!(!*backend.open_ready.borrow());
        assert_eq!(
            session.close_entered.available_permits(),
            0,
            "one close owner"
        );
        assert_eq!(provider.shutdown().await, joined());
    }
}

#[tokio::test]
async fn nonempty_b_c_preserve_pending_lease_and_handoff_only_latest_query() {
    let session = Session::new(/*acknowledgements*/ 1);
    let (runtime, provider, backend) = fixture(vec![Arc::clone(&session)]).await;
    backend.open_ready.send_replace(false);
    let (manager, mut rx) = manager(&runtime);
    manager.on_user_query("A").expect("query A");
    permit(&backend.open_entered).await;
    let control = pending(&manager);
    manager.on_user_query("B").expect("query B");
    manager.on_user_query("C").expect("query C");
    let current = lock(&manager.state).request();
    assert!(Arc::ptr_eq(&control, &pending(&manager)));
    assert!(!session.closing.is_cancelled());
    assert!(lock(&session.updates).is_empty());
    assert_eq!(lock(&backend.opened).len(), 1);
    backend.open_ready.send_replace(true);
    permit(&session.admitted).await;
    assert_eq!(
        *lock(&session.updates),
        vec![SearchQuery {
            id: NonZeroU64::new(current.query_id).expect("query identity"),
            text: "C".into()
        }],
    );
    assert!(lock(&manager.state).pending_start.is_none());
    assert!(!lock(&manager.state).preparing);
    assert!(lock(&manager.state).session.is_some());
    session.emit(SearchPhase::Idle);
    let FileSearchDelivery::Matches {
        request,
        query,
        matches,
    } = delivered(&manager, &mut rx).await
    else {
        panic!("current matches");
    };
    assert!(manager.accepts(&request, &query));
    assert_eq!(
        (request.query_id, query, matches),
        (current.query_id, "C".into(), Vec::new())
    );
    timeout(Duration::from_secs(2), runtime.shutdown())
        .await
        .expect("drain deadline")
        .expect("joined");
    assert_eq!(provider.shutdown().await, joined());
}

#[tokio::test]
async fn retiring_old_preparing_lease_drains_it_before_new_root_c_handoff() {
    let old = Session::new(/*acknowledgements*/ 1);
    let current = Session::new(/*acknowledgements*/ 1);
    old.close_ready.send_replace(false);
    let (runtime, provider, backend) = fixture(vec![Arc::clone(&old), Arc::clone(&current)]).await;
    backend.open_ready.send_replace(false);
    let (mut manager, mut rx) = manager(&runtime);
    manager.on_user_query("A").expect("query A");
    permit(&backend.open_entered).await;
    let old_request = lock(&manager.state).request();
    let old_control = pending(&manager);
    manager.update_search_dir("/new-root".into());
    manager.on_user_query("B").expect("query B");
    manager.on_user_query("C").expect("query C");
    let latest = lock(&manager.state).request();
    permit(&old.close_entered).await;
    assert!(old.closing.is_cancelled());
    assert!(!current.closing.is_cancelled());
    assert_eq!(
        lock(&backend.opened).len(),
        1,
        "new admission waits for old join"
    );
    assert!(lock(&old.updates).is_empty());
    {
        let mut observer = old_control.cancel_and_wait();
        assert!(futures::poll!(&mut observer).is_pending());
    }
    old.close_ready.send_replace(true);
    permit(&backend.open_entered).await;
    let current_control = pending(&manager);
    assert!(!Arc::ptr_eq(&old_control, &current_control));
    assert_eq!(
        lock(&backend.opened)
            .iter()
            .map(|request| request.roots.clone())
            .collect::<Vec<_>>(),
        vec![
            vec![PathBuf::from("/fixture")],
            vec![PathBuf::from("/new-root")]
        ],
    );
    assert!(!*backend.open_ready.borrow());
    assert!(!current.closing.is_cancelled());
    assert!(lock(&old.updates).is_empty());
    assert!(lock(&current.updates).is_empty());
    assert!(!manager.accepts(&old_request, "A"));
    let expected = SearchStartCancellationOutcome {
        operation: Ok(()),
        cleanup: StartCleanup::Confirmed,
    };
    assert_eq!(
        timeout(Duration::from_secs(2), old_control.cancel_and_wait())
            .await
            .expect("old receipt"),
        expected
    );
    assert!(
        !current.closing.is_cancelled(),
        "old control must not target its successor"
    );
    backend.open_ready.send_replace(true);
    permit(&current.admitted).await;
    assert_eq!(
        *lock(&current.updates),
        vec![SearchQuery {
            id: NonZeroU64::new(latest.query_id).expect("query identity"),
            text: "C".into()
        }],
    );
    assert!(lock(&old.updates).is_empty());
    current.emit(SearchPhase::Idle);
    let FileSearchDelivery::Matches {
        request,
        query,
        matches,
    } = delivered(&manager, &mut rx).await
    else {
        panic!("current matches");
    };
    assert!(manager.accepts(&request, &query));
    assert_eq!(
        (request.query_id, query, matches),
        (latest.query_id, "C".into(), Vec::new())
    );
    timeout(Duration::from_secs(2), runtime.shutdown())
        .await
        .expect("drain deadline")
        .expect("joined");
    assert_eq!(provider.shutdown().await, joined());
}

#[tokio::test]
async fn late_genuine_failure_survives_retirement_with_original_cleanup_certainty() {
    let uncertain = SearchError::new(
        SearchErrorKind::ForcedShutdown,
        "late fixture cleanup uncertainty",
    );
    for (failure, cleanup) in [
        (
            SearchError::new(SearchErrorKind::ClosedLease, "spontaneous fixture closure"),
            CloseCleanup::Joined,
        ),
        (
            SearchError::new(
                SearchErrorKind::SearchFailed,
                "late fixture startup failure",
            ),
            CloseCleanup::Unconfirmed(uncertain),
        ),
    ] {
        let original = SearchCloseOutcome {
            operation: Err(failure.clone()),
            cleanup,
        };
        let session = Session::with_outcome(/*acknowledgements*/ 1, original.clone());
        session.close_ready.send_replace(false);
        let (runtime, provider, backend) = fixture(vec![Arc::clone(&session)]).await;
        backend.open_ready.send_replace(false);
        let (manager, rx) = manager(&runtime);
        manager.on_user_query("A").expect("query A");
        permit(&backend.open_entered).await;
        let control = pending(&manager);
        manager.on_user_query("").expect("retirement");
        permit(&session.close_entered).await;
        {
            let mut observer = control.cancel_and_wait();
            assert!(futures::poll!(&mut observer).is_pending());
        }
        assert!(
            lock(&runtime.inner().state).first_error.is_none(),
            "failure is still gated"
        );
        session.close_ready.send_replace(true);
        let expected = SearchStartCancellationOutcome {
            operation: original.operation.clone(),
            cleanup: original.cleanup.clone().into(),
        };
        for _ in 0..2 {
            assert_eq!(
                timeout(Duration::from_secs(2), control.cancel_and_wait())
                    .await
                    .expect("late receipt"),
                expected,
            );
        }
        let error = timeout(Duration::from_secs(2), runtime.shutdown())
            .await
            .expect("drain deadline")
            .expect_err("real failure survives requested cancellation");
        assert!(error.to_string().contains(failure.message()));
        if let CloseCleanup::Unconfirmed(cleanup) = &original.cleanup {
            assert!(error.to_string().contains("cleanup unconfirmed"));
            assert!(error.to_string().contains(cleanup.message()));
        }
        assert_eq!(lock(&runtime.inner().state).first_error, Some(failure));
        assert!(runtime.inner().tasks.is_empty());
        assert!(
            rx.is_empty(),
            "retired generation cannot publish a late error"
        );
        assert!(lock(&session.updates).is_empty());
        assert!(
            !*backend.open_ready.borrow(),
            "failure did not require fixture startup release"
        );
        assert_eq!(
            timeout(Duration::from_secs(2), provider.shutdown())
                .await
                .expect("provider drain"),
            original
        );
    }
}

#[tokio::test]
async fn dropping_ui_receiver_cancels_preparing_without_a_frame_or_open_release() {
    let session = Session::new(/*acknowledgements*/ 1);
    session.close_ready.send_replace(false);
    let (runtime, provider, backend) = fixture(vec![Arc::clone(&session)]).await;
    backend.open_ready.send_replace(false);
    let (manager, rx) = manager(&runtime);
    manager.on_user_query("A").expect("query A");
    permit(&backend.open_entered).await;
    let control = pending(&manager);
    drop(rx);
    // No frame, query, fixture startup release, or explicit cancellation wake.
    permit(&session.close_entered).await;
    assert!(session.closing.is_cancelled());
    assert!(!*backend.open_ready.borrow());
    assert!(lock(&session.updates).is_empty());
    assert!(lock(&manager.state).pending_start.is_none());
    assert!(!lock(&manager.state).preparing);
    let failure = SearchError::new(
        SearchErrorKind::TransportLost,
        "TUI file-search delivery channel is closed",
    );
    assert_eq!(
        lock(&runtime.inner().state).first_error,
        Some(failure.clone())
    );
    for _ in 0..2 {
        assert!(!super::reporter::retry_pending(&runtime.inner()));
        let mut observer = control.cancel_and_wait();
        assert!(futures::poll!(&mut observer).is_pending());
    }
    assert_eq!(
        session.close_entered.available_permits(),
        0,
        "one retained close owner"
    );
    assert_eq!(runtime.inner().tasks.len(), 1);
    session.close_ready.send_replace(true);
    let error = timeout(Duration::from_secs(2), runtime.shutdown())
        .await
        .expect("drain deadline")
        .expect_err("delivery failure remains visible");
    assert!(error.to_string().contains(failure.message()));
    assert_eq!(lock(&runtime.inner().state).first_error, Some(failure));
    assert_eq!(
        timeout(Duration::from_secs(2), control.cancel_and_wait())
            .await
            .expect("joined cancellation"),
        SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::Confirmed
        },
    );
    assert!(runtime.inner().tasks.is_empty());
    assert_eq!(provider.shutdown().await, joined());
}
