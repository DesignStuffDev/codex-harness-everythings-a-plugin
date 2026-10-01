use codex_protocol::ThreadId;
use codex_thread_store::RolloutMigrationFailureReason;
use codex_thread_store::RolloutMigrationMode;
use codex_thread_store::RolloutMigrationOptions;
use codex_thread_store::RolloutMigrationOutcome;
use codex_thread_store::RolloutMigrationReport;
use codex_thread_store::RolloutMigrationStatus;
use pretty_assertions::assert_eq;

use crate::StartMigrationRequest;
use crate::StorageRequest;
use crate::StorageResponse;

#[test]
fn migration_options_and_large_report_preserve_native_values() {
    for mode in [RolloutMigrationMode::DryRun, RolloutMigrationMode::Apply] {
        let options = RolloutMigrationOptions {
            mode,
            thread_ids: vec![ThreadId::new(), ThreadId::new()],
            max_mib_per_second: Some(u64::MAX),
        };
        let request = StorageRequest::StartMigration(StartMigrationRequest {
            contract_version: 1,
            lease_id: "wire".to_owned(),
            options: options.clone(),
        });
        let encoded = serde_json::to_vec(&request).unwrap();
        let StorageRequest::StartMigration(decoded) = serde_json::from_slice(&encoded).unwrap()
        else {
            panic!("migration request changed operation");
        };
        assert_eq!(decoded.options, options);
        assert_eq!(
            (decoded.contract_version, decoded.lease_id),
            (1, "wire".to_owned())
        );
    }
    let failures = [
        None,
        Some(RolloutMigrationFailureReason::MissingSqliteMetadata),
        Some(RolloutMigrationFailureReason::InvalidSessionMetadata),
        Some(RolloutMigrationFailureReason::RolloutReadFailed),
        Some(RolloutMigrationFailureReason::LegacyRolloutConversionFailed),
        Some(RolloutMigrationFailureReason::SqliteMaterializationFailed),
        Some(RolloutMigrationFailureReason::RolloutPublishFailed),
        Some(RolloutMigrationFailureReason::InterruptedMigrationRecoveryFailed),
        Some(RolloutMigrationFailureReason::Unknown),
    ];
    let statuses = [
        RolloutMigrationStatus::Eligible,
        RolloutMigrationStatus::Migrated,
        RolloutMigrationStatus::AlreadyPaginated,
        RolloutMigrationStatus::SkippedEmpty,
        RolloutMigrationStatus::SkippedBusy,
        RolloutMigrationStatus::Failed,
    ];
    let report = RolloutMigrationReport {
        outcomes: failures
            .into_iter()
            .enumerate()
            .map(|(index, failure_reason)| RolloutMigrationOutcome {
                thread_id: (index % 2 == 0).then(ThreadId::new),
                rollout_path: std::path::PathBuf::from(format!("rollout-{index}.jsonl")),
                status: statuses[index % statuses.len()],
                failure_reason,
                bytes_processed: u64::MAX - index as u64,
                message: (index != 0).then(|| "diagnostic".repeat(70_000)),
            })
            .collect(),
    };
    let encoded = serde_json::to_vec(&StorageResponse::MigrationReport(report.clone())).unwrap();
    assert!(encoded.len() > 4 * 1024 * 1024);
    let StorageResponse::MigrationReport(decoded) = serde_json::from_slice(&encoded).unwrap()
    else {
        panic!("migration response changed operation");
    };
    assert_eq!(decoded, report);
}

#[cfg(unix)]
#[test]
fn migration_report_preserves_non_utf8_path_bytes() {
    use std::os::unix::ffi::OsStringExt;
    let report = RolloutMigrationReport {
        outcomes: vec![RolloutMigrationOutcome {
            thread_id: None,
            rollout_path: std::ffi::OsString::from_vec(b"/session/\xff/rollout.jsonl".to_vec())
                .into(),
            status: RolloutMigrationStatus::Failed,
            failure_reason: Some(RolloutMigrationFailureReason::RolloutReadFailed),
            bytes_processed: 0,
            message: None,
        }],
    };
    let encoded = serde_json::to_vec(&StorageResponse::MigrationReport(report.clone())).unwrap();
    let StorageResponse::MigrationReport(decoded) = serde_json::from_slice(&encoded).unwrap()
    else {
        panic!("migration response changed operation");
    };
    assert_eq!(decoded, report);
}
