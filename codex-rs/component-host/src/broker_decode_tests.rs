#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::future::Future;
use std::io::Cursor;
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::task::Poll;
use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::time::Instant;
use tokio::time::timeout;

use super::decode;
use super::decode_reader;
use crate::broker_api::DependencyCancellation;
use crate::broker_api::DependencyContext;
use crate::broker_api::DependencyError;
use crate::broker_api::DependencyErrorCode;
use crate::broker_api::DependencyFuture;
use crate::broker_api::DependencyOwner;
use crate::broker_api::HostDependencyAuthority;
use crate::broker_api::HostOperationScope;
use crate::broker_api::MAX_BROKER_REQUEST_BYTES;
use crate::broker_api::OperationState;
use crate::session_wire::Payload;

struct Authority;

impl HostDependencyAuthority for Authority {
    fn check(&self, _owner: &DependencyOwner) -> Result<(), DependencyError> {
        Ok(())
    }

    fn revoked(&self, _owner: DependencyOwner) -> DependencyFuture<'_, ()> {
        Box::pin(std::future::pending())
    }
}

fn context() -> DependencyContext {
    DependencyContext {
        operation: Arc::new(OperationState {
            parent_id: 1,
            generation: 1,
            scope: HostOperationScope::new(
                DependencyOwner {
                    generation: 1,
                    identity: None,
                },
                Instant::now() + Duration::from_secs(30),
            ),
            finished: DependencyCancellation::new(),
        }),
        authority: Arc::new(Authority),
    }
}

struct GatedReader {
    started: Option<oneshot::Sender<()>>,
    release: mpsc::Receiver<()>,
    dropped: Arc<AtomicBool>,
    input: Cursor<Vec<u8>>,
}

impl Read for GatedReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if let Some(started) = self.started.take() {
            let _ = started.send(());
            // Dropping the sender also releases this worker during panic cleanup.
            self.release.recv().map_err(std::io::Error::other)?;
        }
        self.input.read(output)
    }
}

impl Drop for GatedReader {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

enum CancellationSource {
    Peer,
    Parent,
    ParentCompleted,
}

async fn cancellation_owns_blocked_reader(source: CancellationSource) {
    let context = context();
    let (started, reading) = oneshot::channel();
    let (release, gate) = mpsc::channel();
    let dropped = Arc::new(AtomicBool::new(false));
    let reader = GatedReader {
        started: Some(started),
        release: gate,
        dropped: Arc::clone(&dropped),
        input: Cursor::new(br#"{"value":"read must join"}"#.to_vec()),
    };
    let (cancel, cancelled) = watch::channel(false);
    let decoding = decode_reader(reader, &context, cancelled);
    tokio::pin!(decoding);
    std::future::poll_fn(|cx| {
        assert!(decoding.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    timeout(Duration::from_secs(5), reading)
        .await
        .unwrap()
        .unwrap();
    match source {
        CancellationSource::Peer => {
            cancel.send_replace(true);
        }
        CancellationSource::Parent => context.operation.scope.cancellation.cancel(),
        CancellationSource::ParentCompleted => context.operation.finished.cancel(),
    }
    // This poll must process cancellation but cannot finish while Read still owns
    // the private spool/reader. A detached parser would incorrectly return Ready.
    std::future::poll_fn(|cx| {
        assert!(decoding.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(!dropped.load(Ordering::Acquire));
    release.send(()).unwrap();
    assert_eq!(
        timeout(Duration::from_secs(5), decoding).await.unwrap(),
        Err(DependencyError::new(DependencyErrorCode::Cancelled))
    );
    assert!(dropped.load(Ordering::Acquire));
}

#[tokio::test]
async fn peer_cancellation_joins_running_parser_before_returning() {
    cancellation_owns_blocked_reader(CancellationSource::Peer).await;
}

#[tokio::test]
async fn parent_cancellation_joins_running_parser_before_returning() {
    cancellation_owns_blocked_reader(CancellationSource::Parent).await;
}

#[tokio::test]
async fn parent_completion_joins_running_parser_before_returning() {
    cancellation_owns_blocked_reader(CancellationSource::ParentCompleted).await;
}

#[tokio::test]
async fn pre_cancelled_request_drops_reader_without_starting_read() {
    let context = context();
    let (started, mut reading) = oneshot::channel();
    let (_release, gate) = mpsc::channel();
    let dropped = Arc::new(AtomicBool::new(false));
    let reader = GatedReader {
        started: Some(started),
        release: gate,
        dropped: Arc::clone(&dropped),
        input: Cursor::new(b"{}".to_vec()),
    };
    let (_cancel, cancelled) = watch::channel(true);
    assert_eq!(
        timeout(
            Duration::from_secs(5),
            decode_reader(reader, &context, cancelled)
        )
        .await
        .unwrap(),
        Err(DependencyError::new(DependencyErrorCode::Cancelled))
    );
    assert_eq!(
        reading.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    );
    assert!(dropped.load(Ordering::Acquire));
}

#[tokio::test]
async fn oversized_spooled_request_returns_typed_size_error() {
    let context = context();
    let (_cancel, cancelled) = watch::channel(false);
    // JSON adds two quotation bytes, so this is larger than the 8 MiB wire body cap.
    let payload = Payload::from_value(Value::String("x".repeat(MAX_BROKER_REQUEST_BYTES as usize)))
        .await
        .unwrap();
    assert_eq!(
        decode(payload, &context, cancelled).await,
        Err(DependencyError::new(DependencyErrorCode::RequestTooLarge))
    );
}

#[tokio::test]
async fn malformed_json_is_redacted_and_valid_spooled_json_decodes() {
    let context = context();
    let (_cancel, cancelled) = watch::channel(false);
    let error = decode_reader(
        Cursor::new(b"{\"token\":\"SECRET_REQUEST_VALUE\",invalid".to_vec()),
        &context,
        cancelled.clone(),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error,
        DependencyError::new(DependencyErrorCode::InvalidRequest)
    );
    assert!(!format!("{error:#} {error:?}").contains("SECRET_REQUEST_VALUE"));
    let value = json!({"model": "fixture", "revision": 9});
    let payload = Payload::from_value(value.clone()).await.unwrap();
    assert_eq!(decode(payload, &context, cancelled).await, Ok(value));
}
