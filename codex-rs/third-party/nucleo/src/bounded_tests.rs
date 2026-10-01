use super::*;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

fn with_pool(threads: usize, action: impl FnOnce(ThreadPool)) {
    let exited = Arc::new(AtomicUsize::new(0));
    let panicked = Arc::new(AtomicBool::new(false));
    thread::scope(|scope| {
        let failed = panicked.clone();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .panic_handler(move |_| {
                failed.store(true, Ordering::Release);
            })
            .spawn_handler(|worker| {
                let exited = exited.clone();
                thread::Builder::new().spawn_scoped(scope, move || {
                    worker.run();
                    exited.fetch_add(1, Ordering::Release);
                })?;
                Ok(())
            })
            .build()
            .unwrap();
        action(pool);
    });
    // The scope joined actual OS threads before examining the exit count.
    assert_eq!(exited.load(Ordering::Acquire), threads);
    assert!(
        !panicked.load(Ordering::Acquire),
        "bounded matcher worker panicked"
    );
}

#[test]
fn bounded_matching_preserves_native_results_without_growing_candidate_buffers() {
    with_pool(2, |legacy_pool| {
        with_pool(2, |bounded_pool| {
            let plan = IndexAllocationPlan::<String>::new(
                NonZeroUsize::new(97).unwrap(),
                NonZeroUsize::new(2).unwrap(),
            )
            .unwrap();
            let mut bounded = BoundedNucleo::try_new_with_thread_pool(
                Config::DEFAULT.match_paths(),
                Arc::new(|| {}),
                bounded_pool,
                plan,
            )
            .unwrap();
            let mut legacy = Nucleo::new_with_thread_pool(
                Config::DEFAULT.match_paths(),
                Arc::new(|| {}),
                legacy_pool,
                1,
            );
            let bounded_injector = bounded.injector();
            let legacy_injector = legacy.injector();
            for index in 0..97 {
                let name = format!("file-{index:03}.rs");
                bounded_injector
                    .try_push(name.clone(), name.as_str().into())
                    .unwrap();
                legacy_injector.push(name.clone(), |_, columns| columns[0] = name.as_str().into());
            }
            assert_eq!(
                bounded_injector.try_push("rejected.rs".into(), "rejected.rs".into()),
                Err(CapacityError::EntryLimit),
            );
            assert_eq!(bounded_injector.injected_items(), 97);
            drop(bounded_injector);
            drop(legacy_injector);
            let capacities = (
                bounded.inner.worker.lock().matches.capacity(),
                bounded.inner.snapshot.matches.capacity(),
            );
            let mut previous = "";
            for query in [
                "f", "fi", "file", "file-09", "FILE", "missing", "missing", "file",
            ] {
                let append = query.starts_with(previous);
                bounded.reparse(query, CaseMatching::Ignore, Normalization::Smart, append);
                legacy.pattern.reparse(
                    0,
                    query,
                    CaseMatching::Ignore,
                    Normalization::Smart,
                    append,
                );
                let deadline = Instant::now() + Duration::from_secs(5);
                while bounded.tick(10).running {
                    assert!(Instant::now() < deadline, "bounded matching did not settle");
                }
                while legacy.tick(10).running {
                    assert!(Instant::now() < deadline, "legacy matching did not settle");
                }
                let collect = |snapshot: &Snapshot<String>| {
                    snapshot
                        .matches()
                        .iter()
                        .map(|item| {
                            (
                                item.score,
                                snapshot.get_item(item.idx).unwrap().data.clone(),
                            )
                        })
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    collect(bounded.snapshot()),
                    collect(legacy.snapshot()),
                    "query={query}"
                );
                assert_eq!(bounded.snapshot().item_count(), 97);
                assert_eq!(
                    (
                        bounded.inner.worker.lock().matches.capacity(),
                        bounded.inner.snapshot.matches.capacity()
                    ),
                    capacities,
                );
                previous = query;
            }
            bounded.shutdown();
            legacy.shutdown();
        });
    });
}

