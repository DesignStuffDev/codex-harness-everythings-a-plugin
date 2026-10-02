use super::ThreadShutdownResult;
use super::wait_for_shutdown_result;
use codex_protocol::error::CodexErr;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tokio::time::Instant;

#[tokio::test]
async fn distinguishes_clean_shutdown_from_submission_failure() {
    assert!(matches!(
        wait_for_shutdown_result(async { Ok(()) }).await,
        ThreadShutdownResult::Complete
    ));
    assert!(matches!(
        wait_for_shutdown_result(async { Err(CodexErr::InternalAgentDied) }).await,
        ThreadShutdownResult::SubmitFailed
    ));
}

#[tokio::test(start_paused = true)]
async fn observes_failure_after_shutdown_has_started() {
    let started = Instant::now();
    let result = wait_for_shutdown_result(async {
        tokio::time::sleep(Duration::from_secs(/*secs*/ 1)).await;
        Err(CodexErr::ShutdownFailed)
    })
    .await;

    assert!(matches!(result, ThreadShutdownResult::ShutdownFailed));
    assert_eq!(started.elapsed(), Duration::from_secs(/*secs*/ 1));
}

#[tokio::test(start_paused = true)]
async fn preserves_timeout_as_a_separate_incomplete_outcome() {
    let started = Instant::now();
    let result = wait_for_shutdown_result(std::future::pending()).await;

    assert!(matches!(result, ThreadShutdownResult::TimedOut));
    assert_eq!(started.elapsed(), Duration::from_secs(/*secs*/ 10));
}
