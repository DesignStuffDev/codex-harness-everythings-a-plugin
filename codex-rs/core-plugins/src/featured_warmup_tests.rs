use super::*;
use futures::FutureExt;
use pretty_assertions::assert_eq;
use std::sync::atomic::AtomicUsize;
use tokio::sync::oneshot;

fn scope(registry: &Arc<Registry>) -> Arc<FeaturedWarmupScope> {
    FeaturedWarmupScope::with_registry(Handle::current(), registry)
}

#[tokio::test]
async fn close_before_start_rejects_without_polling() {
    let registry = Arc::new(Registry::default());
    let owner = scope(&registry);
    owner.begin_close();
    assert_eq!(
        owner.start(async { panic!("closed work was polled") }),
        FeaturedWarmupAdmission::Closed
    );
    assert_eq!(owner.wait().await.ownership, FeaturedWarmupOwnership::Idle);
    assert!(lock(&registry.scopes).is_empty());
}

#[tokio::test]
async fn canceled_wait_keeps_handle_for_second_observer() {
    let registry = Arc::new(Registry::default());
    let owner = scope(&registry);
    let (started_tx, started_rx) = oneshot::channel();
    assert_eq!(
        owner.start(async move {
            let _ = started_tx.send(());
            std::future::pending::<FeaturedWarmupOutcome>().await
        }),
        FeaturedWarmupAdmission::Started
    );
    started_rx.await.expect("task entered");
    // Current-thread runtime: this single poll cannot run the canceled task yet.
    assert!(owner.wait().now_or_never().is_none());
    assert!(matches!(lock(&owner.state).record, Record::Running(_)));
    assert_eq!(lock(&registry.scopes).len(), 1);
    let observed = owner.wait().await;
    assert_eq!(observed.outcome, Some(FeaturedWarmupOutcome::Cancelled));
    assert!(observed.is_complete());
    assert!(lock(&registry.scopes).is_empty());
}

