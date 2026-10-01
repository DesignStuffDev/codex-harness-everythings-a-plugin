//! Real walker/matcher lifecycle tests; channels gate callbacks and observe release.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel::bounded;
use crossbeam_channel::unbounded;
use pretty_assertions::assert_eq;

use crate::FileSearchOptions;
use crate::FileSearchSession;
use crate::FileSearchSnapshot;
use crate::SessionReporter;
use crate::create_session;

const WAIT: Duration = Duration::from_secs(5);

enum Callback {
    Observe,
    Block(Receiver<()>),
    Panic,
}

struct Probe {
    updates: Sender<FileSearchSnapshot>,
    completed: Sender<()>,
    dropped: Sender<()>,
    callback: Callback,
}

struct Observations {
    updates: Receiver<FileSearchSnapshot>,
    completed: Receiver<()>,
    dropped: Receiver<()>,
}

impl SessionReporter for Probe {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        let _ = self.updates.send(snapshot.clone());
        if snapshot.query == "needle"
            && snapshot.matches.iter().any(|entry| entry.path == std::path::Path::new("needle.txt"))
        {
            match &self.callback {
                Callback::Observe => {}
                Callback::Block(release) => {
                    release.recv().expect("test must release the matched-file callback");
                }
                Callback::Panic => panic!("deliberate matched-file reporter failure"),
            }
        }
    }

    fn on_complete(&self) {
        let _ = self.completed.send(());
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = self.dropped.send(());
    }
}

impl Observations {
    fn wait_for_match(&self) {
        let deadline = Instant::now() + WAIT;
        loop {
            let snapshot = self.updates
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("real query must find the fixture file");
            if snapshot.query == "needle"
                && snapshot.matches.iter().any(|entry| entry.path == std::path::Path::new("needle.txt"))
            {
                return;
            }
        }
    }
}

fn probe(callback: Callback) -> (Arc<Probe>, Observations) {
    let (updates_tx, updates) = unbounded();
    let (completed_tx, completed) = unbounded();
    let (dropped_tx, dropped) = bounded(1);
    (
        Arc::new(Probe {
            updates: updates_tx,
            completed: completed_tx,
            dropped: dropped_tx,
            callback,
        }),
        Observations { updates, completed, dropped },
    )
}

fn tree() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("needle.txt"), "real search input").unwrap();
    std::fs::write(root.path().join("other.txt"), "unmatched input").unwrap();
    root
}

fn close_bounded(session: FileSearchSession) -> anyhow::Result<()> {
    let (closed_tx, closed) = bounded(1);
    let closer = thread::spawn(move || {
        let _ = closed_tx.send(session.close());
    });
    let result = closed.recv_timeout(WAIT).expect("close must join workers within the deadline");
    closer.join().expect("close caller must not panic");
    result
}

#[test]
fn idle_close_joins_workers_and_releases_reporter() {
    let root = tree();
    let (reporter, observations) = probe(Callback::Observe);
    let weak_reporter = Arc::downgrade(&reporter);
    let session = create_session(
        vec![root.path().to_path_buf()],
        FileSearchOptions::default(),
        reporter,
        /*cancel_flag*/ None,
    ).unwrap();
    observations.completed.recv_timeout(WAIT).expect("initial walk must become idle");
    close_bounded(session).unwrap();
    observations.dropped.recv_timeout(WAIT).expect("joined workers must release reporter ownership");
    assert!(weak_reporter.upgrade().is_none());
}

