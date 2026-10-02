use super::CAPABILITY;
use super::UNCLEAN_OWNERSHIP;
use super::attach_cleanup_error;
use super::is_final_result;
use super::scope;
use crate::AppExitInfo;
use crate::LocalStateDbStartupError;
use codex_utils_process::process_shutdown::ProcessFinalCapability;
use pretty_assertions::assert_eq;
use std::io;
use std::path::PathBuf;

#[test]
fn state_db_startup_failure_keeps_process_admission_open_for_retry() {
    let result: io::Result<()> = Err(io::Error::other(LocalStateDbStartupError::new(
        PathBuf::from("state.sqlite"),
        "database disk image is malformed".to_string(),
    )));

    assert_eq!(is_final_result(&result), false);
}

#[test]
fn ordinary_startup_failure_is_a_final_result() {
    let result: io::Result<()> = Err(io::Error::other("app server connection failed"));

    assert_eq!(is_final_result(&result), true);
}

#[test]
fn successful_exit_is_a_final_result() {
    assert_eq!(is_final_result(&Ok(())), true);
}

#[tokio::test]
async fn library_scope_masks_an_enclosing_executable_capability() {
    scope(Some(ProcessFinalCapability::for_executable()), async {
        assert_eq!(CAPABILITY.with(Option::is_some), true);
        scope(/*capability*/ None, async {
            assert_eq!(CAPABILITY.with(Option::is_none), true);
        })
        .await;
        assert_eq!(CAPABILITY.with(Option::is_some), true);
    })
    .await;
}

#[test]
fn cleanup_failure_preserves_a_fatal_exit_message() {
    let error = attach_cleanup_error(
        Ok(AppExitInfo::fatal("fatal app failure")),
        UNCLEAN_OWNERSHIP.to_string(),
    );
    let report = anyhow::Error::new(error);

    insta::assert_snapshot!(
        "constructor_cleanup_preserves_fatal_exit",
        format!("{report:#}")
    );
    assert!(
        report
            .chain()
            .any(|cause| cause.to_string() == "fatal app failure")
    );
}

#[test]
fn cleanup_failure_preserves_an_operation_error_and_its_kind() {
    let error = attach_cleanup_error(
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "request denied",
        )),
        UNCLEAN_OWNERSHIP.to_string(),
    );

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    let report = anyhow::Error::new(error);
    insta::assert_snapshot!(
        "constructor_cleanup_preserves_operation",
        format!("{report:#}")
    );
    assert!(
        report
            .chain()
            .any(|cause| cause.to_string() == "request denied")
    );
}
