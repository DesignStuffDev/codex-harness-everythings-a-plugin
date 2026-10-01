#![allow(clippy::unwrap_used)]

use super::*;
use pretty_assertions::assert_eq;
use std::time::Duration;

#[test]
fn query_flood_retains_only_the_latest_query_and_all_lifecycle_flags() {
    let (sender, receiver) = WorkSender::channel();
    sender.send(WorkSignal::WalkComplete).unwrap();
    sender.send(WorkSignal::Shutdown).unwrap();
    for query_id in 1..=10_000 {
        sender.send(WorkSignal::NucleoNotify).unwrap();
        sender
            .send(WorkSignal::QueryUpdated {
                query: query_id.to_string(),
                query_id,
            })
            .unwrap();
    }
    assert_eq!(receiver.len(), 1);
    receiver.recv().unwrap();
    assert_eq!(
        sender.take(),
        PendingWork {
            query: Some(("10000".into(), 10_000)),
            notified: true,
            walk_complete: true,
            shutdown: true,
        }
    );
    assert_eq!(sender.take(), PendingWork::default());
}

#[test]
fn admission_between_receiving_wake_and_taking_slot_still_has_a_wake() {
    let (sender, receiver) = WorkSender::channel();
    sender.send(WorkSignal::NucleoNotify).unwrap();
    receiver.recv().unwrap();
    sender.send(WorkSignal::WalkComplete).unwrap();
    assert_eq!(
        sender.take(),
        PendingWork {
            notified: true,
            walk_complete: true,
            ..PendingWork::default()
        }
    );
    // This redundant wake is harmless; work arriving after take must not be
    // lost merely because that wake is already queued.
    sender.send(WorkSignal::Shutdown).unwrap();
    receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        sender.take(),
        PendingWork {
            shutdown: true,
            ..PendingWork::default()
        }
    );
}

#[test]
fn admission_reports_a_stopped_matcher() {
    let (sender, receiver) = WorkSender::channel();
    drop(receiver);
    assert!(sender.send(WorkSignal::NucleoNotify).is_err());
}
