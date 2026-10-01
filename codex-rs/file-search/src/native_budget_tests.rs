//! Real traversal and matching checks complement atomic admission tests.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::NativeBudget;
use crate::FileSearchOptions;
use crate::FileSearchSession;
use crate::FileSearchSnapshot;
use crate::IndexedEntry;
use crate::SessionReporter;
use crate::create_bounded_session;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::StartCleanup;
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use pretty_assertions::assert_eq;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const WAIT: Duration = Duration::from_secs(5);

fn options() -> FileSearchOptions {
    FileSearchOptions {
        threads: NonZeroUsize::new(1).unwrap(),
        respect_gitignore: false,
        ..FileSearchOptions::default()
    }
}

fn budget(entries: usize, bytes: usize) -> SearchBudget {
    SearchBudget {
        max_index_entries: NonZeroUsize::new(entries).unwrap(),
        max_index_bytes: NonZeroUsize::new(bytes).unwrap(),
        max_worker_threads: NonZeroUsize::new(4).unwrap(),
    }
}

fn fixed_bytes(options: &FileSearchOptions, entries: usize) -> usize {
    let plan = nucleo::IndexAllocationPlan::<IndexedEntry>::new(
        NonZeroUsize::new(entries).unwrap(),
        options.threads,
    )
    .unwrap();
    plan.charged_bytes()
        + if options.compute_indices {
            nucleo::Matcher::scratch_allocation_bytes()
        } else {
            0
        }
}

enum Observation {
    Snapshot(FileSearchSnapshot),
    Idle(u64),
    Failed(SearchError),
}

struct Probe(Sender<Observation>);

impl SessionReporter for Probe {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        let _ = self.0.send(Observation::Snapshot(snapshot.clone()));
    }

    fn on_complete(&self) {
        panic!("native search must use tagged completion");
    }

    fn on_complete_tagged(&self, query_id: u64) {
        let _ = self.0.send(Observation::Idle(query_id));
    }

    fn on_error(&self, error: &SearchError) {
        let _ = self.0.send(Observation::Failed(error.clone()));
    }
}

fn probe() -> (Arc<Probe>, Receiver<Observation>) {
    let (sender, receiver) = crossbeam_channel::unbounded();
    (Arc::new(Probe(sender)), receiver)
}

fn joined_close(session: FileSearchSession) -> SearchCloseOutcome {
    let (sender, receiver) = crossbeam_channel::bounded(1);
    let closer = thread::spawn(move || {
        let _ = sender.send(session.close_outcome());
    });
    let outcome = receiver
        .recv_timeout(WAIT)
        .expect("close must finish joining");
    closer.join().expect("close caller must not panic");
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    outcome
}

fn settled(observed: &Receiver<Observation>, query_id: u64) -> FileSearchSnapshot {
    let deadline = Instant::now() + WAIT;
    let mut latest = None;
    loop {
        match observed
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("admitted query must settle")
        {
            Observation::Snapshot(snapshot) if snapshot.query_id == query_id => {
                latest = Some(snapshot);
            }
            Observation::Idle(id) if id == query_id => {
                let snapshot = latest.expect("idle must follow a current query snapshot");
                assert!(snapshot.walk_complete);
                return snapshot;
            }
            Observation::Failed(error) => panic!("unexpected native failure: {error}"),
            Observation::Snapshot(_) | Observation::Idle(_) => {}
        }
    }
}

fn assert_exhaustion(session: FileSearchSession, observed: Receiver<Observation>) {
    let reservation = Arc::downgrade(session.inner.budget.as_ref().unwrap());
    session
        .finished
        .recv_timeout(WAIT)
        .expect("resource exhaustion must finish actual workers");
    let outcome = joined_close(session);
    assert!(
        reservation.upgrade().is_none(),
        "joined close must release the ledger"
    );
    let error = outcome
        .operation
        .expect_err("truncated traversal must fail");
    assert_eq!(error.kind(), SearchErrorKind::ResourceExhausted);
    let mut failures = Vec::new();
    for event in observed.try_iter() {
        match event {
            Observation::Snapshot(snapshot) => assert!(!snapshot.walk_complete),
            Observation::Idle(_) => panic!("exhausted index reported successful idle"),
            Observation::Failed(error) => failures.push(error),
        }
    }
    assert_eq!(failures, vec![error]);
}

