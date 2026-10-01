#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::FileSearchSnapshot;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

enum Callback {
    Observe,
    Block(crossbeam_channel::Receiver<()>),
    Panic,
}

enum Event {
    Snapshot(FileSearchSnapshot),
    Complete(u64),
}

struct Reporter {
    updates: mpsc::UnboundedSender<Event>,
    callback: Callback,
}

impl SessionReporter for Reporter {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        let _ = self.updates.send(Event::Snapshot(snapshot.clone()));
        if snapshot.query_id != 0 {
            match &self.callback {
                Callback::Observe => {}
                Callback::Block(release) => {
                    let _ = release.recv();
                }
                Callback::Panic => panic!("injected asynchronous-client reporter failure"),
            }
        }
    }
    fn on_complete(&self) {}
    fn on_complete_tagged(&self, query_id: u64) {
        let _ = self.updates.send(Event::Complete(query_id));
    }
}

fn new_reporter(callback: Callback) -> (Arc<Reporter>, mpsc::UnboundedReceiver<Event>) {
    let (updates, receiver) = mpsc::unbounded_channel();
    (Arc::new(Reporter { updates, callback }), receiver)
}

async fn snapshot(receiver: &mut mpsc::UnboundedReceiver<Event>, id: u64) -> FileSearchSnapshot {
    timeout(WAIT, async {
        loop {
            if let Event::Snapshot(snapshot) =
                receiver.recv().await.expect("reporter must remain live")
                && snapshot.query_id == id
            {
                return snapshot;
            }
        }
    })
    .await
    .expect("query must produce a tagged snapshot")
}

fn tree() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("apple.txt"), "a").unwrap();
    std::fs::write(root.path().join("banana.txt"), "b").unwrap();
    root
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tagged_a_b_a_queries_reject_reused_ids_and_closed_clones() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (reporter, mut updates) = new_reporter(Callback::Observe);
    let session = owner
        .create(
            vec![root.path().into()],
            FileSearchOptions::default(),
            reporter,
            /*cancel_flag*/ None,
        )
        .await
        .unwrap();
    assert!(session.update_query_tagged("apple", 0).is_err());
    for (id, query, path) in [
        (1, "apple", "apple.txt"),
        (2, "banana", "banana.txt"),
        (3, "apple", "apple.txt"),
    ] {
        session.update_query_tagged(query, id).unwrap();
        let result = timeout(WAIT, async {
            let mut latest = None;
            loop {
                match updates.recv().await.expect("reporter must remain live until completion") {
                    Event::Snapshot(snapshot) if snapshot.query_id == id => {
                        assert_eq!(snapshot.query, query);
                        assert!(
                            snapshot.matches.iter().all(|item| item.path == std::path::Path::new(path)),
                            "query {id} ({query}) published another pattern's matches: {snapshot:?}",
                        );
                        latest = Some(snapshot);
                    }
                    Event::Complete(completed_id) if completed_id == id => {
                        break latest.expect("completed query must have published its final snapshot");
                    }
                    Event::Snapshot(_) | Event::Complete(_) => {}
                }
            }
        }).await.expect("query must finish with its own tagged completion");
        assert_eq!(
            result
                .matches
                .iter()
                .map(|item| item.path.clone())
                .collect::<Vec<_>>(),
            vec![PathBuf::from(path)]
        );
        assert!(session.update_query_tagged(query, id).is_err());
    }
    let clone = session.clone();
    timeout(WAIT, session.close()).await.unwrap().unwrap();
    assert!(clone.update_query_tagged("banana", 4).is_err());
    timeout(WAIT, clone.close()).await.unwrap().unwrap();
    timeout(WAIT, owner.shutdown()).await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aborted_close_waiter_retains_capacity_until_callback_and_threads_join() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (release, gate) = crossbeam_channel::bounded(1);
    let (reporter, mut updates) = new_reporter(Callback::Block(gate));
    let session = owner
        .create(
            vec![root.path().into()],
            FileSearchOptions::default(),
            reporter,
            /*cancel_flag*/ None,
        )
        .await
        .unwrap();
    session.update_query_tagged("apple", 1).unwrap();
    snapshot(&mut updates, 1).await;
    session.request_close();
    let waiter = tokio::spawn(session.close());
    waiter.abort();
    let _ = waiter.await;
    let (next_reporter, _) = new_reporter(Callback::Observe);
    let capacity_result = owner
        .create(
            vec![root.path().into()],
            FileSearchOptions::default(),
            next_reporter,
            /*cancel_flag*/ None,
        )
        .await;
    let draining_owner = owner.clone();
    let mut drain = tokio::spawn(async move { draining_owner.shutdown().await });
    let early = timeout(Duration::from_millis(50), &mut drain).await;
    // Always unblock native work before checking lifecycle assertions.
    release.send(()).unwrap();
    assert!(
        capacity_result.is_err(),
        "closing sessions retain their slot"
    );
    assert!(
        early.is_err(),
        "shutdown must wait beyond cancelled close waiter"
    );
    timeout(WAIT, drain).await.unwrap().unwrap().unwrap();
    timeout(WAIT, owner.shutdown()).await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_final_lease_releases_reporter_and_shutdown_fences_new_sessions() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (reporter, _) = new_reporter(Callback::Observe);
    let weak = Arc::downgrade(&reporter);
    let session = owner
        .create(
            vec![root.path().into()],
            FileSearchOptions::default(),
            reporter,
            /*cancel_flag*/ None,
        )
        .await
        .unwrap();
    let clone = session.clone();
    drop(session);
    clone.update_query_tagged("apple", 1).unwrap();
    drop(clone);
    timeout(WAIT, owner.shutdown()).await.unwrap().unwrap();
    assert!(weak.upgrade().is_none());
    let (reporter, _) = new_reporter(Callback::Observe);
    assert!(
        owner
            .create(
                vec![root.path().into()],
                FileSearchOptions::default(),
                reporter,
                /*cancel_flag*/ None
            )
            .await
            .is_err()
    );
}