#[test]
fn close_waits_for_blocked_callback_but_request_close_does_not() {
    let root = tree();
    let (release_tx, release) = bounded(1);
    let (reporter, observations) = probe(Callback::Block(release));
    let weak_reporter = Arc::downgrade(&reporter);
    let session = create_session(
        vec![root.path().to_path_buf()],
        FileSearchOptions::default(),
        reporter,
        /*cancel_flag*/ None,
    ).unwrap();
    session.update_query("needle");
    observations.wait_for_match();

    let (requested_tx, requested) = bounded(1);
    let (closed_tx, closed) = bounded(1);
    let closer = thread::spawn(move || {
        session.request_close();
        let _ = requested_tx.send(());
        let _ = closed_tx.send(session.close());
    });
    let request_result = requested.recv_timeout(WAIT);
    let early_close = closed.recv_timeout(Duration::from_millis(100));
    let closed_early = early_close.is_ok();
    // Release the callback even when the implementation violates either assertion.
    let _ = release_tx.send(());
    let close_result = early_close
        .or_else(|_| closed.recv_timeout(WAIT))
        .expect("close must complete after callback release");
    closer.join().expect("close caller must not panic");
    request_result.expect("request_close must return while the callback is blocked");
    assert!(!closed_early, "consuming close must wait for the in-flight callback");
    close_result.unwrap();
    observations.dropped.recv_timeout(WAIT).expect("close must release reporter ownership");
    assert!(weak_reporter.upgrade().is_none());
}

#[test]
fn reporter_panic_is_returned_by_close_and_does_not_retain_workers() {
    let root = tree();
    let (reporter, observations) = probe(Callback::Panic);
    let weak_reporter = Arc::downgrade(&reporter);
    let session = create_session(
        vec![root.path().to_path_buf()],
        FileSearchOptions::default(),
        reporter,
        /*cancel_flag*/ None,
    ).unwrap();
    session.update_query("needle");
    observations.wait_for_match();
    assert!(close_bounded(session).is_err(), "worker failure must not be reported as a clean close");
    observations.dropped.recv_timeout(WAIT).expect("panic cleanup must release reporter ownership");
    assert!(weak_reporter.upgrade().is_none());
}

#[test]
fn reporter_panic_wakes_one_shot_waiter_without_completion_callback() {
    let root = tree();
    let (reporter, observations) = probe(Callback::Panic);
    let session = create_session(
        vec![root.path().to_path_buf()],
        FileSearchOptions::default(),
        reporter,
        /*cancel_flag*/ None,
    ).unwrap();
    let (waited_tx, waited) = bounded(1);
    let waiter = thread::spawn(move || {
        // This independent reporter never receives on_update or on_complete.
        // Only the session's worker-finished signal can release its wait.
        let one_shot = crate::RunReporter::default();
        session.update_query("needle");
        let snapshot = one_shot.wait_for_complete(&session);
        let _ = waited_tx.send((snapshot, session));
    });
    observations.wait_for_match();
    let (snapshot, session) = waited
        .recv_timeout(WAIT)
        .expect("reporter panic must wake a one-shot waiter without on_complete");
    waiter.join().expect("one-shot waiter must not panic");
    assert_eq!(snapshot, FileSearchSnapshot::default());
    assert!(close_bounded(session).is_err(), "worker failure must remain observable after wait");
}

#[test]
fn explicit_close_does_not_cancel_sibling_using_shared_external_flag() {
    let root = tree();
    let external = Arc::new(AtomicBool::new(false));
    let (first_reporter, _) = probe(Callback::Observe);
    let first = create_session(
        vec![root.path().to_path_buf()],
        FileSearchOptions::default(),
        first_reporter,
        Some(Arc::clone(&external)),
    ).unwrap();
    let (reporter, observations) = probe(Callback::Observe);
    let sibling = create_session(
        vec![root.path().to_path_buf()],
        FileSearchOptions::default(),
        reporter,
        Some(Arc::clone(&external)),
    ).unwrap();

    first.request_close();
    close_bounded(first).unwrap();
    assert!(!external.load(Ordering::SeqCst));
    sibling.update_query("needle");
    observations.wait_for_match();
    close_bounded(sibling).unwrap();
    assert!(!external.load(Ordering::SeqCst));
}

#[test]
fn dropping_session_eventually_releases_reporter_ownership() {
    let root = tree();
    let (reporter, observations) = probe(Callback::Observe);
    let weak_reporter = Arc::downgrade(&reporter);
    let session = create_session(
        vec![root.path().to_path_buf()],
        FileSearchOptions::default(),
        reporter,
        /*cancel_flag*/ None,
    ).unwrap();
    session.update_query("needle");
    observations.wait_for_match();
    drop(session);
    observations.dropped.recv_timeout(WAIT).expect("drop must eventually drain workers and release reporter");
    assert!(weak_reporter.upgrade().is_none());
}
