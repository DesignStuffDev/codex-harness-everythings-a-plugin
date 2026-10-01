//! Logical size rejection must preserve ownership, not merely return an error.

use std::num::NonZeroU64;
use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::time::timeout;

use super::Fixture;
use crate::PayloadKind;
use crate::PayloadLimitExceeded;
use crate::SessionPayloadLimits;
use crate::session::CALL_SLOTS;

#[tokio::test]
async fn local_oversize_never_admits_and_reserved_cleanup_survives_full_ordinary_capacity() {
    let fixture = Fixture::new();
    let session = fixture
        .binding
        .connect_with_limits(SessionPayloadLimits {
            request_bytes: NonZeroU64::new(8),
            reply_bytes: NonZeroU64::new(64 * 1024),
        })
        .await
        .unwrap();
    let (reply, cleanup) = session
        .start_with_cleanup("wait", json!({}), "release", json!({"lease":"reserved"}))
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
    let error = timeout(
        Duration::from_secs(1),
        session.start_with_cleanup(
            "prepare",
            json!({"lease":"never-admitted"}),
            "release",
            json!({"lease":"never-admitted"}),
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(
        error.downcast_ref::<PayloadLimitExceeded>(),
        Some(&PayloadLimitExceeded {
            kind: PayloadKind::Request,
            limit_bytes: 8,
        })
    );
    assert_eq!(
        cleanup.release_reply().await.unwrap(),
        json!({"lease":"reserved"})
    );
    fixture.open_gate("gate");
    for call in calls {
        call.await.unwrap().unwrap();
    }
    assert_eq!(session.call("echo", json!(42)).await.unwrap(), json!(42));
    session.close().await.unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("wire_accepted")).unwrap(),
        format!("{}release\necho\n", "wait\n".repeat(CALL_SLOTS))
    );
}

#[tokio::test]
async fn cleanup_reply_retains_unconfirmed_receipt_and_operation_error() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let receipt = json!({
        "operation":{"Err":{"kind":"search_failed", "message":"retained failure"}},
        "cleanup":{"status":"unconfirmed", "error":{"kind":"transport_lost", "message":"receipt unavailable"}},
    });
    let (reply, cleanup) = session
        .start_with_cleanup(
            "prepare",
            json!({"lease":"receipt"}),
            "release",
            json!({"lease":"receipt", "reply":receipt}),
        )
        .await
        .unwrap();
    reply.wait().await.unwrap();
    assert_eq!(cleanup.release_reply().await.unwrap(), receipt);
    session.close().await.unwrap();
}

#[tokio::test]
async fn legacy_remote_error_text_cannot_forge_a_local_size_limit_receipt() {
    let fixture = Fixture::new();
    let session = fixture
        .binding
        .connect_with_limits(SessionPayloadLimits {
            request_bytes: NonZeroU64::new(1024),
            reply_bytes: NonZeroU64::new(64 * 1024),
        })
        .await
        .unwrap();
    let message = PayloadLimitExceeded {
        kind: PayloadKind::Reply,
        limit_bytes: 64 * 1024,
    }
    .to_string();
    let error = session
        .call("remote_error", json!({"message":message}))
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), message);
    assert!(error.downcast_ref::<PayloadLimitExceeded>().is_none());
    assert_eq!(
        session.call("echo", json!("healthy")).await.unwrap(),
        json!("healthy")
    );
    session.close().await.unwrap();
}

#[tokio::test]
async fn reply_cap_smaller_than_control_allowance_is_rejected_before_process_launch() {
    let fixture = Fixture::new();
    let error = fixture
        .binding
        .connect_with_limits(SessionPayloadLimits {
            request_bytes: None,
            reply_bytes: NonZeroU64::new(64 * 1024 - 1),
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("64 KiB"));
    assert!(!fixture.binding.state_dir.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn peer_oversized_declaration_is_terminal_reaped_and_preserves_typed_cause() {
    let fixture = Fixture::new();
    let session = fixture
        .binding
        .connect_with_limits(SessionPayloadLimits {
            request_bytes: NonZeroU64::new(256),
            reply_bytes: NonZeroU64::new(64 * 1024),
        })
        .await
        .unwrap();
    let (reply, cleanup) = session
        .start_with_cleanup(
            "oversized_reply",
            json!({"bytes":u64::MAX}),
            "release",
            json!({"lease":"lost"}),
        )
        .await
        .unwrap();
    let error = reply.wait().await.unwrap_err();
    let expected = PayloadLimitExceeded {
        kind: PayloadKind::Reply,
        limit_bytes: 64 * 1024,
    };
    assert_eq!(
        error.downcast_ref::<PayloadLimitExceeded>(),
        Some(&expected)
    );
    assert!(format!("{error:#}").contains("outcomes may be unknown"));
    let cleanup_error = cleanup.release_reply().await.unwrap_err();
    assert_eq!(
        cleanup_error.downcast_ref::<PayloadLimitExceeded>(),
        Some(&expected)
    );
    let close_error = session.close().await.unwrap_err();
    assert_eq!(
        close_error.downcast_ref::<PayloadLimitExceeded>(),
        Some(&expected)
    );
    assert!(session.call("echo", json!({})).await.is_err());
    let pid: libc::pid_t = std::fs::read_to_string(fixture.binding.state_dir.join("pid"))
        .unwrap()
        .parse()
        .unwrap();
    // SAFETY: signal 0 probes this exact fixture child without sending a signal.
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("wire_accepted")).unwrap(),
        "oversized_reply\n"
    );
}
