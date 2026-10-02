use super::CodexErr;
use super::CodexErrKind;
use super::CodexErrorDetails;
use super::RetryAfter;
use crate::protocol::CodexErrorInfo;
use pretty_assertions::assert_eq;
use std::time::Duration;

#[test]
fn shutdown_failure_is_terminal_even_with_retry_advice() -> std::io::Result<()> {
    let advice = RetryAfter::from_delay(Duration::from_secs(1))
        .ok_or_else(|| std::io::Error::other("retry deadline unavailable"))?;
    let error = CodexErr::ShutdownFailed.with_retry_after(advice);

    assert!(matches!(error.details(), CodexErrorDetails::ShutdownFailed));
    assert!(matches!(CodexErrKind::from(&error), CodexErrKind::ShutdownFailed));
    assert_eq!(error.retry_delay(/*retry_count*/ 1), None);
    assert_eq!(error.retry_delay(/*retry_count*/ 3), None);
    Ok(())
}

#[test]
fn shutdown_failure_reports_a_fixed_internal_error() {
    let event = CodexErr::ShutdownFailed.to_error_event(/*message_prefix*/ None);

    assert_eq!(event.message, "session shutdown did not complete cleanly");
    assert_eq!(event.codex_error_info, Some(CodexErrorInfo::InternalServerError));
}