#[test]
fn preflight_charges_fixed_storage_and_optional_highlighting_before_startup() {
    let root = tempfile::tempdir().unwrap();
    let mut options = options();
    for compute_indices in [false, true] {
        options.compute_indices = compute_indices;
        let floor = fixed_bytes(&options, 33);
        let rejected = NativeBudget::prepare(&options, budget(33, floor - 1));
        assert_eq!(
            rejected.err().unwrap().kind(),
            SearchErrorKind::ResourceExhausted
        );
        let (reporter, observed) = probe();
        let rejected = create_bounded_session(
            vec![root.path().into()],
            options.clone(),
            budget(33, floor - 1),
            reporter,
        )
        .err()
        .expect("fixed storage must be prepaid before startup");
        assert_eq!(rejected.cleanup, StartCleanup::NotAdmitted);
        assert!(observed.try_iter().next().is_none());
        let (_, accepted) = NativeBudget::prepare(&options, budget(33, floor)).unwrap();
        let state = accepted.state.lock().unwrap();
        assert_eq!(
            (state.entries, state.bytes, &state.failure),
            (0, floor, &None)
        );
    }
}

#[test]
fn unsupported_nucleo_limits_reject_before_allocating_the_index() {
    let mut options = options();
    options.threads = NonZeroUsize::new(rayon::max_num_threads() + 1).unwrap();
    let mut allocation = budget(32, usize::MAX);
    allocation.max_worker_threads = NonZeroUsize::new(usize::MAX).unwrap();
    assert_eq!(
        NativeBudget::prepare(&options, allocation)
            .err()
            .unwrap()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
    options.threads = NonZeroUsize::new(1).unwrap();
    allocation.max_index_entries = NonZeroUsize::new(u32::MAX as usize).unwrap();
    assert_eq!(
        NativeBudget::prepare(&options, allocation)
            .err()
            .unwrap()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
}

#[test]
fn worker_ceiling_and_overflow_reject_before_native_admission() {
    let root = tempfile::tempdir().unwrap();
    for threads in [2, usize::MAX] {
        let options = FileSearchOptions {
            threads: NonZeroUsize::new(threads).unwrap(),
            ..options()
        };
        let (reporter, observed) = probe();
        let result = create_bounded_session(
            vec![root.path().into()],
            options,
            budget(32, 8 * 1024 * 1024),
            reporter,
        );
        let error = result.err().expect("worker ceiling must reject startup");
        assert_eq!(error.operation.kind(), SearchErrorKind::ResourceExhausted);
        assert_eq!(error.cleanup, StartCleanup::NotAdmitted);
        assert!(observed.try_iter().next().is_none());
    }
}

#[test]
fn concurrent_reservations_charge_both_counters_without_partial_rejection() {
    let options = options();
    let floor = fixed_bytes(&options, 11);
    let (_, ledger) = NativeBudget::prepare(&options, budget(11, floor + 77)).unwrap();
    let successes = thread::scope(|scope| {
        #[expect(
            clippy::needless_collect,
            reason = "spawn every worker before joining so admission is concurrent"
        )]
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let ledger = ledger.clone();
                scope.spawn(move || (0..32).filter(|_| ledger.reserve(7).is_ok()).count())
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .sum::<usize>()
    });
    assert_eq!(successes, 11);
    let error = ledger.reserve(0).unwrap_err();
    let state = ledger.state.lock().unwrap();
    assert_eq!(
        (state.entries, state.bytes, state.failure.as_ref()),
        (11, floor + 77, Some(&error))
    );
    assert_eq!(error.kind(), SearchErrorKind::ResourceExhausted);
}

