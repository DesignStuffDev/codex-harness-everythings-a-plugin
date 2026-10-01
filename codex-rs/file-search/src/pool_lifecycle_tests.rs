#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use pretty_assertions::assert_eq;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

#[test]
fn partial_pool_start_failure_joins_every_started_thread() {
    let exited = Arc::new(AtomicUsize::new(0));
    let mut threads = PoolThreads::default();
    let mut spawned = 0;
    let result = threads.build(
        3,
        |_| {},
        |worker| {
            if spawned == 2 {
                return Err(io::Error::other("injected third-thread spawn failure"));
            }
            spawned += 1;
            let exited = exited.clone();
            thread::Builder::new().spawn(move || {
                worker.run();
                exited.fetch_add(1, Ordering::Release);
            })
        },
    );
    assert!(result.is_err());
    assert_eq!(exited.load(Ordering::Acquire), 2);
    assert!(threads.handles.is_empty());
}

#[test]
fn join_waits_beyond_rayon_worker_exit_until_os_thread_finishes() {
    let (after_rayon_tx, after_rayon_rx) = bounded(1);
    let (release_tx, release_rx) = bounded(1);
    let (joining_tx, joining_rx) = bounded(1);
    let (done_tx, done_rx) = bounded(1);
    let exited = Arc::new(AtomicBool::new(false));
    let observed_exit = exited.clone();
    let owner = thread::spawn(move || {
        let mut threads = PoolThreads::default();
        let pool = threads
            .build(
                1,
                |_| {},
                |worker| {
                    let after_rayon_tx = after_rayon_tx.clone();
                    let release_rx = release_rx.clone();
                    let exited = exited.clone();
                    thread::Builder::new().spawn(move || {
                        worker.run();
                        after_rayon_tx.send(()).unwrap();
                        release_rx.recv().unwrap();
                        exited.store(true, Ordering::Release);
                    })
                },
            )
            .unwrap();
        drop(pool);
        joining_tx.send(()).unwrap();
        let failures = threads.join();
        done_tx.send(failures).unwrap();
    });
    after_rayon_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    joining_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(observed_exit.load(Ordering::Acquire), false);
    assert!(done_rx.try_recv().is_err());
    release_tx.send(()).unwrap();
    assert_eq!(done_rx.recv_timeout(Duration::from_secs(5)).unwrap(), Vec::<String>::new());
    owner.join().unwrap();
    assert_eq!(observed_exit.load(Ordering::Acquire), true);
}

#[test]
fn pool_thread_panic_is_reported_after_join() {
    let mut threads = PoolThreads::default();
    let pool = threads
        .build(
            1,
            |_| {},
            |worker| thread::Builder::new().spawn(move || {
                worker.run();
                panic!("injected OS-thread teardown panic");
            }),
        )
        .unwrap();
    drop(pool);
    assert_eq!(threads.join(), vec!["file-search pool thread panicked"]);
}

#[test]
fn asynchronous_rayon_task_panic_is_observed_without_aborting() {
    let (failure_tx, failure_rx) = bounded(1);
    let mut threads = PoolThreads::default();
    let pool = threads
        .build(
            1,
            move |_| { let _ = failure_tx.send(()); },
            spawn_pool_thread,
        )
        .unwrap();
    pool.spawn(|| panic!("injected asynchronous matcher-task panic"));
    failure_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(pool);
    assert_eq!(threads.join(), Vec::<String>::new());
}

#[test]
fn walker_owner_joins_during_supervisor_unwind() {
    let root = tempfile::tempdir().unwrap();
    let native = crate::create_session(
        vec![root.path().into()],
        crate::FileSearchOptions::default(),
        Arc::new(crate::RunReporter::default()),
        /*cancel_flag*/ None,
    ).unwrap();
    let inner = native.inner.clone();
    native.close().unwrap();
    let (release, gate) = bounded(1);
    let (unwinding, unwind_started) = bounded(1);
    let (done, joined) = bounded(1);
    let exited = Arc::new(AtomicBool::new(false));
    let observed = exited.clone();
    let supervisor = thread::spawn(move || {
        let walker = thread::spawn(move || {
            gate.recv().unwrap();
            exited.store(true, Ordering::Release);
            Ok(())
        });
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _walker = WalkerThread { handle: Some(walker), inner };
            unwinding.send(()).unwrap();
            panic!("injected unexpected supervisor unwind");
        }));
        done.send(result.is_err()).unwrap();
    });
    unwind_started.recv_timeout(Duration::from_secs(5)).unwrap();
    let early = joined.recv_timeout(Duration::from_millis(50));
    release.send(()).unwrap();
    assert!(early.is_err(), "unwind must not detach its walker");
    assert_eq!(joined.recv_timeout(Duration::from_secs(5)).unwrap(), true);
    supervisor.join().unwrap();
    assert_eq!(observed.load(Ordering::Acquire), true);
}

