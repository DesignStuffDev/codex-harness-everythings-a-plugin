#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io::Cursor;
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tokio::io::BufReader;
use tokio::io::DuplexStream;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;

use super::ComponentServer;
use super::End;
use super::ServerState;
use super::SessionInitialization;
use super::read_requests;
use crate::PayloadKind;
use crate::PayloadLimitExceeded;
use crate::SessionPayloadLimits;
use crate::session::CALL_SLOTS;
use crate::session_failure::SessionFailure;
use crate::session_wire::Header;
use crate::session_wire::MAX_PENDING;
use crate::session_wire::MessageReader;
use crate::session_wire::write_messages;

fn request(id: u64, is_control: bool, params: Value) -> Vec<Value> {
    let bytes = serde_json::to_vec(&params).unwrap();
    vec![
        json!({"type":"request_start", "id":id,
            "component":{"kind":"file_search", "name":"default"},
            "method":if is_control {"release"} else {"query"},
            "is_control":is_control, "bytes":bytes.len()}),
        json!({"type":"chunk", "id":id, "index":0, "data":STANDARD.encode(bytes)}),
        json!({"type":"end", "id":id, "chunks":1}),
    ]
}

fn server(
    frames: Vec<Value>,
    limits: SessionPayloadLimits,
) -> (ComponentServer, BufReader<DuplexStream>) {
    let bytes = frames
        .into_iter()
        .map(|frame| format!("{frame}\n"))
        .collect::<String>()
        .into_bytes();
    let stdin = BufReader::new(Cursor::new(bytes));
    let (stdout, output) = tokio::io::duplex(4096);
    let (regular, regular_rx) = mpsc::channel(CALL_SLOTS);
    let (control, control_rx) = mpsc::channel(CALL_SLOTS);
    let (incoming, receiver) = mpsc::channel(MAX_PENDING);
    let state = Arc::new(ServerState {
        end: Mutex::new(None),
        disconnected: AtomicBool::new(false),
        regular_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
        control_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
    });
    let reader_state = Arc::clone(&state);
    let responses = (regular.clone(), control.clone());
    let reader = tokio::spawn(async move {
        let outcome = read_requests(
            stdin,
            incoming,
            responses,
            Arc::clone(&reader_state),
            limits,
        )
        .await;
        let end = outcome.unwrap_or_else(|error| End::Failure(SessionFailure::from_error(error)));
        reader_state
            .disconnected
            .store(!matches!(end, End::Shutdown), Ordering::Release);
        *reader_state.end.lock().unwrap() = Some(end);
    });
    let writer = tokio::spawn(write_messages(stdout, regular_rx, control_rx, limits));
    (
        ComponentServer {
            initialization: SessionInitialization {
                api_version: 1,
                plugin_id: "fixture".to_owned(),
                config: json!({}),
                state_dir: PathBuf::new(),
            },
            incoming: receiver,
            regular: Some(regular),
            control: Some(control),
            reader: Some(reader),
            writer: Some(writer),
            state,
        },
        BufReader::new(output),
    )
}

#[tokio::test]
async fn oversized_reply_uses_legacy_error_and_preserves_control_lane() {
    let limits = SessionPayloadLimits {
        request_bytes: NonZeroU64::new(2),
        reply_bytes: NonZeroU64::new(128 * 1024),
    };
    let mut frames = request(/*id*/ 1, /*is_control*/ false, json!({}));
    frames.extend(request(
        /*id*/ 2,
        /*is_control*/ true,
        json!({"lease":"control"}),
    ));
    frames.push(json!({"type":"shutdown"}));
    let (mut server, output) = server(frames, limits);
    let ordinary = server.next().await.unwrap().unwrap();
    let large = json!("x".repeat(128 * 1024));
    let expected = PayloadLimitExceeded {
        kind: PayloadKind::Reply,
        limit_bytes: 128 * 1024,
    };
    assert_eq!(
        ordinary
            .validate_response(&large)
            .unwrap_err()
            .downcast_ref(),
        Some(&expected)
    );
    assert_eq!(
        ordinary
            .respond(Ok(large))
            .await
            .unwrap_err()
            .downcast_ref(),
        Some(&expected)
    );
    let control = server.next().await.unwrap().unwrap();
    let receipt = json!({"cleanup":{"status":"unconfirmed"}});
    control.respond(Ok(receipt.clone())).await.unwrap();
    assert!(server.next().await.unwrap().is_none());
    server.finish().await.unwrap();
    let mut reader = MessageReader::new(output).with_limits(limits);
    let mut replies = Vec::new();
    while let Some(message) = reader.next().await.unwrap() {
        match message.header {
            Header::Error { id, message } => replies.push((id, json!({"legacy_error":message}))),
            Header::Result { id } => {
                replies.push((id, message.payload.unwrap().into_value().await.unwrap()))
            }
            Header::ShutdownComplete => break,
            Header::Request { .. } | Header::Shutdown => panic!("unexpected server frame"),
        }
    }
    replies.sort_by_key(|(id, _)| *id);
    assert_eq!(
        replies,
        vec![
            (
                1,
                json!({"legacy_error":"component response exceeds configured payload limit"})
            ),
            (2, receipt),
        ]
    );
}

