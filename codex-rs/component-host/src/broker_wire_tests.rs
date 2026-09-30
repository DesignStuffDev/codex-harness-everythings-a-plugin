use super::*;

use std::io::Cursor;

use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::io::BufReader;

fn request_start(id: u64) -> Value {
    json!({"type":"broker_request_start","id":id,"service":"host.example","version":1,
        "method":"read","authority":"grant","context":"operation","parent_id":1,
        "request_generation":2,"expected_owner_generation":3,"bytes":2})
}

fn wire(frames: &[Value], enabled: bool) -> BrokerMessageReader<BufReader<Cursor<Vec<u8>>>> {
    let bytes = frames.iter().map(|frame| format!("{frame}\n")).collect::<String>().into_bytes();
    BrokerMessageReader::new(BufReader::new(Cursor::new(bytes)), enabled)
}

#[tokio::test]
async fn simultaneous_equal_ids_in_both_namespaces_do_not_collide() -> Result<()> {
    let mut reader = wire(&[
        json!({"type":"result_start","id":1,"bytes":2}),
        request_start(1),
        json!({"type":"broker_chunk","id":1,"index":0,"data":"e30="}),
        json!({"type":"chunk","id":1,"index":0,"data":"e30="}),
        json!({"type":"broker_end","id":1,"chunks":1}),
        json!({"type":"end","id":1,"chunks":1}),
    ], /*enabled*/ true);
    match reader.next().await?.context("broker request")? {
        BrokerIncoming::Request { header, payload } => {
            assert_eq!((header.id, header.service, payload.into_value().await?), (1, "host.example".to_owned(), json!({})));
        }
        _ => anyhow::bail!("expected dependency request"),
    }
    match reader.next().await?.context("ordinary result")? {
        BrokerIncoming::Ordinary { message, scope } => {
            assert!(scope.is_none());
            assert!(matches!(message.header, Header::Result { id: 1 }));
            assert_eq!(message.payload.context("payload")?.into_value().await?, json!({}));
        }
        _ => anyhow::bail!("expected ordinary result"),
    }
    assert!(reader.next().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn scoped_requests_require_negotiation_and_strict_context() -> Result<()> {
    let start = json!({"type":"request_start","id":1,"component":{"kind":"example","name":"default"},
        "method":"read","is_control":false,"bytes":2,
        "broker_context":{"handle":"operation","request_generation":2,"owner_generation":3}});
    assert!(wire(&[start.clone()], /*enabled*/ false).next().await.is_err());
    let mut invalid = start.clone();
    invalid["broker_context"]["extra"] = json!(true);
    assert!(wire(&[invalid], /*enabled*/ true).next().await.is_err());
    let mut reader = wire(&[start,
        json!({"type":"chunk","id":1,"index":0,"data":"e30="}),
        json!({"type":"end","id":1,"chunks":1}),
    ], /*enabled*/ true);
    match reader.next().await?.context("scoped request")? {
        BrokerIncoming::Ordinary { message, scope } => {
            assert!(matches!(message.header, Header::Request { id: 1, .. }));
            assert_eq!(scope, Some(DependencyScope { handle: "operation".to_owned(), request_generation: 2, owner_generation: 3 }));
        }
        _ => anyhow::bail!("expected scoped ordinary request"),
    }
    Ok(())
}

#[tokio::test]
async fn broker_frames_enforce_negotiation_bounds_order_and_shutdown() {
    assert!(wire(&[request_start(1)], /*enabled*/ false).next().await.is_err());
    let mut unknown = request_start(1);
    unknown["extra"] = json!(true);
    let mut long_handle = request_start(1);
    long_handle["authority"] = json!("x".repeat(MAX_HANDLE_BYTES + 1));
    let mut cases = vec![
        vec![unknown], vec![long_handle], vec![request_start(18)],
        vec![json!({"type":"broker_cancel","id":1})],
        vec![request_start(1), json!({"type":"broker_cancel","id":1})],
        vec![request_start(1), json!({"type":"shutdown"})],
        vec![request_start(1), json!({"type":"broker_chunk","id":1,"index":1,"data":"e30="})],
        vec![request_start(1), json!({"type":"broker_error","id":1,"code":"cancelled","message":"ignored"})],
    ];
    cases.push((1..=17).map(request_start).collect());
    for frames in cases {
        assert!(wire(&frames, /*enabled*/ true).next().await.is_err(), "{frames}", frames = json!(frames));
    }
    let duplicate = b"{\"type\":\"broker_result_start\",\"id\":1,\"id\":2,\"bytes\":2}\n{\"type\":\"broker_chunk\",\"id\":2,\"index\":0,\"data\":\"e30=\"}\n{\"type\":\"broker_end\",\"id\":2,\"chunks\":1}\n";
    let mut reader = BrokerMessageReader::new(BufReader::new(Cursor::new(duplicate)), /*enabled*/ true);
    assert!(reader.next().await.is_err());
}

#[tokio::test]
async fn late_cancellation_is_bounded_and_duplicate_results_fail() -> Result<()> {
    let mut reader = wire(&[
        request_start(1),
        json!({"type":"broker_chunk","id":1,"index":0,"data":"e30="}),
        json!({"type":"broker_end","id":1,"chunks":1}),
        json!({"type":"broker_cancel","id":1}),
        json!({"type":"broker_cancel","id":1}),
        request_start(1),
    ], /*enabled*/ true);
    assert!(matches!(reader.next().await?.context("request")?, BrokerIncoming::Request { .. }));
    for _ in 0..2 {
        assert!(matches!(reader.next().await?.context("cancel")?, BrokerIncoming::Cancel { id: 1 }));
    }
    assert!(reader.next().await.is_err());
    Ok(())
}

#[tokio::test]
async fn broker_error_discards_partial_result_and_remote_message() -> Result<()> {
    let invalid = wire(&[json!({"type":"broker_error","id":1,"code":"private token","message":"private token"})],
        /*enabled*/ true).next().await;
    match invalid {
        Err(error) => assert!(!format!("{error:#}").contains("private token")),
        Ok(_) => anyhow::bail!("expected invalid broker error code"),
    }
    let mut reader = wire(&[
        json!({"type":"broker_result_start","id":1,"bytes":10}),
        json!({"type":"broker_chunk","id":1,"index":0,"data":"eA=="}),
        json!({"type":"broker_error","id":1,"code":"owner_changed","message":"private token"}),
        json!({"type":"broker_result_start","id":2,"bytes":1}),
        json!({"type":"broker_chunk","id":2,"index":0,"data":"eA=="}),
        json!({"type":"broker_end","id":2,"chunks":1}),
    ], /*enabled*/ true);
    match reader.next().await?.context("error")? {
        BrokerIncoming::Error { id, error } => {
            assert_eq!((id, error), (1, DependencyError::new(DependencyErrorCode::OwnerChanged)));
        }
        _ => anyhow::bail!("expected broker error"),
    }
    match reader.next().await?.context("lazy result")? {
        BrokerIncoming::Result { payload, .. } => assert!(payload.into_value().await.is_err()),
        _ => anyhow::bail!("expected lazy result"),
    }
    assert!(reader.next().await?.is_none());
    Ok(())
}
