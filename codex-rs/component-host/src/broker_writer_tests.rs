use super::*;

use pretty_assertions::assert_eq;
use serde_json::json;
use std::pin::Pin;
use std::task::Context as TaskContext;
use std::task::Poll;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;

use crate::broker_api::DependencyCancellation;
use crate::broker_api::DependencyErrorCode;
use crate::broker_api::DependencyFuture;
use crate::broker_api::DependencyOwner;
use crate::broker_api::HostDependencyAuthority;
use crate::broker_api::HostOperationScope;
use crate::broker_api::OperationState;
use crate::broker_wire::BrokerIncoming;
use crate::broker_wire::BrokerMessageReader;
use crate::session_wire::CHUNK_BYTES;

fn header(id: u64) -> BrokerRequestHeader {
    BrokerRequestHeader {
        id,
        service: "host.example".to_owned(),
        version: 1,
        method: "read".to_owned(),
        authority: "grant".to_owned(),
        context: "operation".to_owned(),
        parent_id: 1,
        request_generation: 2,
        expected_owner_generation: 3,
    }
}

fn response(message: BrokerOutgoing) -> BrokerResponse {
    let (completed, _) = oneshot::channel();
    BrokerResponse { message, completed }
}

#[tokio::test]
async fn large_dependency_result_uses_shared_spool_without_frame_size_limit() -> Result<()> {
    let expected = json!({"native_catalog": "x".repeat(17 * 1024 * 1024)});
    let (writer, reader) = tokio::io::duplex(4096);
    let (regular_tx, regular) = mpsc::channel(1);
    let (control_tx, control) = mpsc::channel(1);
    let (request_tx, requests) = mpsc::channel(1);
    let (response_tx, responses) = mpsc::channel(1);
    let (cancel_tx, cancellations) = mpsc::channel(1);
    response_tx
        .send(response(BrokerOutgoing::Result {
            id: 1,
            result: expected.clone(),
            authority: None,
        }))
        .await?;
    drop((regular_tx, control_tx, request_tx, response_tx, cancel_tx));
    let writing = tokio::spawn(write_broker_messages(
        writer,
        regular,
        control,
        requests,
        responses,
        cancellations,
        BrokerWriterControl::new(),
    ));
    let mut reader = BrokerMessageReader::new(BufReader::new(reader), /*enabled*/ true);
    match reader.next().await?.context("broker result")? {
        BrokerIncoming::Result { id, payload } => {
            assert_eq!((id, payload.into_value().await?), (1, expected))
        }
        _ => anyhow::bail!("expected broker result"),
    }
    assert!(reader.next().await?.is_none());
    writing.await??;
    Ok(())
}

#[tokio::test]
async fn response_priority_and_cancel_fence_preserve_both_directions_progress() -> Result<()> {
    let (writer, reader) = tokio::io::duplex(4096);
    let (regular_tx, regular) = mpsc::channel(1);
    let (control_tx, control) = mpsc::channel(1);
    let (request_tx, requests) = mpsc::channel(1);
    let (response_tx, responses) = mpsc::channel(1);
    let (cancel_tx, cancellations) = mpsc::channel(1);
    let sent = Arc::new(AtomicBool::new(false));
    regular_tx
        .send(ScopedOutgoing {
            message: Outgoing {
                header: Header::Result { id: 1 },
                value: json!("forward".repeat(CHUNK_BYTES)),
                after_sent: None,
                sent: None,
            },
            scope: None,
        })
        .await?;
    request_tx
        .send(BrokerOutgoing::Request {
            header: header(1),
            params: json!("reverse".repeat(CHUNK_BYTES)),
            sent: Arc::clone(&sent),
        })
        .await?;
    response_tx
        .send(response(BrokerOutgoing::Result {
            id: 2,
            result: json!({"urgent":true}),
            authority: None,
        }))
        .await?;
    cancel_tx
        .send(BrokerCancellation {
            id: 1,
            after_sent: Arc::clone(&sent),
        })
        .await?;
    drop((regular_tx, control_tx, request_tx, response_tx, cancel_tx));
    let writing = tokio::spawn(write_broker_messages(
        writer,
        regular,
        control,
        requests,
        responses,
        cancellations,
        BrokerWriterControl::new(),
    ));
    let mut reader = BufReader::new(reader);
    let mut frames = vec![];
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }
        frames.push(serde_json::from_str::<Value>(&line)?);
    }
    writing.await??;
    let position = |kind: &str, id: u64| {
        frames
            .iter()
            .position(|frame| frame["type"] == kind && frame["id"] == id)
            .expect("frame")
    };
    assert_eq!(
        (frames[0]["type"].clone(), frames[0]["id"].clone()),
        (json!("broker_result_start"), json!(2))
    );
    assert!(position("broker_end", 2) < position("end", 1));
    assert!(position("broker_cancel", 1) > position("broker_end", 1));
    assert!(position("result_start", 1) < position("broker_end", 1));
    assert!(position("broker_request_start", 1) < position("end", 1));
    assert!(sent.load(Ordering::Acquire));
    Ok(())
}

