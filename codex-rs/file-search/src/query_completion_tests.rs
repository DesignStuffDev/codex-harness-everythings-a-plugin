#![allow(clippy::expect_used, clippy::unwrap_used)]

use crate::FileSearchOptions;
use crate::FileSearchSnapshot;
use crate::SessionReporter;
use crate::create_session;
use pretty_assertions::assert_eq;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

enum Observation {
    Snapshot(FileSearchSnapshot),
    Complete(u64),
}

struct Reporter(crossbeam_channel::Sender<Observation>);

impl SessionReporter for Reporter {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        self.0
            .send(Observation::Snapshot(snapshot.clone()))
            .unwrap();
    }
    fn on_complete(&self) {
        panic!("matcher must report tagged completion");
    }
    fn on_complete_tagged(&self, query_id: u64) {
        self.0.send(Observation::Complete(query_id)).unwrap();
    }
}

#[test]
fn each_settled_generation_publishes_its_own_completed_snapshot_before_idle() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("needle.txt"), "a").unwrap();
    std::fs::write(root.path().join("other.txt"), "b").unwrap();
    let (updates, observed) = crossbeam_channel::unbounded();
    let session = create_session(
        vec![root.path().into()],
        FileSearchOptions {
            compute_indices: true,
            ..FileSearchOptions::default()
        },
        Arc::new(Reporter(updates)),
        /*cancel_flag*/ None,
    )
    .unwrap();
    let mut canonical_needle = None;
    let mut canonical_missing = None;
    for (index, (query, expected_path)) in [
        ("needle", Some("needle.txt")),
        ("needle", Some("needle.txt")),
        ("NEEDLE", Some("needle.txt")),
        ("other", Some("other.txt")),
        ("needle", Some("needle.txt")),
        ("missing-a", None),
        ("missing-b", None),
        ("missing-b", None),
    ]
    .into_iter()
    .enumerate()
    {
        let query_id = index as u64 + 1;
        session.update_query_tagged(query, query_id).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut latest = None;
        loop {
            match observed
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("admitted query must settle with a current snapshot")
            {
                Observation::Snapshot(snapshot) if snapshot.query_id == query_id => {
                    assert_eq!(snapshot.query, query);
                    for entry in &snapshot.matches {
                        assert_eq!(Some(entry.path.as_path()), expected_path.map(Path::new));
                    }
                    latest = Some(snapshot);
                }
                Observation::Complete(id) if id == query_id => break,
                Observation::Snapshot(_) | Observation::Complete(_) => {}
            }
        }
        let mut snapshot = latest.expect("completion must follow this generation's snapshot");
        assert!(snapshot.walk_complete);
        assert_eq!(
            snapshot.total_match_count,
            usize::from(expected_path.is_some())
        );
        assert_eq!(snapshot.matches.len(), snapshot.total_match_count);
        // Equal, normalized-equivalent and a/b/a generations must preserve the
        // full settled results while acquiring a genuinely fresh query identity.
        snapshot.query_id = 0;
        snapshot.query.clear();
        let canonical = match expected_path {
            Some("needle.txt") => Some(&mut canonical_needle),
            None => Some(&mut canonical_missing),
            Some(_) => None,
        };
        if let Some(canonical) = canonical {
            if let Some(previous) = canonical.as_ref() {
                assert_eq!(&snapshot, previous);
            } else {
                *canonical = Some(snapshot);
            }
        }
    }
    session.close().unwrap();
}