#[test]
fn cancelled_preparing_waiter_keeps_ownership_until_startup_and_cleanup_finish() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let root = tree();
        let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
        let (release, gate) = crossbeam_channel::bounded(1);
        let (entered, started) = crossbeam_channel::bounded(1);
        let blocker = tokio::task::spawn_blocking(move || {
            entered.send(()).unwrap();
            gate.recv().unwrap();
        });
        started.recv_timeout(WAIT).unwrap();
        let (reporter, _) = new_reporter(Callback::Observe);
        let weak = Arc::downgrade(&reporter);
        let mut pending = Box::pin(owner.create(
            vec![root.path().into()],
            FileSearchOptions::default(),
            reporter,
            /*cancel_flag*/ None,
        ));
        std::future::poll_fn(|context| {
            assert!(std::future::Future::poll(pending.as_mut(), context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        drop(pending);
        let (other, _) = new_reporter(Callback::Observe);
        let denied = owner
            .create(
                vec![root.path().into()],
                FileSearchOptions::default(),
                other,
                /*cancel_flag*/ None,
            )
            .await;
        owner.request_shutdown();
        release.send(()).unwrap();
        blocker.await.unwrap();
        assert!(denied.is_err());
        timeout(WAIT, owner.shutdown()).await.unwrap().unwrap();
        assert!(weak.upgrade().is_none());
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_failure_is_observable_and_retained_by_repeated_shutdown_waiters() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (reporter, mut updates) = new_reporter(Callback::Panic);
    let session = owner
        .create(
            vec![root.path().into()],
            FileSearchOptions::default(),
            reporter,
            /*cancel_flag*/ None,
        )
        .await
        .unwrap();
    session.update_query_tagged("apple", 1).unwrap();
    snapshot(&mut updates, 1).await;
    timeout(WAIT, async {
        while !session.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(session.close().await.is_err());
    let first = owner.shutdown().await.unwrap_err().to_string();
    assert_eq!(owner.shutdown().await.unwrap_err().to_string(), first);
    assert!(first.contains("reporter panicked"));
}

#[derive(Clone, Copy)]
enum Teardown {
    Succeed,
    Fail,
}

fn observed_cancelled_start(teardown: Teardown) -> (anyhow::Error, anyhow::Result<()>) {
    struct GatedDropReporter {
        teardown: Teardown,
        entered: Option<tokio::sync::oneshot::Sender<()>>,
        release: crossbeam_channel::Receiver<()>,
    }
    impl SessionReporter for GatedDropReporter {
        fn on_update(&self, _snapshot: &FileSearchSnapshot) {}
        fn on_complete(&self) {}
    }
    impl Drop for GatedDropReporter {
        fn drop(&mut self) {
            if let Some(entered) = self.entered.take() {
                let _ = entered.send(());
            }
            self.release.recv().unwrap();
            if matches!(self.teardown, Teardown::Fail) {
                panic!("injected reporter teardown failure after joined native threads");
            }
        }
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let root = tree();
        let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
        let (allow_start, start_gate) = crossbeam_channel::bounded(1);
        let (blocker_entered, blocker_started) = crossbeam_channel::bounded(1);
        let blocker = tokio::task::spawn_blocking(move || {
            blocker_entered.send(()).unwrap();
            start_gate.recv().unwrap();
        });
        blocker_started.recv_timeout(WAIT).unwrap();
        let (drop_entered, drop_started) = tokio::sync::oneshot::channel();
        let (finish_drop, drop_gate) = crossbeam_channel::bounded(1);
        let reporter = Arc::new(GatedDropReporter {
            teardown,
            entered: Some(drop_entered),
            release: drop_gate,
        });
        let weak = Arc::downgrade(&reporter);
        let creating_owner = owner.clone();
        let mut pending = Box::pin(async move {
            creating_owner
                .create(
                    vec![root.path().into()],
                    FileSearchOptions::default(),
                    reporter,
                    /*cancel_flag*/ None,
                )
                .await
        });
        std::future::poll_fn(|context| {
            assert!(std::future::Future::poll(pending.as_mut(), context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        owner.request_shutdown();
        allow_start.send(()).unwrap();
        blocker.await.unwrap();
        let mut creation = tokio::spawn(pending);
        timeout(WAIT, drop_started).await.unwrap().unwrap();
        let early = timeout(Duration::from_millis(50), &mut creation).await;
        // Unblock cleanup even if creation incorrectly returned its error early.
        finish_drop.send(()).unwrap();
        let result = match early {
            Ok(result) => {
                let _ = result;
                panic!("an observed startup rejection must wait for accepted cleanup");
            }
            Err(_) => timeout(WAIT, creation).await.unwrap().unwrap(),
        };
        let error = match result {
            Ok(_) => panic!("shutdown during startup must reject creation"),
            Err(error) => error,
        };
        assert!(weak.upgrade().is_none());
        (error, timeout(WAIT, owner.shutdown()).await.unwrap())
    })
}

#[test]
fn observed_cancelled_start_waits_for_cleanup_and_preserves_both_errors() {
    let (error, shutdown) = observed_cancelled_start(Teardown::Fail);
    let typed = error
        .downcast_ref::<FileSearchStartError>()
        .expect("observed startup failure has a typed cleanup outcome");
    assert!(
        typed
            .operation_error()
            .to_string()
            .contains("closed during startup")
    );
    assert!(
        typed
            .cleanup_error()
            .expect("failed teardown must remain observable")
            .to_string()
            .contains("close task failed")
    );
    assert!(error.to_string().contains("startup cleanup failed"));
    assert!(shutdown.is_err());
}

#[test]
fn observed_cooperative_start_cancellation_has_no_cleanup_failure() {
    let (error, shutdown) = observed_cancelled_start(Teardown::Succeed);
    let typed = error
        .downcast_ref::<FileSearchStartError>()
        .expect("observed cancellation has a typed cleanup outcome");
    assert!(
        typed
            .operation_error()
            .to_string()
            .contains("closed during startup")
    );
    assert!(typed.cleanup_error().is_none());
    shutdown.expect("cooperative cancellation must drain cleanly");
}

#[path = "async_owner_receipt_tests.rs"]
mod receipts;