#[test]
fn changed_old_pattern_snapshot_is_not_published_with_new_query_identity() {
    use crate::FileSearchSnapshot;
    use crate::IndexedEntry;
    use crate::MatchType;
    use crate::SessionReporter;
    use nucleo::Utf32String;
    use nucleo::pattern::CaseMatching;
    use nucleo::pattern::Normalization;
    use std::sync::Mutex;

    struct Reporter(crossbeam_channel::Sender<FileSearchSnapshot>);
    impl SessionReporter for Reporter {
        fn on_update(&self, snapshot: &FileSearchSnapshot) { let _ = self.0.send(snapshot.clone()); }
        fn on_complete(&self) {}
    }
    let root = tempfile::tempdir().unwrap();
    let (work_tx, work_rx) = crossbeam_channel::unbounded();
    let (updates, observed) = crossbeam_channel::unbounded();
    let inner = Arc::new(SessionInner {
        search_directories: vec![root.path().into()],
        limit: 20,
        threads: 1,
        compute_indices: true,
        respect_gitignore: true,
        cancelled: Arc::new(AtomicBool::new(false)),
        shutdown: Arc::new(AtomicBool::new(false)),
        last_query_id: Mutex::new(0),
        reporter: Arc::new(Reporter(updates)),
        work_tx: work_tx.clone(),
    });
    let mut pool_threads = PoolThreads::default();
    let pool = pool_threads.build(1, |_| {}, spawn_pool_thread).unwrap();
    let (first_release, first_gate) = bounded(1);
    let (first_entered, first_started) = bounded(1);
    pool.spawn(move || { first_entered.send(()).unwrap(); first_gate.recv().unwrap(); });
    first_started.recv_timeout(Duration::from_secs(5)).unwrap();
    let (second_release, second_gate) = bounded(1);
    let (second_entered, second_started) = bounded(1);
    let queued = AtomicBool::new(false);
    let notify_work = work_tx.clone();
    let notify = Arc::new(move || {
        if rayon::current_thread_index().is_some() && !queued.swap(true, Ordering::AcqRel) {
            let gate = second_gate.clone();
            let entered = second_entered.clone();
            rayon::spawn(move || { entered.send(()).unwrap(); gate.recv().unwrap(); });
        }
        let _ = notify_work.send(WorkSignal::NucleoNotify);
    });
    let mut nucleo = Nucleo::new_with_thread_pool(Config::DEFAULT.match_paths(), notify, pool, /*columns*/ 1);
    let injector = nucleo.injector();
    for name in ["apple.txt", "apple-b.txt"] {
        injector.push(IndexedEntry {
            full_path: Arc::from(root.path().join(name).to_str().unwrap()),
            match_type: MatchType::File,
        }, |_, columns| columns[0] = Utf32String::from(name));
    }
    drop(injector);
    nucleo.pattern.reparse(/*column*/ 0, "apple", CaseMatching::Ignore, Normalization::Smart, /*append*/ false);
    assert!(nucleo.tick(/*timeout*/ 0).running);
    first_release.send(()).unwrap();
    // The apple worker has completed, but its changed snapshot is not consumed.
    // The only pool thread is now blocked before it can compute apple-b.
    second_started.recv_timeout(Duration::from_secs(5)).unwrap();
    while work_rx.try_recv().is_ok() {}
    work_tx.send(WorkSignal::QueryUpdated { query: "apple-b".into(), query_id: 2 }).unwrap();
    let worker_inner = inner.clone();
    let matcher = thread::spawn(move || {
        let result = matcher_worker(worker_inner, work_rx, &mut nucleo);
        nucleo.shutdown();
        result
    });
    let stale = observed.recv_timeout(Duration::from_millis(100));
    second_release.send(()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let final_snapshot = loop {
        let snapshot = observed.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())).unwrap();
        if snapshot.query_id == 2 { break snapshot; }
    };
    inner.shutdown.store(true, Ordering::Release);
    work_tx.send(WorkSignal::Shutdown).unwrap();
    matcher.join().unwrap().unwrap();
    assert_eq!(pool_threads.join(), Vec::<String>::new());
    assert!(stale.is_err(), "old apple matches must not be relabeled as apple-b");
    assert_eq!(final_snapshot.query, "apple-b");
    assert_eq!(final_snapshot.matches.iter().map(|item| item.path.clone()).collect::<Vec<_>>(), vec![std::path::PathBuf::from("apple-b.txt")]);
}
