//! Exercise output limits through real native sessions and joined cleanup.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::NativeOutputLimits;
use crate::FileMatch;
use crate::FileSearchOptions;
use crate::FileSearchSession;
use crate::FileSearchSnapshot;
use crate::NativePolicy;
use crate::SessionReporter;
use crate::create_session_with_receipt;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use crossbeam_channel::Receiver;
use pretty_assertions::assert_eq;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const WAIT: Duration = Duration::from_secs(5);

struct SnapshotObservation {
    snapshot: FileSearchSnapshot,
    charged_bytes: usize,
    indices_capacity: usize,
}

enum Observation {
    Snapshot(SnapshotObservation),
    Idle(u64),
    Failed(SearchError),
}

struct Probe(crossbeam_channel::Sender<Observation>);

impl SessionReporter for Probe {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        // Observe original native capacities before cloning for test assertions.
        let mut bytes = snapshot.query.capacity()
            + snapshot.matches.capacity() * std::mem::size_of::<FileMatch>();
        let mut indices_capacity = 0;
        for matched in &snapshot.matches {
            bytes += matched.path.capacity() + matched.root.capacity();
            if let Some(indices) = &matched.indices {
                indices_capacity += indices.capacity();
                bytes += indices.capacity() * std::mem::size_of::<u32>();
            }
        }
        let _ = self.0.send(Observation::Snapshot(SnapshotObservation {
            snapshot: snapshot.clone(),
            charged_bytes: bytes,
            indices_capacity,
        }));
    }
    fn on_complete(&self) {
        panic!("native session must use tagged completion");
    }
    fn on_complete_tagged(&self, query_id: u64) {
        let _ = self.0.send(Observation::Idle(query_id));
    }
    fn on_error(&self, error: &SearchError) {
        let _ = self.0.send(Observation::Failed(error.clone()));
    }
}

fn options() -> FileSearchOptions {
    FileSearchOptions {
        threads: NonZeroUsize::new(1).unwrap(),
        compute_indices: true,
        respect_gitignore: false,
        ..FileSearchOptions::default()
    }
}

fn start(root: &Path, output: NativeOutputLimits) -> (FileSearchSession, Receiver<Observation>) {
    let (sender, observed) = crossbeam_channel::unbounded();
    let session = create_session_with_receipt(
        vec![root.into()],
        options(),
        Arc::new(Probe(sender)),
        /*cancel_flag*/ None,
        NativePolicy::Backend {
            budget: SearchBudget {
                max_index_entries: NonZeroUsize::new(32).unwrap(),
                max_index_bytes: NonZeroUsize::new(8 * 1024 * 1024).unwrap(),
                max_worker_threads: NonZeroUsize::new(4).unwrap(),
            },
            output,
        },
    )
    .unwrap();
    (session, observed)
}

fn close(session: FileSearchSession) -> SearchCloseOutcome {
    let (sender, observed) = crossbeam_channel::bounded(1);
    let closer = thread::spawn(move || {
        let _ = sender.send(session.close_outcome());
    });
    let outcome = observed
        .recv_timeout(WAIT)
        .expect("native output close must join");
    closer.join().unwrap();
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    outcome
}

fn settled(observed: &Receiver<Observation>, id: u64, limit: usize) -> SnapshotObservation {
    let deadline = Instant::now() + WAIT;
    let mut latest = None;
    loop {
        match observed
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("native query must settle")
        {
            Observation::Snapshot(value) => {
                assert!(value.charged_bytes <= limit);
                if value.snapshot.query_id == id {
                    latest = Some(value);
                }
            }
            Observation::Idle(query_id) if query_id == id => {
                let value = latest.expect("idle requires the matching output snapshot");
                assert!(value.snapshot.walk_complete);
                return value;
            }
            Observation::Idle(_) => {}
            Observation::Failed(error) => panic!("unexpected output failure: {error}"),
        }
    }
}

#[test]
fn output_exhaustion_fails_without_idle_and_joins_index_ownership() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("needle.txt"), "real traversal input").unwrap();
    let (session, observed) = start(
        root.path(),
        NativeOutputLimits {
            max_query_bytes: NonZeroUsize::new(1024).unwrap(),
            max_snapshot_bytes: NonZeroUsize::new(1).unwrap(),
        },
    );
    let ledger = Arc::downgrade(session.inner.budget.as_ref().unwrap());
    session
        .finished
        .recv_timeout(WAIT)
        .expect("oversized native output must terminate");
    let outcome = close(session);
    assert!(ledger.upgrade().is_none());
    let error = outcome.operation.unwrap_err();
    assert_eq!(error.kind(), SearchErrorKind::ResourceExhausted);
    let mut failures = Vec::new();
    for event in observed.try_iter() {
        match event {
            Observation::Snapshot(value) => {
                assert!(value.charged_bytes <= 1);
                assert!(!value.snapshot.walk_complete);
            }
            Observation::Idle(_) => panic!("oversized native output reported idle"),
            Observation::Failed(error) => failures.push(error),
        }
    }
    assert_eq!(failures, vec![error]);
}

