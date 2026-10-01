#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests of the production facade's ledgers/owners with controllable backend
//! boundaries. Native worker and installed-process proof belong integration gates.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::state::lock;
use codex_file_search_api::*;
use pretty_assertions::assert_eq;
#[path = "test_support.rs"]
mod support;
use support::*;

#[path = "startup_cancel_tests.rs"]
mod startup_cancellation;

#[tokio::test]
async fn scopes_isolate_close_and_share_provider_capacity() {
    let one = Session::new(1, joined());
    let two = Session::new(1, joined());
    let three = Session::new(1, joined());
    let backend = Backend::new(vec![Arc::clone(&one), Arc::clone(&two), three], 3);
    let provider = provider(Arc::clone(&backend));
    let a = scope(&provider);
    let b = scope(&provider);
    let first = a
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    let second = b
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    let rejected = a
        .open(request(), Arc::new(Reporter::default()))
        .await
        .err()
        .unwrap();
    assert_eq!(rejected.cleanup, StartCleanup::NotAdmitted);
    assert_eq!(
        rejected.operation.kind(),
        SearchErrorKind::ResourceExhausted
    );
    assert_eq!(bounded(a.shutdown()).await, joined());
    assert!(!backend.stopped.load(Ordering::SeqCst));
    assert!(!two.closing.load(Ordering::SeqCst));
    assert_eq!(
        second
            .update_query(query(1, "active"))
            .await
            .unwrap()
            .id
            .get(),
        1
    );
    assert_eq!(bounded(first.close()).await, joined());
    assert_eq!(bounded(provider.shutdown()).await, joined());
    assert!(two.closing.load(Ordering::SeqCst));
    assert_eq!(lock(&provider.inner.state).sessions, 0);
}

#[tokio::test]
async fn abandoned_startup_keeps_capacity_until_late_backend_close_joins() {
    let native = Session::new(0, joined());
    let backend = Backend::new(vec![Arc::clone(&native)], 0);
    let provider = provider(Arc::clone(&backend));
    let scope = scope(&provider);
    let waiting_scope = scope.clone();
    let opening = tokio::spawn(async move {
        waiting_scope
            .open(request(), Arc::new(Reporter::default()))
            .await
    });
    bounded(backend.opened.acquire()).await.unwrap().forget();
    opening.abort();
    let _ = opening.await;
    assert_eq!(lock(&provider.inner.state).sessions, 1);
    backend.starts.add_permits(1);
    bounded(native.close_entered.acquire())
        .await
        .unwrap()
        .forget();
    let rejected = scope
        .open(request(), Arc::new(Reporter::default()))
        .await
        .err()
        .unwrap();
    assert_eq!(
        rejected.operation.kind(),
        SearchErrorKind::ResourceExhausted
    );
    native.closes.add_permits(1);
    assert_eq!(bounded(scope.shutdown()).await, joined());
    assert_eq!(native.close_count.load(Ordering::SeqCst), 1);
    assert_eq!(lock(&provider.inner.state).sessions, 0);
    assert_eq!(bounded(provider.shutdown()).await, joined());
}

#[tokio::test]
async fn dropped_update_observer_retains_single_slot_and_close_waits_its_reply() {
    let native = Session::new(1, joined());
    native.updates.acquire_many(100).await.unwrap().forget();
    let backend = Backend::new(vec![Arc::clone(&native)], 1);
    let provider = provider(backend);
    let scope = scope(&provider);
    let session = scope
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    let observing = session.clone();
    let task = tokio::spawn(async move { observing.update_query(query(1, "first")).await });
    bounded(native.update_entered.acquire())
        .await
        .unwrap()
        .forget();
    task.abort();
    let _ = task.await;
    assert_eq!(
        session
            .update_query(query(2, "second"))
            .await
            .err()
            .unwrap()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
    let close = session.close();
    assert!(
        tokio::time::timeout(Duration::from_millis(20), close)
            .await
            .is_err()
    );
    assert_eq!(lock(&provider.inner.state).sessions, 1);
    native.updates.add_permits(1);
    assert_eq!(bounded(session.close()).await, joined());
    assert_eq!(bounded(provider.shutdown()).await, joined());
}

#[tokio::test]
async fn failed_but_joined_close_releases_slot_and_retains_operation_error() {
    let native = Session::new(
        1,
        SearchCloseOutcome {
            operation: Err(failure()),
            cleanup: CloseCleanup::Joined,
        },
    );
    let next = Session::new(1, joined());
    let backend = Backend::new(vec![native, next], 2);
    let provider = provider(backend);
    let scope = scope(&provider);
    let first = scope
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    let closed = bounded(first.close()).await;
    assert_eq!(
        closed,
        SearchCloseOutcome {
            operation: Err(failure()),
            cleanup: CloseCleanup::Joined
        }
    );
    let second = scope
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    assert_eq!(bounded(second.close()).await, joined());
    assert_eq!(bounded(scope.shutdown()).await, closed);
    assert_eq!(bounded(provider.shutdown()).await, closed);
}

#[tokio::test]
async fn factories_are_bounded_and_provider_incarnations_are_independent() {
    let provider = provider(Backend::new(vec![], 0));
    let factory = provider.scope_factory();
    let first = scope(&provider);
    let second = scope(&provider);
    assert!(factory.owns_scope(&first));
    assert_eq!(
        factory
            .new_scope(ScopeLimits {
                max_sessions: positive(1)
            })
            .err()
            .unwrap()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
    let other = support::provider(Backend::new(vec![], 0));
    assert!(!other.scope_factory().owns_scope(&first));
    assert_eq!(bounded(first.shutdown()).await, joined());
    let replacement = scope(&provider);
    drop(second);
    drop(replacement);
    let shutdown = provider.shutdown(); // fence before polling its returned observer
    assert_eq!(
        factory
            .new_scope(ScopeLimits {
                max_sessions: positive(1)
            })
            .err()
            .unwrap()
            .kind(),
        SearchErrorKind::ClosedLease
    );
    assert_eq!(bounded(shutdown).await, joined());
    assert_eq!(bounded(other.shutdown()).await, joined());
}

#[tokio::test]
async fn current_query_only_idle_delivers_snapshot_before_tagged_completion() {
    let native = Session::new(1, joined());
    let provider = provider(Backend::new(vec![Arc::clone(&native)], 1));
    let scope = scope(&provider);
    let reporter = Arc::new(Reporter::default());
    let session = scope.open(request(), reporter.clone()).await.unwrap();
    session.update_query(query(1, "A")).await.unwrap();
    session.update_query(query(2, "B")).await.unwrap();
    native.emit(1, 1, "A", true);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(*lock(&reporter.events), vec![]);
    native.emit(2, 2, "B", true);
    reporter.wait_len(2).await;
    assert_eq!(
        *lock(&reporter.events),
        vec![Event::Update(2, "B".to_owned()), Event::Complete(2)]
    );
    session.update_query(query(3, "A")).await.unwrap();
    native.emit(3, 3, "A", false);
    reporter.wait_len(3).await;
    native.emit(4, 3, "A", true);
    reporter.wait_len(5).await;
    assert_eq!(
        *lock(&reporter.events),
        vec![
            Event::Update(2, "B".to_owned()),
            Event::Complete(2),
            Event::Update(3, "A".to_owned()),
            Event::Update(3, "A".to_owned()),
            Event::Complete(3)
        ]
    );
    assert_eq!(bounded(provider.shutdown()).await, joined());
}

#[path = "callback_tests.rs"]
mod callbacks;

#[path = "cause_race_tests.rs"]
mod cause_races;