struct Authority(AtomicBool);

impl HostDependencyAuthority for Authority {
    fn check(&self, _owner: &DependencyOwner) -> std::result::Result<(), DependencyError> {
        if self.0.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(DependencyError::new(DependencyErrorCode::OwnerChanged))
        }
    }

    fn revoked(&self, _owner: DependencyOwner) -> DependencyFuture<'_, ()> {
        Box::pin(std::future::pending())
    }
}

#[tokio::test]
async fn owner_revocation_replaces_partially_written_result() -> Result<()> {
    let (writer, reader) = tokio::io::duplex(4096);
    let (regular_tx, regular) = mpsc::channel(1);
    let (control_tx, control) = mpsc::channel(1);
    let (request_tx, requests) = mpsc::channel(1);
    let (response_tx, responses) = mpsc::channel(1);
    let (cancel_tx, cancellations) = mpsc::channel(1);
    let authority = Arc::new(Authority(AtomicBool::new(true)));
    let context = DependencyContext {
        operation: Arc::new(OperationState {
            parent_id: 1,
            generation: 1,
            scope: HostOperationScope {
                owner: DependencyOwner {
                    generation: 1,
                    identity: None,
                },
                deadline: tokio::time::Instant::now() + std::time::Duration::from_secs(60),
                cancellation: DependencyCancellation::new(),
                request_context: None,
            },
            finished: DependencyCancellation::new(),
        }),
        authority: authority.clone(),
    };
    response_tx
        .send(response(BrokerOutgoing::Result {
            id: 1,
            result: json!("x".repeat(4 * CHUNK_BYTES)),
            authority: Some(context),
        }))
        .await?;
    drop((regular_tx, control_tx, request_tx, response_tx, cancel_tx));
    let writing = tokio::spawn(write_broker_messages(
        writer,
        regular,
        control,
        requests,
        responses,
        cancellations,
        BrokerWriterControl::new(),
    ));
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    assert_eq!(
        serde_json::from_str::<Value>(&line)?["type"],
        "broker_result_start"
    );
    authority.0.store(false, Ordering::Release);
    let mut terminal = None;
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }
        let frame: Value = serde_json::from_str(&line)?;
        assert_ne!(frame["type"], "broker_end");
        if frame["type"] == "broker_error" {
            terminal = Some(frame);
        }
    }
    assert_eq!(
        terminal,
        Some(json!({"type":"broker_error","id":1,"code":"owner_changed",
        "message":"host dependency owner changed"}))
    );
    writing.await??;
    Ok(())
}

#[tokio::test]
async fn missing_cancellation_send_fence_fails_instead_of_hanging() -> Result<()> {
    let (regular_tx, regular) = mpsc::channel(1);
    let (control_tx, control) = mpsc::channel(1);
    let (request_tx, requests) = mpsc::channel(1);
    let (response_tx, responses) = mpsc::channel(1);
    let (cancel_tx, cancellations) = mpsc::channel(1);
    cancel_tx
        .send(BrokerCancellation {
            id: 1,
            after_sent: Arc::new(AtomicBool::new(false)),
        })
        .await?;
    drop((regular_tx, control_tx, request_tx, response_tx, cancel_tx));
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            write_broker_messages(
                tokio::io::sink(),
                regular,
                control,
                requests,
                responses,
                cancellations,
                BrokerWriterControl::new()
            )
        )
        .await?
        .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn full_closed_deferred_lanes_reject_impossible_fences_without_hanging() -> Result<()> {
    enum Lane {
        Control,
        Cancellation,
    }
    for lane in [Lane::Control, Lane::Cancellation] {
        let (regular_tx, regular) = mpsc::channel(1);
        let (control_tx, control) = mpsc::channel(MAX_PENDING);
        let (request_tx, requests) = mpsc::channel(1);
        let (response_tx, responses) = mpsc::channel(1);
        let (cancel_tx, cancellations) = mpsc::channel(MAX_BROKER_PENDING);
        match lane {
            Lane::Control => {
                for id in 1..=MAX_PENDING as u64 {
                    control_tx
                        .send(Outgoing {
                            header: Header::Request {
                                id,
                                component: SessionComponent {
                                    kind: "store".to_owned(),
                                    name: "default".to_owned(),
                                },
                                method: "release".to_owned(),
                                is_control: true,
                            },
                            value: json!({}),
                            after_sent: Some(Arc::new(AtomicBool::new(false))),
                            sent: None,
                        })
                        .await?;
                }
            }
            Lane::Cancellation => {
                for id in 1..=MAX_BROKER_PENDING as u64 {
                    cancel_tx
                        .send(BrokerCancellation {
                            id,
                            after_sent: Arc::new(AtomicBool::new(false)),
                        })
                        .await?;
                }
            }
        }
        drop((regular_tx, control_tx, request_tx, response_tx, cancel_tx));
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            write_broker_messages(
                tokio::io::sink(),
                regular,
                control,
                requests,
                responses,
                cancellations,
                BrokerWriterControl::new(),
            ),
        )
        .await?;
        assert_eq!(
            outcome.unwrap_err().to_string(),
            "cleanup or cancellation depends on an unsent request"
        );
    }
    Ok(())
}