#[test]
fn construction_rejects_a_pool_that_differs_from_the_prepaid_plan() {
    let plan = IndexAllocationPlan::<()>::new(
        NonZeroUsize::new(32).unwrap(),
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    with_pool(2, |pool| {
        let result =
            BoundedNucleo::try_new_with_thread_pool(Config::DEFAULT, Arc::new(|| {}), pool, plan);
        assert_eq!(result.err(), Some(CapacityError::PlanMismatch));
    });
}

#[test]
fn over_limit_plan_is_rejected_without_creating_a_pool_or_arena() {
    let result = IndexAllocationPlan::<()>::new(
        NonZeroUsize::new(u32::MAX as usize).unwrap(),
        NonZeroUsize::new(1).unwrap(),
    );
    assert_eq!(result.err(), Some(CapacityError::InvalidCapacity));
}

#[test]
fn notification_panic_keeps_fully_initialized_payload_owned_until_index_drop() {
    struct Payload(Arc<AtomicUsize>);
    impl Drop for Payload {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::AcqRel);
        }
    }
    let dropped = Arc::new(AtomicUsize::new(0));
    let plan = IndexAllocationPlan::<Payload>::new(
        NonZeroUsize::new(1).unwrap(),
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    with_pool(1, |pool| {
        let bounded = BoundedNucleo::try_new_with_thread_pool(
            Config::DEFAULT,
            Arc::new(|| panic!("injected notification panic")),
            pool,
            plan,
        )
        .unwrap();
        let injector = bounded.injector();
        assert!(catch_unwind(AssertUnwindSafe(|| {
            let _ = injector.try_push(Payload(dropped.clone()), "prepared".into());
        }))
        .is_err());
        assert_eq!(
            (injector.injected_items(), dropped.load(Ordering::Acquire)),
            (1, 0)
        );
        drop(injector);
        bounded.shutdown();
        assert_eq!(dropped.load(Ordering::Acquire), 1);
    });
}

#[test]
fn concurrent_injection_and_superseded_queries_keep_the_prepaid_index_intact() {
    with_pool(2, |pool| {
        let plan = IndexAllocationPlan::<String>::new(
            NonZeroUsize::new(2048).unwrap(),
            NonZeroUsize::new(2).unwrap(),
        )
        .unwrap();
        let mut bounded = BoundedNucleo::try_new_with_thread_pool(
            Config::DEFAULT.match_paths(),
            Arc::new(|| {}),
            pool,
            plan,
        )
        .unwrap();
        let before = (
            bounded.inner.worker.lock().matches.capacity(),
            bounded.inner.snapshot.matches.capacity(),
        );
        let start = std::sync::Barrier::new(3);
        thread::scope(|scope| {
            let workers: Vec<_> = (0..2)
                .map(|worker| {
                    let injector = bounded.injector();
                    let start = &start;
                    scope.spawn(move || {
                        start.wait();
                        for entry in 0..1024 {
                            let name = format!("file-{worker}-{entry:04}.txt");
                            let column = Utf32String::from(name.as_str());
                            injector.try_push(name, column).unwrap();
                        }
                    })
                })
                .collect();
            start.wait();
            for query in ["file", "missing", "fi", "file-0", "other", "file"]
                .into_iter()
                .cycle()
                .take(60)
            {
                // Reparse supersedes a running matcher while producers may have
                // reserved or published new items; ticks retain in-flight items.
                bounded.reparse(query, CaseMatching::Ignore, Normalization::Smart, false);
                bounded.tick(0);
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        bounded.reparse("file", CaseMatching::Ignore, Normalization::Smart, false);
        let deadline = Instant::now() + Duration::from_secs(5);
        while bounded.tick(10).running {
            assert!(Instant::now() < deadline);
        }
        let mut actual: Vec<_> = bounded
            .snapshot()
            .matched_items(..)
            .map(|item| item.data.clone())
            .collect();
        actual.sort();
        let expected: Vec<_> = (0..2)
            .flat_map(|worker| (0..1024).map(move |entry| format!("file-{worker}-{entry:04}.txt")))
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(
            (
                bounded.inner.worker.lock().matches.capacity(),
                bounded.inner.snapshot.matches.capacity()
            ),
            before,
        );
        assert_eq!(bounded.snapshot().item_count(), 2048);
        bounded.shutdown();
    });
}