#[test]
fn byte_overflow_retains_first_failure_and_changes_neither_counter() {
    let options = options();
    let floor = fixed_bytes(&options, 4);
    let (_, ledger) = NativeBudget::prepare(&options, budget(4, floor + 1024)).unwrap();
    let first = ledger.reserve(usize::MAX).unwrap_err();
    assert_eq!(ledger.reserve(0), Err(first.clone()));
    let state = ledger.state.lock().unwrap();
    assert_eq!(
        (state.entries, state.bytes, state.failure.as_ref()),
        (0, floor, Some(&first))
    );
    assert_eq!(first.kind(), SearchErrorKind::ResourceExhausted);
}

#[test]
fn native_entry_exhaustion_joins_without_idle_then_reuses_budget_for_empty_tree() {
    let root = tempfile::tempdir().unwrap();
    let fixture = root.path().join("needle.txt");
    std::fs::write(&fixture, "real traversal input").unwrap();
    let options = options();
    let allocation = budget(1, 8 * 1024 * 1024);
    let (reporter, observed) = probe();
    let failed = create_bounded_session(
        vec![root.path().into()],
        options.clone(),
        allocation,
        reporter,
    )
    .unwrap();
    assert_exhaustion(failed, observed);

    std::fs::remove_file(fixture).unwrap();
    let (reporter, observed) = probe();
    let recovered =
        create_bounded_session(vec![root.path().into()], options, allocation, reporter).unwrap();
    recovered.update_query_tagged("missing", 1).unwrap();
    let snapshot = settled(&observed, 1);
    assert_eq!(
        (snapshot.scanned_file_count, snapshot.total_match_count),
        (1, 0)
    );
    assert!(snapshot.matches.is_empty());
    assert_eq!(joined_close(recovered).operation, Ok(()));
}

#[test]
fn native_byte_exhaustion_cannot_report_a_complete_partial_index() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("needle.txt"), "real traversal input").unwrap();
    let options = options();
    let allocation = budget(32, fixed_bytes(&options, 32));
    let (reporter, observed) = probe();
    let session =
        create_bounded_session(vec![root.path().into()], options, allocation, reporter).unwrap();
    assert_exhaustion(session, observed);
}

#[test]
fn bounded_unicode_search_preserves_full_legacy_matches_and_highlights() {
    let root = tempfile::tempdir().unwrap();
    let names = ["naïve.txt", "u\u{0308}nicode.txt", "中.txt"];
    for name in names {
        std::fs::write(root.path().join(name), "real Unicode filename").unwrap();
    }
    #[cfg(unix)]
    std::fs::write(root.path().join("line\r\nbreak.txt"), "real CRLF filename").unwrap();
    let options = FileSearchOptions {
        compute_indices: true,
        ..options()
    };
    let (reporter, observed) = probe();
    let session = create_bounded_session(
        vec![root.path().into()],
        options.clone(),
        budget(32, 8 * 1024 * 1024),
        reporter,
    )
    .unwrap();
    let queries = ["naive", "unicode", "中"]
        .into_iter()
        .chain(cfg!(unix).then_some("line"));
    for (index, query) in queries.enumerate() {
        let query_id = index as u64 + 1;
        session.update_query_tagged(query, query_id).unwrap();
        let actual = settled(&observed, query_id);
        assert_eq!(actual.total_match_count, 1, "query={query:?}");
        assert!(
            actual.matches[0]
                .indices
                .as_ref()
                .is_some_and(|indices| !indices.is_empty()),
            "real Unicode result must include native highlights"
        );
        let expected = crate::run(
            query,
            vec![root.path().into()],
            options.clone(),
            /*cancel_flag*/ None,
        )
        .unwrap();
        assert_eq!(
            (actual.total_match_count, actual.matches),
            (expected.total_match_count, expected.matches),
            "query={query:?}"
        );
    }
    assert_eq!(joined_close(session).operation, Ok(()));
}