#[test]
fn query_byte_rejection_does_not_advance_identity_or_poison_the_session() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("中.txt"), "real Unicode fixture").unwrap();
    let (session, observed) = start(
        root.path(),
        NativeOutputLimits {
            max_query_bytes: NonZeroUsize::new(3).unwrap(),
            max_snapshot_bytes: NonZeroUsize::new(1024 * 1024).unwrap(),
        },
    );
    let rejected = session.update_query_tagged("😃", 1).unwrap_err();
    assert_eq!(
        rejected.downcast_ref::<SearchError>().unwrap().kind(),
        SearchErrorKind::ResourceExhausted
    );
    session.update_query_tagged("中", 1).unwrap();
    let output = settled(&observed, 1, 1024 * 1024);
    assert_eq!(
        (
            output.snapshot.query.as_str(),
            output.snapshot.total_match_count
        ),
        ("中", 1)
    );
    assert_eq!(output.snapshot.matches[0].path, Path::new("中.txt"));
    assert_eq!(close(session).operation, Ok(()));
}

#[test]
fn output_preallocation_preserves_unicode_scores_paths_and_positive_atom_highlights() {
    let root = tempfile::tempdir().unwrap();
    for name in ["naïve.txt", "u\u{0308}nicode.txt", "中.txt"] {
        std::fs::write(root.path().join(name), "real Unicode fixture").unwrap();
    }
    #[cfg(unix)]
    std::fs::write(root.path().join("line\r\nbreak.txt"), "real CRLF fixture").unwrap();
    let limits = NativeOutputLimits {
        max_query_bytes: NonZeroUsize::new(1024).unwrap(),
        max_snapshot_bytes: NonZeroUsize::new(1024 * 1024).unwrap(),
    };
    let (session, observed) = start(root.path(), limits);
    let queries = ["naive", "unicode", "中", "naive naive !absent"]
        .into_iter()
        .chain(cfg!(unix).then_some("line break"));
    for (index, query) in queries.enumerate() {
        let id = index as u64 + 1;
        session.update_query_tagged(query, id).unwrap();
        let output = settled(&observed, id, limits.max_snapshot_bytes.get());
        assert_eq!(output.snapshot.total_match_count, 1, "query={query:?}");
        let expected = crate::run(
            query,
            vec![root.path().into()],
            options(),
            /*cancel_flag*/ None,
        )
        .unwrap();
        assert_eq!(
            (output.snapshot.total_match_count, &output.snapshot.matches),
            (expected.total_match_count, &expected.matches),
            "query={query:?}"
        );
        if query == "naive naive !absent" {
            assert_eq!(output.indices_capacity, 10);
            assert_eq!(
                output.snapshot.matches[0].indices.as_ref().unwrap().len(),
                5
            );
        }
    }
    assert_eq!(close(session).operation, Ok(()));
}

#[test]
fn exact_native_path_copy_preserves_platform_encoding_and_capacity() {
    let mut sources = vec![std::ffi::OsString::from("naïve/中.txt")];
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        sources.push(std::ffi::OsString::from_vec(vec![b'a', 0xff, b'b']));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        sources.push(std::ffi::OsString::from_wide(&[0xd800, u16::from(b'a')]));
    }
    for source in sources {
        let copy = super::exact_path(Path::new(&source)).unwrap();
        assert_eq!(copy.as_os_str(), source.as_os_str());
        assert_eq!(copy.capacity(), source.as_encoded_bytes().len());
    }
}

#[test]
fn exact_input_copy_preserves_values_without_retaining_spare_capacity() {
    let mut source = String::with_capacity(64 * 1024);
    source.push_str("naïve/中/line\r\nbreak.txt");
    let mut path = std::ffi::OsString::with_capacity(64 * 1024);
    path.push(&source);
    let mut roots = Vec::with_capacity(4096);
    roots.push(std::path::PathBuf::from(path));
    let mut copy = super::exact_vec(roots.len()).unwrap();
    for root in &roots {
        copy.push(super::exact_path(root).unwrap());
    }
    let text = super::exact_string(&source).unwrap();
    assert_eq!((&text, &copy), (&source, &roots));
    assert!(source.capacity() > text.capacity());
    assert!(roots.capacity() > copy.capacity());
    assert!(roots[0].capacity() > copy[0].capacity());
    assert_eq!(text.capacity(), text.len());
    assert_eq!(copy.capacity(), copy.len());
    assert_eq!(
        copy[0].capacity(),
        copy[0].as_os_str().as_encoded_bytes().len()
    );
}
