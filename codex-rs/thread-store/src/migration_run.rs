//! Observation and control of one backend-owned manual rollout migration.

use serde::Deserialize;
use serde::Serialize;

use crate::RolloutMigrationReport;
use crate::RolloutMigrationStatus;
use crate::ThreadStoreFuture;

/// Explicit lifecycle; neither zero progress nor an absent total implies completion.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RolloutMigrationPhase {
    #[default]
    Scanning,
    Running,
    Completed,
    Cancelled,
}

/// Cumulative outcomes allow observers to coalesce intermediate progress safely.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RolloutMigrationCounts {
    pub eligible: u64,
    pub migrated: u64,
    pub already_paginated: u64,
    pub skipped_empty: u64,
    pub skipped_busy: u64,
    pub failed: u64,
}

impl RolloutMigrationCounts {
    pub(crate) fn observe(&mut self, status: RolloutMigrationStatus) {
        let count = match status {
            RolloutMigrationStatus::Eligible => &mut self.eligible,
            RolloutMigrationStatus::Migrated => &mut self.migrated,
            RolloutMigrationStatus::AlreadyPaginated => &mut self.already_paginated,
            RolloutMigrationStatus::SkippedEmpty => &mut self.skipped_empty,
            RolloutMigrationStatus::SkippedBusy => &mut self.skipped_busy,
            RolloutMigrationStatus::Failed => &mut self.failed,
        };
        *count = count.saturating_add(1);
    }
}

/// One consistent progress observation. Revisions advance on observable transitions.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RolloutMigrationSnapshot {
    pub revision: u64,
    pub phase: RolloutMigrationPhase,
    pub processed_paths: u64,
    pub total_paths: Option<u64>,
    pub counts: RolloutMigrationCounts,
}

/// Owns observation/control, while the backend retains accepted work until cleanup.
///
/// Snapshot and report are immediate observations; report returns Conflict while
/// work is active. Cancel acknowledges a stop request, not durable completion.
/// Drop requests release without blocking. Close joins cleanup before succeeding;
/// abandoning its future must not cancel the backend's accepted mutation.
pub trait RolloutMigrationRun: Send + Sync {
    /// Reads cumulative progress without waiting for the next transition.
    fn snapshot(&self) -> ThreadStoreFuture<'_, RolloutMigrationSnapshot>;
    /// Returns the retained terminal result, or Conflict while the job is active.
    fn report(&self) -> ThreadStoreFuture<'_, RolloutMigrationReport>;
    /// Requests cooperative stopping; success does not promise durable completion.
    fn cancel(&self) -> ThreadStoreFuture<'_, ()>;
    /// Releases this observer after the backend has joined accepted work and cleanup.
    fn close(self: Box<Self>) -> ThreadStoreFuture<'static, ()>;
}