#[tokio::test]
async fn cleanup_progresses_during_a_large_dependency_response() -> Result<()> {
    let (writer, reader) = tokio::io::duplex(4096);
    let (regular_tx, regular) = mpsc::channel(1);
    let (control_tx, control) = mpsc::channel(1);
    let (request_tx, requests) = mpsc::channel(1);
    let (response_tx, responses) = mpsc::channel(1);
    let (cancel_tx, cancellations) = mpsc::channel(1);
    response_tx
        .send(response(BrokerOutgoing::Result {
            id: 1,
            result: json!("x".repeat(12 * CHUNK_BYTES)),
            authority: None,
        }))
        .await?;
    drop((regular_tx, request_tx, response_tx, cancel_tx));
    let writing = tokio::spawn(write_broker_messages(
        writer,
        regular,
        control,
        requests,
        responses,
        cancellations,
        BrokerWriterControl::new(),
    ));
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    assert_eq!(
        serde_json::from_str::<Value>(&line)?["type"],
        "broker_result_start"
    );
    control_tx
        .send(Outgoing {
            header: Header::Request {
                id: 1,
                component: SessionComponent {
                    kind: "store".to_owned(),
                    name: "default".to_owned(),
                },
                method: "release".to_owned(),
                is_control: true,
            },
            value: json!({}),
            after_sent: Some(Arc::new(AtomicBool::new(true))),
            sent: None,
        })
        .await?;
    drop(control_tx);
    let mut frames = Vec::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }
        frames.push(serde_json::from_str::<Value>(&line)?);
    }
    writing.await??;
    let release = frames
        .iter()
        .position(|frame| frame["type"] == "end")
        .context("release end")?;
    let result = frames
        .iter()
        .position(|frame| frame["type"] == "broker_end")
        .context("broker result end")?;
    assert!(release < result);
    assert!(
        frames[..release]
            .iter()
            .filter(|frame| frame["type"] == "broker_chunk")
            .count()
            <= 2
    );
    Ok(())
}

#[tokio::test]
async fn terminal_frame_drains_large_active_and_queued_broker_results() -> Result<()> {
    let (writer, reader) = tokio::io::duplex(4096);
    let (regular_tx, regular) = mpsc::channel(1);
    let (control_tx, control) = mpsc::channel(1);
    let (request_tx, requests) = mpsc::channel(1);
    let (response_tx, responses) = mpsc::channel(2);
    let (cancel_tx, cancellations) = mpsc::channel(1);
    for id in 1..=2 {
        response_tx
            .send(response(BrokerOutgoing::Result {
                id,
                result: json!("x".repeat(3 * CHUNK_BYTES)),
                authority: None,
            }))
            .await?;
    }
    regular_tx
        .send(ScopedOutgoing {
            message: Outgoing {
                header: Header::Shutdown,
                value: Value::Null,
                after_sent: None,
                sent: None,
            },
            scope: None,
        })
        .await?;
    drop((regular_tx, control_tx, request_tx, response_tx, cancel_tx));
    let writing = tokio::spawn(write_broker_messages(
        writer,
        regular,
        control,
        requests,
        responses,
        cancellations,
        BrokerWriterControl::new(),
    ));
    let mut reader = BrokerMessageReader::new(BufReader::new(reader), /*enabled*/ true);
    for expected in 1..=2 {
        match reader
            .next()
            .await?
            .context("dependency result before shutdown")?
        {
            BrokerIncoming::Result { id, .. } => assert_eq!(id, expected),
            _ => anyhow::bail!("shutdown interrupted dependency results"),
        }
    }
    match reader.next().await?.context("shutdown")? {
        BrokerIncoming::Ordinary { message, .. } => {
            assert!(matches!(message.header, Header::Shutdown))
        }
        _ => anyhow::bail!("expected shutdown"),
    }
    assert!(reader.next().await?.is_none());
    writing.await??;
    Ok(())
}