#[tokio::test]
async fn publication_is_suppressed_after_close() {
    let registry = Arc::new(Registry::default());
    let owner = scope(&registry);
    let published = AtomicUsize::new(0);
    assert_eq!(
        owner.publish_if_open(|| published.fetch_add(1, Ordering::SeqCst)),
        Some(0)
    );
    owner.begin_close();
    assert_eq!(
        owner.publish_if_open(|| published.fetch_add(1, Ordering::SeqCst)),
        None
    );
    assert_eq!(published.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn publication_holds_the_same_lock_used_by_close() {
    let registry = Arc::new(Registry::default());
    let owner = scope(&registry);
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let publisher = Arc::clone(&owner);
    let publication = std::thread::spawn(move || {
        publisher.publish_if_open(|| {
            entered_tx.send(()).expect("publication entered");
            release_rx.recv().expect("publication released");
            9
        })
    });
    entered_rx.recv().expect("publication holds fence");
    assert!(owner.state.try_lock().is_err());
    let closer = Arc::clone(&owner);
    let (closing_tx, closing_rx) = std::sync::mpsc::channel();
    let closing = std::thread::spawn(move || {
        closing_tx.send(()).expect("close requested");
        closer.begin_close();
    });
    closing_rx.recv().expect("closer entered");
    release_tx.send(()).expect("allow publication to finish");
    assert_eq!(publication.join().expect("publisher joined"), Some(9));
    closing.join().expect("closer joined");
    assert_eq!(owner.publish_if_open(|| 10), None);
}

#[tokio::test]
async fn closing_a_leaves_b_admission_and_publication_open() {
    let registry = Arc::new(Registry::default());
    let a = scope(&registry);
    let b = scope(&registry);
    a.begin_close();
    assert_eq!(b.publish_if_open(|| 7), Some(7));
    let (done_tx, done_rx) = oneshot::channel();
    assert_eq!(
        b.start(async move {
            let _ = done_tx.send(());
            FeaturedWarmupOutcome::Succeeded
        }),
        FeaturedWarmupAdmission::Started
    );
    done_rx.await.expect("B ran");
    let observed = b.wait().await;
    assert_eq!(observed.outcome, Some(FeaturedWarmupOutcome::Succeeded));
    assert!(observed.is_complete());
}

#[tokio::test]
async fn panic_is_joined_and_retained_as_unclean() {
    let registry = Arc::new(Registry::default());
    let owner = scope(&registry);
    let (entered_tx, entered_rx) = oneshot::channel();
    owner.start(async move {
        let _ = entered_tx.send(());
        panic!("warmup panic fixture");
    });
    entered_rx.await.expect("task ran");
    let observed = owner.wait().await;
    assert_eq!(observed.ownership, FeaturedWarmupOwnership::Panicked);
    assert!(!observed.is_complete());
    assert_eq!(lock(&registry.scopes).len(), 1);
    assert_eq!(owner.wait().await, observed);
}

#[tokio::test]
async fn ordinary_fetch_failure_is_clean_after_exact_join() {
    let registry = Arc::new(Registry::default());
    let owner = scope(&registry);
    let (done_tx, done_rx) = oneshot::channel();
    owner.start(async move {
        let _ = done_tx.send(());
        FeaturedWarmupOutcome::Failed
    });
    done_rx.await.expect("task ran");
    let observed = owner.wait().await;
    assert_eq!(observed.outcome, Some(FeaturedWarmupOutcome::Failed));
    assert!(observed.is_complete());
}

#[tokio::test]
async fn process_close_is_sticky_and_drain_uses_supplied_deadline() {
    let registry = Arc::new(Registry::default());
    let owner = scope(&registry);
    let (entered_tx, entered_rx) = oneshot::channel();
    owner.start(async move {
        let _ = entered_tx.send(());
        std::future::pending::<FeaturedWarmupOutcome>().await
    });
    entered_rx.await.expect("task entered");
    let drain = registry.begin_close();
    let late = scope(&registry);
    assert_eq!(
        late.start(async { panic!("late work") }),
        FeaturedWarmupAdmission::Closed
    );
    assert_eq!(owner.publish_if_open(|| 1), None);
    // Keep another observer's polling permit; expiry must retain the exact handle.
    let waiter = owner.waiter.acquire().await.expect("polling permit");
    let deadline_observation = drain.wait_until(Instant::now()).await;
    assert_eq!(deadline_observation.pending, 1);
    assert!(!deadline_observation.is_complete());
    assert!(matches!(lock(&owner.state).record, Record::Running(_)));
    drop(waiter);
    let joined = owner.wait().await;
    assert_eq!(joined.outcome, Some(FeaturedWarmupOutcome::Cancelled));
    let observed = drain.wait_until(Instant::now()).await;
    assert_eq!(observed.cancelled, 1);
    assert!(observed.is_complete());
}

#[tokio::test]
async fn registry_capacity_preserves_custody_and_reopens_only_after_clean_retirement() {
    let registry = Arc::new(Registry {
        capacity: 1,
        ..Default::default()
    });
    let first = scope(&registry);
    let (entered_tx, entered_rx) = oneshot::channel();
    first.start(async move {
        let _ = entered_tx.send(());
        std::future::pending::<FeaturedWarmupOutcome>().await
    });
    entered_rx.await.expect("first task entered");
    let denied = scope(&registry);
    assert_eq!(
        denied.start(async { panic!("capacity-denied work") }),
        FeaturedWarmupAdmission::CapacityExceeded
    );
    assert_eq!(denied.publish_if_open(|| 1), None);
    assert_eq!(lock(&registry.scopes).len(), 1);
    assert!(Arc::ptr_eq(&lock(&registry.scopes)[0], &first));
    assert!(matches!(lock(&first.state).record, Record::Running(_)));
    assert!(first.wait().await.is_complete());
    assert!(lock(&registry.scopes).is_empty());

    let replacement = scope(&registry);
    let (entered_tx, entered_rx) = oneshot::channel();
    assert_eq!(
        replacement.start(async move {
            let _ = entered_tx.send(());
            panic!("retained panic consumes capacity");
        }),
        FeaturedWarmupAdmission::Started
    );
    entered_rx.await.expect("replacement ran");
    assert!(!replacement.wait().await.is_complete());
    let denied_again = scope(&registry);
    assert_eq!(
        denied_again.start(async { panic!("panic custody evicted") }),
        FeaturedWarmupAdmission::CapacityExceeded
    );
    assert!(Arc::ptr_eq(&lock(&registry.scopes)[0], &replacement));
    registry.begin_close();
    assert_eq!(
        denied_again.start(async { panic!("closed registry") }),
        FeaturedWarmupAdmission::Closed
    );
}

#[tokio::test]
async fn canceled_queued_observer_leaves_the_next_observer_able_to_join() {
    let registry = Arc::new(Registry::default());
    let owner = scope(&registry);
    let (entered_tx, entered_rx) = oneshot::channel();
    owner.start(async move {
        let _ = entered_tx.send(());
        std::future::pending::<FeaturedWarmupOutcome>().await
    });
    entered_rx.await.expect("task entered");
    let permit = owner.waiter.acquire().await.expect("first observer permit");
    // Cancellation while queued must not consume the only polling permit.
    assert!(owner.wait().now_or_never().is_none());
    assert!(matches!(lock(&owner.state).record, Record::Running(_)));
    drop(permit);
    let observed = owner.wait().await;
    assert_eq!(
        observed,
        FeaturedWarmupObservation {
            ownership: FeaturedWarmupOwnership::Joined,
            outcome: Some(FeaturedWarmupOutcome::Cancelled),
        },
    );
    assert!(lock(&registry.scopes).is_empty());
}
