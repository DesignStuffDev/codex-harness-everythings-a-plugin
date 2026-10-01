use super::*;
use crate::app_event::AppEvent;
use crate::history_cell::HistoryCell;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileMatch;
use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::MatchType;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchPhase;
use codex_file_search_api::SessionReporter;
use pretty_assertions::assert_eq;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::timeout;

use super::reporter::TuiSessionReporter;
use super::test_support::*;

fn manager(runtime: &FileSearchRuntime) -> (FileSearchManager, UnboundedReceiver<AppEvent>) {
    let (tx, rx) = unbounded_channel();
    (
        FileSearchManager::new("/fixture".into(), AppEventSender::new(tx), runtime.clone()),
        rx,
    )
}
async fn delivery(
    manager: &FileSearchManager,
    rx: &mut UnboundedReceiver<AppEvent>,
) -> FileSearchDelivery {
    timeout(Duration::from_secs(2), async {
        loop {
            if let Some(AppEvent::FileSearchReady { wake }) = rx.recv().await
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

#[tokio::test]
async fn blocked_ack_coalesces_b_c_to_latest_without_parallel_updates() {
    let session = Session::new(/*acknowledgements*/ 0);
    let (runtime, _provider, backend) = fixture(vec![Arc::clone(&session)]).await;
    let (manager, mut rx) = manager(&runtime);
    manager.on_user_query("A").expect("query admission");
    permit(&session.admitted).await;
    manager.on_user_query("B").expect("query admission");
    manager.on_user_query("C").expect("query admission");
    assert_eq!(
        lock(&session.updates)
            .iter()
            .map(|query| query.text.as_str())
            .collect::<Vec<_>>(),
        vec!["A"]
    );
    assert_eq!(runtime.inner().tasks.len(), 1);
    session.acknowledgements.add_permits(1);
    permit(&session.admitted).await;
    assert_eq!(
        lock(&session.updates)
            .iter()
            .map(|query| query.text.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "C"]
    );
    assert_eq!(lock(&backend.opened).len(), 1);
    session.acknowledgements.add_permits(1);
    session.emit(SearchPhase::Idle);
    let FileSearchDelivery::Matches { query, .. } = delivery(&manager, &mut rx).await else {
        panic!("matches");
    };
    assert_eq!(query, "C");
    runtime.shutdown().await.expect("joined cleanup");
}

#[tokio::test]
async fn old_a_failure_retires_pending_b_and_allows_explicit_same_text_retry() {
    let first = Session::new(/*acknowledgements*/ 0);
    let retry = Session::new(/*acknowledgements*/ 1);
    let (runtime, _provider, backend) = fixture(vec![Arc::clone(&first), Arc::clone(&retry)]).await;
    let (manager, mut rx) = manager(&runtime);
    manager.on_user_query("A").expect("query admission");
    permit(&first.admitted).await;
    manager.on_user_query("B").expect("query admission");
    *lock(&first.update_error) = Some(SearchError::new(
        SearchErrorKind::SearchFailed,
        "matcher unavailable",
    ));
    first.acknowledgements.add_permits(1);
    let FileSearchDelivery::Failed { query, error, .. } = delivery(&manager, &mut rx).await else {
        panic!("failure");
    };
    assert_eq!(
        (query, error.kind()),
        ("B".into(), SearchErrorKind::SearchFailed)
    );
    let cell = crate::history_cell::new_error_event(FileSearchDelivery::error_notice(&error));
    let rendered = cell
        .display_lines(/*width*/ 80)
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(rendered, @"■ File search failed: matcher unavailable");
    assert!(lock(&manager.state).session.is_none());
    assert!(!lock(&manager.state).preparing);
    assert_eq!(lock(&backend.opened).len(), 1, "pending B is not replayed");
    manager.on_user_query("B").expect("query admission");
    permit(&retry.admitted).await;
    assert_eq!(lock(&retry.updates)[0].text, "B");
    runtime
        .shutdown()
        .await
        .expect_err("first operation failure remains visible");
}

#[tokio::test]
async fn bounded_result_slot_and_voice_filter_retry_keep_latest_payload() {
    let (runtime, _provider, _backend) = fixture(vec![]).await;
    let (manager, mut rx) = manager(&runtime);
    let generation = {
        let mut state = lock(&manager.state);
        state.latest_query = "A".into();
        assert!(state.advance());
        state.tx.voice_only.store(true, Ordering::Relaxed);
        state.request()
    };
    let reporter = TuiSessionReporter {
        state: Arc::downgrade(&manager.state),
        runtime: Arc::downgrade(&runtime.inner()),
        generation: generation.clone(),
    };
    let mut snapshot = FileSearchSnapshot {
        query_id: generation.query_id,
        query: "A".into(),
        matches: Vec::new(),
        total_match_count: 0,
        scanned_file_count: 0,
        walk_complete: false,
    };
    for index in 0..1_000 {
        snapshot.matches = vec![FileMatch {
            path: format!("marker-{index}.rs").into(),
            root: "/fixture".into(),
            score: index,
            match_type: MatchType::File,
            indices: Some(vec![0]),
        }];
        reporter.on_update(&snapshot);
    }
    assert!(rx.is_empty());
    assert!(!lock(&manager.state).wake_queued);
    lock(&manager.state)
        .tx
        .voice_only
        .store(false, Ordering::Relaxed);
    assert!(!reporter::retry_pending(&runtime.inner()));
    assert_eq!(rx.len(), 1);
    for _ in 0..1_000 {
        reporter.on_update(&snapshot);
    }
    assert_eq!(rx.len(), 1, "updates replace one slot while UI is stalled");
    let received = delivery(&manager, &mut rx).await;
    let FileSearchDelivery::Matches { matches, .. } = received else {
        panic!("latest matches");
    };
    assert_eq!(
        (matches[0].path.clone(), matches[0].score),
        (PathBuf::from("marker-999.rs"), 999)
    );
    assert!(!lock(&manager.state).wake_queued);
    runtime.shutdown().await.expect("cleanup");
}

#[tokio::test]
async fn intentional_retirement_does_not_relabel_joined_close_as_failure() {
    let session = Session::new(/*acknowledgements*/ 1);
    let (runtime, _provider, _backend) = fixture(vec![Arc::clone(&session)]).await;
    let (manager, mut rx) = manager(&runtime);
    manager.on_user_query("A").expect("query admission");
    permit(&session.admitted).await;
    session.emit(SearchPhase::Idle);
    let old = match delivery(&manager, &mut rx).await {
        FileSearchDelivery::Matches { request, .. } => request,
        _ => panic!("matches"),
    };
    manager.on_user_query("").expect("empty query");
    // Deterministic final fence for a passive close observer that already woke
    // before the UI's empty-query invalidation; it must not poison shutdown.
    reporter::fail(
        &manager.state,
        &old,
        &runtime.inner(),
        SearchError::new(SearchErrorKind::ClosedLease, "late joined close"),
    );
    assert!(lock(&runtime.inner().state).first_error.is_none());
    runtime
        .shutdown()
        .await
        .expect("deliberate joined close remains success");
}

#[tokio::test]
async fn unsolicited_cancelled_terminal_has_failure_delivery_and_no_idle() {
    let session = Session::new(/*acknowledgements*/ 1);
    let (runtime, _provider, _backend) = fixture(vec![Arc::clone(&session)]).await;
    let (manager, mut rx) = manager(&runtime);
    manager.on_user_query("A").expect("query admission");
    permit(&session.admitted).await;
    session.emit(SearchPhase::Cancelled);
    let FileSearchDelivery::Failed { query, error, .. } = delivery(&manager, &mut rx).await else {
        panic!("terminal failure");
    };
    assert_eq!(
        (query, error.kind()),
        ("A".into(), SearchErrorKind::ClosedLease)
    );
    runtime
        .shutdown()
        .await
        .expect_err("retained unexpected closure");
}

#[tokio::test]
async fn abandoned_shutdown_observer_keeps_drain_and_retained_failure() {
    let failure = SearchError::new(SearchErrorKind::SearchFailed, "original matcher failure");
    let uncertainty = SearchError::new(SearchErrorKind::ForcedShutdown, "fixture forced cleanup");
    let session = Session::with_outcome(
        /*acknowledgements*/ 1,
        SearchCloseOutcome {
            operation: Err(failure.clone()),
            cleanup: CloseCleanup::Unconfirmed(uncertainty),
        },
    );
    session.close_ready.send_replace(false);
    let (runtime, _provider, _backend) = fixture(vec![Arc::clone(&session)]).await;
    let (manager, _rx) = manager(&runtime);
    manager.on_user_query("A").expect("query admission");
    permit(&session.admitted).await;
    {
        let observer = runtime.shutdown();
        tokio::pin!(observer);
        assert!(futures::poll!(&mut observer).is_pending());
    }
    permit(&session.close_entered).await;
    session.close_ready.send_replace(true);
    let error = timeout(Duration::from_secs(2), runtime.shutdown())
        .await
        .expect("drain deadline")
        .expect_err("uncertain cleanup");
    let message = error.to_string();
    assert!(message.contains("original matcher failure"));
    assert!(message.contains("cleanup unconfirmed"));
    assert!(runtime.inner().tasks.is_empty());
}

#[tokio::test]
async fn oversized_utf8_query_is_not_retained_and_clears_old_generation() {
    let (runtime, _provider, _backend) = fixture(vec![]).await;
    let (manager, mut rx) = manager(&runtime);
    let rejected = "é".repeat(runtime.max_query_bytes() / 2 + 1);
    let error = manager
        .on_user_query(&rejected)
        .expect_err("synchronous bounded rejection");
    assert!(lock(&manager.state).latest_query.is_empty());
    assert!(
        rx.try_recv().is_err(),
        "oversized text must not be placed in the async mailbox"
    );
    assert_eq!(error.kind(), SearchErrorKind::ResourceExhausted);
    assert!(error.message().len() < 200);
    runtime
        .shutdown()
        .await
        .expect_err("explicit rejection remains visible");
}

#[test]
fn unpolled_coordinator_drop_retains_uncertainty_and_original_failure() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("executor");
    let runtime = {
        let _entered = executor.enter();
        let runtime = FileSearchRuntime::for_tests();
        runtime.inner().failed(SearchError::new(
            SearchErrorKind::SearchFailed,
            "original matcher failure",
        ));
        runtime
    };
    // The coordinator was enqueued but never polled. Its guard must already be
    // owned by the task, and public runtime handles remain alive during teardown.
    drop(executor);
    let observer = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("observer executor");
    let error = observer.block_on(async {
        timeout(Duration::from_secs(2), runtime.shutdown())
            .await
            .expect("lost owner receipt deadline")
            .expect_err("unconfirmed cleanup")
    });
    let message = error.to_string();
    assert!(message.contains("original matcher failure"));
    assert!(message.contains("cleanup unconfirmed"));
}

#[tokio::test]
async fn same_channel_restarts_reuse_one_wake_and_only_current_payload() {
    let (runtime, _provider, _backend) = fixture(vec![]).await;
    let (tx, mut rx) = unbounded_channel();
    let sender = AppEventSender::new(tx);
    let mut manager = FileSearchManager::new("/fixture".into(), sender.clone(), runtime.clone());
    for index in 0..100 {
        manager.restart("/fixture".into(), sender.clone());
        let generation = {
            let mut state = lock(&manager.state);
            state.latest_query = "current".into();
            state.advance();
            state.request()
        };
        let reporter = TuiSessionReporter {
            state: Arc::downgrade(&manager.state),
            runtime: Arc::downgrade(&runtime.inner()),
            generation: generation.clone(),
        };
        reporter.on_update(&FileSearchSnapshot {
            query_id: generation.query_id,
            query: "current".into(),
            matches: vec![FileMatch {
                path: format!("current-{index}").into(),
                root: "/fixture".into(),
                score: index,
                match_type: MatchType::File,
                indices: None,
            }],
            ..Default::default()
        });
    }
    assert_eq!(rx.len(), 1);
    let FileSearchDelivery::Matches { matches, .. } = delivery(&manager, &mut rx).await else {
        panic!("current matches");
    };
    assert_eq!(matches[0].path, PathBuf::from("current-99"));
    runtime.shutdown().await.expect("joined cleanup");
}

#[tokio::test]
async fn retired_channels_share_bounded_wake_credits_and_release_for_current_ui() {
    let (runtime, _provider, _backend) = fixture(vec![]).await;
    let (mut manager, rx) = manager(&runtime);
    let mut receivers = vec![rx];
    for index in 0..32 {
        let (tx, rx) = unbounded_channel();
        manager.restart("/fixture".into(), AppEventSender::new(tx));
        receivers.push(rx);
        let generation = {
            let mut state = lock(&manager.state);
            state.latest_query = "current".into();
            state.advance();
            state.request()
        };
        TuiSessionReporter {
            state: Arc::downgrade(&manager.state),
            runtime: Arc::downgrade(&runtime.inner()),
            generation: generation.clone(),
        }
        .on_update(&FileSearchSnapshot {
            query_id: generation.query_id,
            query: "current".into(),
            matches: vec![FileMatch {
                path: format!("current-{index}").into(),
                root: "/fixture".into(),
                score: index,
                match_type: MatchType::File,
                indices: None,
            }],
            ..Default::default()
        });
    }
    assert_eq!(
        receivers.iter().map(UnboundedReceiver::len).sum::<usize>(),
        8
    );
    let mut current = receivers.pop().expect("current receiver");
    drop(receivers);
    let FileSearchDelivery::Matches { matches, .. } = delivery(&manager, &mut current).await else {
        panic!("current matches");
    };
    assert_eq!(matches[0].path, PathBuf::from("current-31"));
    runtime.shutdown().await.expect("joined cleanup");
}

#[tokio::test]
async fn filtered_payload_retries_while_original_update_ack_is_still_pending() {
    let session = Session::new(/*acknowledgements*/ 0);
    let (runtime, _provider, _backend) = fixture(vec![Arc::clone(&session)]).await;
    let (manager, mut rx) = manager(&runtime);
    manager.on_user_query("A").expect("query admission");
    permit(&session.admitted).await;
    let generation = {
        let state = lock(&manager.state);
        state.tx.voice_only.store(true, Ordering::Relaxed);
        state.request()
    };
    // Exercise the real callback and coordinator while its same update observer
    // remains pending; there is deliberately no later ack or backend frame.
    TuiSessionReporter {
        state: Arc::downgrade(&manager.state),
        runtime: Arc::downgrade(&runtime.inner()),
        generation: generation.clone(),
    }
    .on_update(&FileSearchSnapshot {
        query_id: generation.query_id,
        query: "A".into(),
        matches: vec![FileMatch {
            path: "current.rs".into(),
            root: "/fixture".into(),
            score: 1,
            match_type: MatchType::File,
            indices: None,
        }],
        ..Default::default()
    });
    assert!(rx.is_empty());
    lock(&manager.state)
        .tx
        .voice_only
        .store(false, Ordering::Relaxed);
    let FileSearchDelivery::Matches { matches, .. } = delivery(&manager, &mut rx).await else {
        panic!("current matches");
    };
    assert_eq!(matches[0].path, PathBuf::from("current.rs"));
    assert_eq!(lock(&session.updates).len(), 1);
    assert_eq!(session.acknowledgements.available_permits(), 0);
    assert_eq!(runtime.inner().tasks.len(), 1);
    runtime.shutdown().await.expect("joined cleanup");
}

#[tokio::test]
async fn requested_shutdown_during_accepted_open_joins_without_synthetic_failure() {
    let session = Session::new(/*acknowledgements*/ 1);
    let (runtime, _provider, backend) = fixture(vec![Arc::clone(&session)]).await;
    backend.open_ready.send_replace(false);
    let (manager, mut rx) = manager(&runtime);
    manager.on_user_query("A").expect("query admission");
    permit(&backend.open_entered).await;
    runtime.request_shutdown();
    assert!(session.updates.lock().unwrap().is_empty());
    backend.open_ready.send_replace(true);
    timeout(Duration::from_secs(2), runtime.shutdown())
        .await
        .expect("pending startup cleanup deadline")
        .expect("requested close is successful joined cleanup");
    assert!(session.closing.is_cancelled());
    assert!(runtime.inner().tasks.is_empty());
    assert!(rx.try_recv().is_err());
}
