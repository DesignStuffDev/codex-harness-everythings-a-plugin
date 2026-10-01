use codex_thread_store::RolloutMigrationCounts;
use pretty_assertions::assert_eq;

use super::*;

#[test]
fn coalesced_snapshots_render_cumulative_counts_without_double_counting() {
    let mut progress = MigrationProgress::new(RolloutMigrationMode::Apply, /*json*/ true);
    let initial = RolloutMigrationSnapshot {
        revision: 2,
        phase: RolloutMigrationPhase::Running,
        processed_paths: 1,
        total_paths: Some(1_200),
        counts: RolloutMigrationCounts {
            migrated: 1,
            ..Default::default()
        },
    };
    progress.update(initial);
    // Polling can skip hundreds of transitions and read the same snapshot twice.
    let coalesced = RolloutMigrationSnapshot {
        revision: 1_003,
        phase: RolloutMigrationPhase::Running,
        processed_paths: 1_000,
        total_paths: Some(1_200),
        counts: RolloutMigrationCounts {
            migrated: 800,
            already_paginated: 180,
            skipped_busy: 15,
            failed: 5,
            ..Default::default()
        },
    };
    progress.update(coalesced.clone());
    progress.update(coalesced.clone());
    assert_eq!(progress.counts.migrated, coalesced.counts.migrated);
    let line = progress.line(&coalesced);
    let (content, _) = line.rsplit_once("  •  ").unwrap();
    insta::assert_snapshot!(content, @"Migrating rollouts  1000/1200 (83%)  •  800 migrated  •  180 already paginated  •  15 skipped  •  5 failed");
}