// This is an allocation-floor diagnostic plus real native behavior check, not
// a benchmark or evidence that the entry ceiling was populated in production.
#[test]
fn consumer_candidate_profiles_prepay_actual_index_layout_and_preserve_native_matching() {
    const CEILING: usize = 128 * 1024 * 1024;
    let empty = tempfile::tempdir().unwrap();
    let fixture = tempfile::tempdir().unwrap();
    std::fs::write(fixture.path().join("alpha.txt"), "profile fixture").unwrap();
    for threads in [2_usize, 12] {
        let workers = threads
            .checked_mul(2)
            .and_then(|count| count.checked_add(2))
            .unwrap();
        for entries in [100_000, 250_000] {
            for compute_indices in [false, true] {
                let options = FileSearchOptions {
                    threads: NonZeroUsize::new(threads).unwrap(),
                    compute_indices,
                    ..options()
                };
                let floor = fixed_bytes(&options, entries);
                let mut allocation = budget(entries, floor - 1);
                allocation.max_worker_threads = NonZeroUsize::new(workers).unwrap();
                let (reporter, observed) = probe();
                let rejected = create_bounded_session(
                    vec![empty.path().into()],
                    options.clone(),
                    allocation,
                    reporter,
                )
                .err()
                .expect("one byte below the actual target-layout floor must reject");
                assert_eq!(rejected.cleanup, StartCleanup::NotAdmitted);
                assert_eq!(
                    rejected.operation.kind(),
                    SearchErrorKind::ResourceExhausted
                );
                assert!(observed.try_iter().next().is_none());

                allocation.max_index_bytes = NonZeroUsize::new(floor).unwrap();
                let (_, ledger) = NativeBudget::prepare(&options, allocation).unwrap();
                assert_eq!(ledger.state.lock().unwrap().bytes, floor);
                // The root directory is itself an indexed entry. Prepay its actual
                // path bytes and real empty UTF32 column plan in addition to floor.
                let root_payload = empty.path().to_str().unwrap().len()
                    + nucleo::Utf32String::allocation_plan("")
                        .unwrap()
                        .charged_bytes();
                allocation.max_index_bytes = NonZeroUsize::new(floor + root_payload).unwrap();
                let (reporter, observed) = probe();
                let session = create_bounded_session(
                    vec![empty.path().into()],
                    options.clone(),
                    allocation,
                    reporter,
                )
                .unwrap();
                session.update_query_tagged("absent", 1).unwrap();
                let snapshot = settled(&observed, 1);
                assert!(snapshot.matches.is_empty());
                assert_eq!(snapshot.total_match_count, 0);
                let reservation = session.inner.budget.as_ref().unwrap();
                let reserved = reservation.state.lock().unwrap();
                assert_eq!(
                    (reserved.entries, reserved.bytes),
                    (1, floor + root_payload)
                );
                drop(reserved);
                assert_eq!(
                    joined_close(session),
                    SearchCloseOutcome {
                        operation: Ok(()),
                        cleanup: CloseCleanup::Joined,
                    }
                );

                assert!(floor < CEILING, "candidate ceiling needs payload headroom");
                allocation.max_index_bytes = NonZeroUsize::new(CEILING).unwrap();
                let (reporter, observed) = probe();
                let session = create_bounded_session(
                    vec![fixture.path().into()],
                    options,
                    allocation,
                    reporter,
                )
                .unwrap();
                session.update_query_tagged("alpha", 1).unwrap();
                let snapshot = settled(&observed, 1);
                assert_eq!(snapshot.total_match_count, 1);
                assert_eq!(snapshot.matches.len(), 1);
                assert_eq!(snapshot.matches[0].path, std::path::Path::new("alpha.txt"));
                assert_eq!(
                    snapshot.matches[0].indices,
                    compute_indices.then(|| vec![0, 1, 2, 3, 4])
                );
                assert_eq!(
                    joined_close(session),
                    SearchCloseOutcome {
                        operation: Ok(()),
                        cleanup: CloseCleanup::Joined,
                    }
                );
                eprintln!(
                    "native_consumer_profile entries={entries} threads={threads} compute_indices={compute_indices} fixed_bytes={floor} empty_root_payload_bytes={root_payload} ceiling_bytes={CEILING} payload_headroom_bytes={} reserved_os_workers={workers} pointer_bits={}",
                    CEILING - floor,
                    usize::BITS,
                );
            }
        }
    }
}
