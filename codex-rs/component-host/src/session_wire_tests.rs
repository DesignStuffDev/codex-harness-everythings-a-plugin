use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::io::Cursor;
use tokio::io::BufReader;

fn result(id: u64, value: Value) -> Outgoing {
    Outgoing {
        header: Header::Result { id },
        value,
        after_sent: None,
        sent: None,
    }
}

fn wire(frames: &[Value]) -> MessageReader<BufReader<Cursor<Vec<u8>>>> {
    let bytes = frames
        .iter()
        .map(|frame| format!("{frame}\n"))
        .collect::<String>()
        .into_bytes();
    MessageReader::new(BufReader::new(Cursor::new(bytes)))
}

#[tokio::test]
async fn histories_larger_than_sixteen_megabytes_round_trip() -> Result<()> {
    let expected = json!({"history": "x".repeat(17 * 1024 * 1024)});
    let (writer, reader) = tokio::io::duplex(4096);
    let (regular_tx, regular_rx) = mpsc::channel(1);
    let (control_tx, control_rx) = mpsc::channel(1);
    regular_tx.send(result(/*id*/ 1, expected.clone())).await?;
    drop(regular_tx);
    drop(control_tx);
    let writing = tokio::spawn(write_messages(writer, regular_rx, control_rx));
    let mut reading = MessageReader::new(BufReader::new(reader));
    let incoming = reading.next().await?.context("message")?;
    assert!(matches!(incoming.header, Header::Result { id: 1 }));
    assert_eq!(
        incoming.payload.context("payload")?.into_value().await?,
        expected
    );
    assert!(reading.next().await?.is_none());
    writing.await??;
    Ok(())
}

#[tokio::test]
async fn malformed_streams_fail_without_accepting_partial_values() {
    let cases = vec![
        vec![json!({"type":"chunk","id":1,"index":0,"data":"eA=="})],
        vec![
            json!({"type":"result_start","id":1,"bytes":1}),
            json!({"type":"chunk","id":1,"index":1,"data":"eA=="}),
        ],
        vec![
            json!({"type":"result_start","id":1,"bytes":1}),
            json!({"type":"chunk","id":1,"index":0,"data":"not-base64"}),
        ],
        vec![
            json!({"type":"result_start","id":1,"bytes":1}),
            json!({"type":"chunk","id":1,"index":0,"data":"eHg="}),
        ],
        vec![
            json!({"type":"result_start","id":1,"bytes":2}),
            json!({"type":"chunk","id":1,"index":0,"data":"eA=="}),
            json!({"type":"end","id":1,"chunks":1}),
        ],
        vec![
            json!({"type":"result_start","id":1,"bytes":1}),
            json!({"type":"chunk","id":1,"index":0,"data":"eA=="}),
            json!({"type":"end","id":1,"chunks":2}),
        ],
        vec![
            json!({"type":"result_start","id":1,"bytes":1}),
            json!({"type":"shutdown"}),
        ],
        vec![json!({"type":"result_start","id":1,"bytes":1})],
        vec![json!({"type":"result_start","id":0,"bytes":1})],
        vec![json!({"type":"result_start","id":66,"bytes":1})],
        vec![
            json!({"type":"request_start","id":1,"component":{"kind":"x","name":"y"},"method":"x","is_control":true,"bytes":65537}),
        ],
        vec![
            json!({"type":"request_start","id":1,"component":{"kind":"x","name":"y"},"method":"x".repeat(257),"is_control":false,"bytes":1}),
        ],
        vec![
            json!({"type":"result_start","id":1,"bytes":CHUNK_BYTES + 1}),
            json!({"type":"chunk","id":1,"index":0,"data":STANDARD.encode(vec![b'x'; CHUNK_BYTES + 1])}),
        ],
    ];
    for frames in cases {
        assert!(wire(&frames).next().await.is_err(), "{frames:?}");
    }
    let mut no_newline =
        MessageReader::new(BufReader::new(Cursor::new(vec![b'x'; MAX_FRAME_BYTES + 1])));
    assert!(no_newline.next().await.is_err());
}