/// Accept every byte but hold the terminal frame's actual poll_flush operation.
struct FlushGate {
    frame: Vec<u8>,
    terminal: bool,
    arrived: Option<oneshot::Sender<()>>,
    release: mpsc::Receiver<()>,
}

impl AsyncWrite for FlushGate {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.frame.extend_from_slice(bytes);
        if self.frame.last() == Some(&b'\n') {
            let frame: Value =
                serde_json::from_slice(&self.frame).map_err(std::io::Error::other)?;
            self.terminal = frame["type"] == "broker_end";
            self.frame.clear();
        }
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        if self.terminal {
            if let Some(arrived) = self.arrived.take() {
                let _ = arrived.send(());
            }
            match self.release.poll_recv(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    return Poll::Ready(Err(std::io::Error::other("flush gate closed")));
                }
                Poll::Ready(Some(())) => self.terminal = false,
            }
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

enum FlushOutcome {
    Released,
    Cancelled,
    Failed,
}

async fn response_completion_follows_terminal_flush(outcome: FlushOutcome) -> Result<()> {
    let (arrived, flushing) = oneshot::channel();
    let (release, gate) = mpsc::channel(1);
    let writer = FlushGate {
        frame: Vec::new(),
        terminal: false,
        arrived: Some(arrived),
        release: gate,
    };
    let (regular_tx, regular) = mpsc::channel(1);
    let (control_tx, control) = mpsc::channel(1);
    let (request_tx, requests) = mpsc::channel(1);
    let (response_tx, responses) = mpsc::channel(1);
    let (cancel_tx, cancellations) = mpsc::channel(1);
    let (completed, mut completion) = oneshot::channel();
    response_tx
        .send(BrokerResponse {
            message: BrokerOutgoing::Result {
                id: 1,
                result: json!({"catalog": []}),
                authority: None,
            },
            completed,
        })
        .await?;
    drop((regular_tx, control_tx, request_tx, response_tx, cancel_tx));
    let token = BrokerWriterControl::new();
    let writing = tokio::spawn(write_broker_messages(
        writer,
        regular,
        control,
        requests,
        responses,
        cancellations,
        token.clone(),
    ));
    tokio::time::timeout(std::time::Duration::from_secs(5), flushing).await??;
    assert_eq!(
        completion.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    );
    assert!(!writing.is_finished());
    match outcome {
        FlushOutcome::Released => {
            release.send(()).await?;
            tokio::time::timeout(std::time::Duration::from_secs(5), writing).await???;
            assert_eq!(completion.await?, Ok(()));
        }
        FlushOutcome::Cancelled => {
            token.cancel();
            // A controlled stop confirms writer cleanup, not response delivery.
            tokio::time::timeout(std::time::Duration::from_secs(5), writing).await???;
            assert_eq!(
                completion.await?,
                Err("dependency response flush was not confirmed".to_owned())
            );
        }
        FlushOutcome::Failed => {
            drop(release);
            assert!(
                tokio::time::timeout(std::time::Duration::from_secs(5), writing)
                    .await??
                    .is_err()
            );
            assert_eq!(
                completion.await?,
                Err("dependency response flush was not confirmed".to_owned())
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn response_acknowledgement_waits_for_the_final_physical_flush() -> Result<()> {
    response_completion_follows_terminal_flush(FlushOutcome::Released).await
}

#[tokio::test]
async fn controlled_stop_during_final_flush_never_acknowledges_delivery() -> Result<()> {
    response_completion_follows_terminal_flush(FlushOutcome::Cancelled).await
}

#[tokio::test]
async fn failed_terminal_flush_fails_response_and_writer() -> Result<()> {
    response_completion_follows_terminal_flush(FlushOutcome::Failed).await
}
