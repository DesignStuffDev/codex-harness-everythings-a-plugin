//! Exercise paired admission through a real, independently running process.

use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::time::timeout;

use super::Fixture;
use crate::session::CALL_SLOTS;

fn assert_reaped(fixture: &Fixture) {
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("shutdown")).unwrap(),
        "graceful"
    );
    #[cfg(unix)]
    {
        let pid: libc::pid_t = std::fs::read_to_string(fixture.binding.state_dir.join("pid"))
            .unwrap()
            .parse()
            .unwrap();
        // SAFETY: signal 0 only probes this fixture's exact child process.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH),
            "fixture child {pid} must be absent, including zombies"
        );
    }
}

#[tokio::test]
async fn immediately_dropped_cleanup_follows_the_whole_paired_request() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let params = json!({"lease":"ordered", "body":"x".repeat(2 * 1024 * 1024)});
    let (reply, cleanup) = session
        .start_with_cleanup(
            "prepare",
            params.clone(),
            "release",
            json!({"lease":"ordered"}),
        )
        .await
        .unwrap();
    drop(cleanup);
    assert_eq!(reply.wait().await.unwrap(), params);
    session.close().await.unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("wire_accepted")).unwrap(),
        "prepare\nrelease\n"
    );
    assert_reaped(&fixture);
}

#[tokio::test]
async fn rejected_start_keeps_cleanup_observable_until_owned_worker_is_joined() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let (reply, cleanup) = session
        .start_with_cleanup(
            "prepare",
            json!({"lease":"rejected", "owned_worker":true, "fail":true}),
            "release",
            json!({"lease":"rejected"}),
        )
        .await
        .unwrap();
    assert_eq!(
        reply.wait().await.unwrap_err().to_string(),
        "prepare rejected"
    );
    assert!(!fixture.binding.state_dir.join("released").exists());
    let mut releasing = tokio::spawn(cleanup.release());
    fixture.wait_for("release_started", /*count*/ 1).await;
    assert!(
        timeout(Duration::from_millis(30), &mut releasing)
            .await
            .is_err()
    );
    assert!(!fixture.binding.state_dir.join("worker_finished").exists());
    fixture.open_gate("join_gate");
    releasing.await.unwrap().unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("worker_finished")).unwrap(),
        "rejected"
    );
    session.close().await.unwrap();
    assert_reaped(&fixture);
}

#[tokio::test]
async fn abandoned_response_and_release_waiters_do_not_cancel_or_replay_work() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let (reply, cleanup) = session
        .start_with_cleanup(
            "prepare",
            json!({"lease":"abandoned", "owned_worker":true, "wait":true}),
            "release",
            json!({"lease":"abandoned"}),
        )
        .await
        .unwrap();
    fixture.wait_for("prepare_started", /*count*/ 1).await;
    drop(reply);
    let releasing = tokio::spawn(cleanup.release());
    fixture.wait_for("release_started", /*count*/ 1).await;
    releasing.abort();
    assert!(releasing.await.unwrap_err().is_cancelled());
    let closer = session.clone();
    let mut closing = tokio::spawn(async move { closer.close().await });
    assert!(
        timeout(Duration::from_millis(30), &mut closing)
            .await
            .is_err()
    );
    fixture.open_gate("prepare_gate");
    fixture.open_gate("join_gate");
    closing.await.unwrap().unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("accepted")).unwrap(),
        "prepare\nrelease\n"
    );
    assert_reaped(&fixture);
}

#[tokio::test]
async fn response_timeout_keeps_cleanup_available_and_release_failure_is_not_success() {
    let mut fixture = Fixture::new();
    fixture.binding.timeout_ms = 500;
    let session = fixture.binding.connect().await.unwrap();
    let (reply, cleanup) = session
        .start_with_cleanup(
            "prepare",
            json!({"lease":"timeout", "owned_worker":true, "wait":true}),
            "release",
            json!({"lease":"timeout", "fail":true}),
        )
        .await
        .unwrap();
    let error = reply.wait().await.unwrap_err();
    assert!(format!("{error:#}").contains("accepted operation outcome is unknown"));
    assert_eq!(
        cleanup.release().await.unwrap_err().to_string(),
        "release failed"
    );
    assert!(!fixture.binding.state_dir.join("released").exists());
    assert!(!fixture.binding.state_dir.join("worker_finished").exists());
    fixture.open_gate("prepare_gate");
    fixture.open_gate("join_gate");
    session.close().await.unwrap();
    assert_reaped(&fixture);
}

