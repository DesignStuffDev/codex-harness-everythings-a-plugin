#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use serde_json::json;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::timeout;

use super::CALL_SLOTS;
use super::ComponentServer;
use super::End;
use super::ServerState;
use super::SessionInitialization;

#[tokio::test]
async fn cancelling_next_during_reader_completion_preserves_shutdown() {
    let (incoming, receiver) = mpsc::channel(1);
    drop(incoming);
    let (finish, finished) = oneshot::channel();
    let state = Arc::new(ServerState {
        end: Mutex::new(None),
        disconnected: AtomicBool::new(false),
        regular_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
        control_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
    });
    let reader_state = Arc::clone(&state);
    let reader = tokio::spawn(async move {
        finished.await.unwrap();
        *reader_state.end.lock().unwrap() = Some(End::Shutdown);
    });
    let mut server = ComponentServer {
        initialization: SessionInitialization {
            api_version: 1,
            plugin_id: "fixture".to_owned(),
            config: json!({}),
            state_dir: PathBuf::new(),
        },
        incoming: receiver,
        regular: None,
        control: None,
        reader: Some(reader),
        writer: None,
        state,
    };
    // Model a handler completing after the input channel closes but before the
    // reader publishes terminal state: select! drops this pending next future.
    assert!(
        timeout(Duration::from_millis(10), server.next())
            .await
            .is_err()
    );
    finish.send(()).unwrap();
    assert!(server.next().await.unwrap().is_none());
    assert!(!server.was_disconnected());
}
