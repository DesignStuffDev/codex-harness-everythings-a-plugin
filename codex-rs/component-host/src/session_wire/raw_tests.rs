//! Raw bytes are essential: a Value intermediary would erase duplicate fields.
use super::MessageReader;
use std::io::Cursor;
use tokio::io::BufReader;

#[tokio::test]
async fn raw_duplicate_fields_are_rejected_before_assembly() {
    let frames = [
        r#"{"type":"result_start","id":1,"id":2,"bytes":0}"#,
        r#"{"type":"result_start","id":1,"bytes":0,"bytes":1}"#,
        r#"{"type":"result_start","type":"error","id":1,"bytes":0}"#,
        r#"{"type":"request_start","id":1,"component":{"kind":"a","kind":"b","name":"n"},"method":"m","is_control":false,"bytes":0}"#,
    ];
    for frame in frames {
        let bytes = format!("{frame}\n").into_bytes();
        let mut reader = MessageReader::new(BufReader::new(Cursor::new(bytes)));
        let error = match reader.next().await {
            Err(error) => error,
            Ok(_) => panic!("raw duplicate fields must fail: {frame}"),
        };
        let diagnostic = format!("{error:#}");
        assert!(diagnostic.contains("invalid session frame"), "{diagnostic}");
        assert!(diagnostic.contains("duplicate field"), "{diagnostic}");
    }
}