#[tokio::test]
async fn reserved_release_bypasses_full_call_capacity_and_shutdown_drains_abandoned_start() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let (reply, cleanup) = session
        .start_with_cleanup("wait", json!({}), "release", json!({"lease":"full"}))
        .await
        .unwrap();
    drop(reply);
    let mut calls = Vec::new();
    for _ in 1..CALL_SLOTS {
        let caller = session.clone();
        calls.push(tokio::spawn(
            async move { caller.call("wait", json!({})).await },
        ));
    }
    fixture.wait_for("accepted", CALL_SLOTS).await;
    timeout(Duration::from_secs(5), cleanup.release())
        .await
        .unwrap()
        .unwrap();
    fixture.wait_for("released", /*count*/ 1).await;
    let closer = session.clone();
    let mut closing = tokio::spawn(async move { closer.close().await });
    assert!(
        timeout(Duration::from_millis(30), &mut closing)
            .await
            .is_err()
    );
    fixture.open_gate("gate");
    for call in calls {
        call.await.unwrap().unwrap();
    }
    closing.await.unwrap().unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("committed"))
            .unwrap()
            .lines()
            .count(),
        CALL_SLOTS
    );
    assert_reaped(&fixture);
}

#[tokio::test]
async fn cancelling_before_regular_admission_sends_neither_start_nor_release() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let mut calls = Vec::new();
    for _ in 0..CALL_SLOTS {
        let caller = session.clone();
        calls.push(tokio::spawn(
            async move { caller.call("wait", json!({})).await },
        ));
    }
    fixture.wait_for("accepted", CALL_SLOTS).await;
    assert!(
        timeout(
            Duration::from_millis(30),
            session.start_with_cleanup(
                "prepare",
                json!({"lease":"never"}),
                "release",
                json!({"lease":"never"}),
            ),
        )
        .await
        .is_err()
    );
    fixture.open_gate("gate");
    for call in calls {
        call.await.unwrap().unwrap();
    }
    session.close().await.unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("accepted")).unwrap(),
        "wait\n".repeat(CALL_SLOTS)
    );
    assert_reaped(&fixture);
}

#[tokio::test]
async fn full_cleanup_capacity_times_out_before_admission_and_recovers_after_release() {
    let mut fixture = Fixture::new();
    fixture.binding.timeout_ms = 500;
    let session = fixture.binding.connect().await.unwrap();
    let mut guards = Vec::new();
    for lease in 0..CALL_SLOTS {
        let (reply, cleanup) = session
            .start_with_cleanup(
                "prepare",
                json!({"lease":lease}),
                "release",
                json!({"lease":lease}),
            )
            .await
            .unwrap();
        reply.wait().await.unwrap();
        guards.push(cleanup);
    }
    let error = session
        .start_with_cleanup(
            "prepare",
            json!({"lease":"never"}),
            "release",
            json!({"lease":"never"}),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("no request was submitted"));
    for cleanup in guards {
        cleanup.release().await.unwrap();
    }
    let (reply, cleanup) = session
        .start_with_cleanup(
            "prepare",
            json!({"lease":"recovered"}),
            "release",
            json!({"lease":"recovered"}),
        )
        .await
        .unwrap();
    reply.wait().await.unwrap();
    cleanup.release().await.unwrap();
    session.close().await.unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("accepted")).unwrap(),
        format!(
            "{}{}prepare\nrelease\n",
            "prepare\n".repeat(CALL_SLOTS),
            "release\n".repeat(CALL_SLOTS)
        )
    );
    assert_reaped(&fixture);
}
