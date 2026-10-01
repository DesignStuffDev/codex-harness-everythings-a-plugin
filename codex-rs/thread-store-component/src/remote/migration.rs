//! Additive manual migration wire, preserving native report paths losslessly.

use codex_protocol::ThreadId;
use codex_thread_store::RolloutMigrationFailureReason;
use codex_thread_store::RolloutMigrationMode;
use codex_thread_store::RolloutMigrationOptions;
use codex_thread_store::RolloutMigrationOutcome;
use codex_thread_store::RolloutMigrationReport;
use codex_thread_store::RolloutMigrationStatus;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(remote = "RolloutMigrationMode", rename_all = "snake_case")]
enum Mode {
    DryRun,
    Apply,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "RolloutMigrationStatus", rename_all = "snake_case")]
enum Status {
    Eligible,
    Migrated,
    AlreadyPaginated,
    SkippedEmpty,
    SkippedBusy,
    Failed,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "RolloutMigrationFailureReason", rename_all = "snake_case")]
enum Failure {
    MissingSqliteMetadata,
    InvalidSessionMetadata,
    RolloutReadFailed,
    LegacyRolloutConversionFailed,
    SqliteMaterializationFailed,
    RolloutPublishFailed,
    InterruptedMigrationRecoveryFailed,
    Unknown,
}
remote_adapter!(failure, RolloutMigrationFailureReason, Failure, "Failure");

#[derive(Serialize, Deserialize)]
#[serde(remote = "RolloutMigrationOptions", deny_unknown_fields)]
struct Options {
    #[serde(with = "Mode")]
    mode: RolloutMigrationMode,
    thread_ids: Vec<ThreadId>,
    max_mib_per_second: Option<u64>,
}
remote_adapter!(options, RolloutMigrationOptions, Options, "Options");

#[derive(Serialize, Deserialize)]
#[serde(remote = "RolloutMigrationOutcome", deny_unknown_fields)]
struct Outcome {
    thread_id: Option<ThreadId>,
    #[serde(with = "codex_component_state_codec::native_path")]
    rollout_path: PathBuf,
    #[serde(with = "Status")]
    status: RolloutMigrationStatus,
    #[serde(with = "failure::option")]
    failure_reason: Option<RolloutMigrationFailureReason>,
    bytes_processed: u64,
    message: Option<String>,
}
remote_adapter!(outcome, RolloutMigrationOutcome, Outcome, "Outcome");

#[derive(Serialize, Deserialize)]
#[serde(remote = "RolloutMigrationReport", deny_unknown_fields)]
struct Report {
    #[serde(with = "outcome::vec")]
    outcomes: Vec<RolloutMigrationOutcome>,
}
remote_adapter!(report, RolloutMigrationReport, Report, "Report");

#[cfg(test)]
#[path = "migration_tests.rs"]
mod tests;