#[tokio::test]
async fn response_preflight_uses_reserved_lane_even_if_public_metadata_changes() {
    let limits = SessionPayloadLimits {
        request_bytes: None,
        reply_bytes: NonZeroU64::new(128 * 1024),
    };
    let mut frames = request(/*id*/ 1, /*is_control*/ true, json!({}));
    frames.push(json!({"type":"shutdown"}));
    let (mut server, output) = server(frames, limits);
    let mut request = server.next().await.unwrap().unwrap();
    request.is_control = false;
    let error = request
        .respond(Ok(json!("x".repeat(64 * 1024))))
        .await
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<PayloadLimitExceeded>(),
        Some(&PayloadLimitExceeded {
            kind: PayloadKind::Reply,
            limit_bytes: 64 * 1024,
        })
    );
    assert!(server.next().await.unwrap().is_none());
    server.finish().await.unwrap();
    let mut reader = MessageReader::new(output);
    assert!(matches!(
        reader.next().await.unwrap().unwrap().header,
        Header::Error { id: 1, .. }
    ));
    assert!(matches!(
        reader.next().await.unwrap().unwrap().header,
        Header::ShutdownComplete
    ));
}

#[tokio::test]
async fn oversized_request_is_rejected_before_body_decode_and_survives_finish() {
    let limits = SessionPayloadLimits {
        request_bytes: NonZeroU64::new(8),
        reply_bytes: NonZeroU64::new(64 * 1024),
    };
    let mut frames = request(/*id*/ 1, /*is_control*/ false, json!({}));
    frames[0]["bytes"] = json!(u64::MAX);
    frames[1]["data"] = json!("not valid base64 or JSON");
    let (mut server, _output) = server(frames, limits);
    let expected = PayloadLimitExceeded {
        kind: PayloadKind::Request,
        limit_bytes: 8,
    };
    assert_eq!(
        server.next().await.unwrap_err().downcast_ref(),
        Some(&expected)
    );
    assert!(server.was_disconnected());
    assert_eq!(
        server.finish().await.unwrap_err().downcast_ref(),
        Some(&expected)
    );
}

#[tokio::test]
async fn reader_size_failure_is_retained_when_a_later_writer_also_fails() {
    for observe in ["next", "finish"] {
        let limits = SessionPayloadLimits {
            request_bytes: NonZeroU64::new(8),
            reply_bytes: NonZeroU64::new(64 * 1024),
        };
        let mut frames = request(/*id*/ 1, /*is_control*/ false, json!({}));
        let mut oversized = request(/*id*/ 2, /*is_control*/ false, json!({}));
        oversized[0]["bytes"] = json!(u64::MAX);
        frames.extend(oversized);
        let (mut server, output) = server(frames, limits);
        let request = server.next().await.unwrap().unwrap();
        while server.state.end.lock().unwrap().is_none() {
            tokio::task::yield_now().await;
        }
        drop(output);
        request.respond(Ok(json!("reply"))).await.unwrap();
        let expected = PayloadLimitExceeded {
            kind: PayloadKind::Request,
            limit_bytes: 8,
        };
        if observe == "next" {
            assert_eq!(
                server.next().await.unwrap_err().downcast_ref(),
                Some(&expected)
            );
        }
        let error = server.finish().await.unwrap_err();
        assert_eq!(
            error.downcast_ref::<PayloadLimitExceeded>(),
            Some(&expected)
        );
        assert!(format!("{error:#}").contains("writer failure"));
    }
}