#[tokio::test]
async fn out_of_order_ids_are_bounded_and_completed_duplicates_rejected() -> Result<()> {
    let mut reader = wire(&[
        json!({"type":"error","id":3,"message":"third"}),
        json!({"type":"error","id":1,"message":"first"}),
        json!({"type":"error","id":2,"message":"second"}),
        json!({"type":"error","id":1,"message":"duplicate"}),
    ]);
    for expected in [3, 1, 2] {
        assert!(
            matches!(reader.next().await?.context("message")?.header, Header::Error { id, .. } if id == expected)
        );
    }
    assert!(reader.next().await.is_err());
    let frames: Vec<_> = (1..=65)
        .map(|id| json!({"type":"result_start","id":id,"bytes":1}))
        .collect();
    assert!(wire(&frames).next().await.is_err());
    Ok(())
}

#[tokio::test]
async fn error_discards_partial_result_and_json_parsing_is_lazy() -> Result<()> {
    let mut reader = wire(&[
        json!({"type":"result_start","id":1,"bytes":5}),
        json!({"type":"chunk","id":1,"index":0,"data":"eA=="}),
        json!({"type":"error","id":1,"message":"abandoned"}),
        json!({"type":"result_start","id":2,"bytes":1}),
        json!({"type":"chunk","id":2,"index":0,"data":"eA=="}),
        json!({"type":"end","id":2,"chunks":1}),
    ]);
    assert!(matches!(
        reader.next().await?.context("message")?.header,
        Header::Error { id: 1, .. }
    ));
    let incoming = reader.next().await?.context("lazy message")?;
    assert!(
        incoming
            .payload
            .context("payload")?
            .into_value()
            .await
            .is_err()
    );
    assert!(reader.next().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn urgent_controls_interleave_but_cleanup_waits_for_request_end() -> Result<()> {
    let (writer, reader) = tokio::io::duplex(4096);
    let (regular_tx, regular_rx) = mpsc::channel(1);
    let (control_tx, control_rx) = mpsc::channel(4);
    let sent = Arc::new(AtomicBool::new(false));
    let mut ordinary = result(/*id*/ 1, json!("x".repeat(3 * CHUNK_BYTES)));
    ordinary.sent = Some(Arc::clone(&sent));
    regular_tx.send(ordinary).await?;
    drop(regular_tx);
    let writing = tokio::spawn(write_messages(writer, regular_rx, control_rx));
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    assert_eq!(
        serde_json::from_str::<Value>(&line)?["type"],
        "result_start"
    );
    let mut cleanup = result(/*id*/ 3, json!("cleanup"));
    cleanup.after_sent = Some(Arc::clone(&sent));
    control_tx.send(cleanup).await?;
    control_tx.send(result(/*id*/ 2, json!("urgent"))).await?;
    drop(control_tx);
    let mut frames = vec![];
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }
        frames.push(serde_json::from_str::<Value>(&line)?);
    }
    writing.await??;
    let end = |id| {
        frames
            .iter()
            .position(|frame| frame["type"] == "end" && frame["id"] == id)
            .expect("end")
    };
    let start = |id| {
        frames
            .iter()
            .position(|frame| frame["type"] == "result_start" && frame["id"] == id)
            .expect("start")
    };
    assert!(end(2) < end(1));
    assert!(start(3) > end(1));
    assert!(sent.load(Ordering::Acquire));
    Ok(())
}

#[tokio::test]
async fn oversized_controls_and_never_sent_dependencies_fail() -> Result<()> {
    for gated in [false, true] {
        let (regular_tx, regular_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = mpsc::channel(1);
        let mut outgoing = result(/*id*/ 1, json!("x".repeat(65536)));
        if gated {
            outgoing.after_sent = Some(Arc::new(AtomicBool::new(false)));
        }
        control_tx.send(outgoing).await?;
        drop(regular_tx);
        drop(control_tx);
        assert!(
            write_messages(tokio::io::sink(), regular_rx, control_rx)
                .await
                .is_err()
        );
    }
    Ok(())
}
