#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::future::Future;
use std::io::Write;
use std::sync::Mutex;
use std::sync::mpsc;
use std::task::Poll;
use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::sync::oneshot;
use tokio::time::timeout;

use super::*;

struct GatedWriter {
    started: Option<oneshot::Sender<()>>,
    release: mpsc::Receiver<()>,
    dropped: Arc<AtomicBool>,
    sizes: Arc<Mutex<Vec<usize>>>,
}

impl Write for GatedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.sizes.lock().unwrap().push(buffer.len());
        if let Some(started) = self.started.take() {
            let _ = started.send(());
            self.release.recv().map_err(std::io::Error::other)?;
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for GatedWriter {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

#[tokio::test]
async fn cancellation_joins_a_running_serializer_before_returning() {
    let control = BrokerWriterControl::new();
    let (started, writing) = oneshot::channel();
    let (release, gate) = mpsc::channel();
    let dropped = Arc::new(AtomicBool::new(false));
    let sizes = Arc::new(Mutex::new(Vec::new()));
    let writer = GatedWriter {
        started: Some(started),
        release: gate,
        dropped: Arc::clone(&dropped),
        sizes: Arc::clone(&sizes),
    };
    let serializing = serialize(
        json!({"history": "x".repeat(17 * 1024 * 1024)}),
        writer,
        &control,
    );
    tokio::pin!(serializing);
    std::future::poll_fn(|cx| {
        assert!(serializing.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    timeout(Duration::from_secs(5), writing)
        .await
        .unwrap()
        .unwrap();
    let writes_before_cancellation = sizes.lock().unwrap().clone();
    control.cancel();
    std::future::poll_fn(|cx| {
        assert!(serializing.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(!dropped.load(Ordering::Acquire));
    release.send(()).unwrap();
    let result = timeout(Duration::from_secs(5), serializing).await.unwrap();
    assert!(result.err().unwrap().is::<WriterCancelled>());
    assert!(dropped.load(Ordering::Acquire));
    assert_eq!(*sizes.lock().unwrap(), writes_before_cancellation);
}

#[test]
fn serializer_bounds_each_blocking_write_without_a_logical_payload_cap() {
    let (_release, gate) = mpsc::channel();
    let sizes = Arc::new(Mutex::new(Vec::new()));
    let recording = GatedWriter {
        started: None,
        release: gate,
        dropped: Arc::new(AtomicBool::new(false)),
        sizes: Arc::clone(&sizes),
    };
    let mut writer = CancelWrite {
        writer: recording,
        stopped: Arc::new(AtomicBool::new(false)),
    };
    let bytes = vec![b'x'; CHECK_BYTES * 3 + 17];
    assert_eq!(writer.write(&bytes).unwrap(), bytes.len());
    assert_eq!(
        *sizes.lock().unwrap(),
        vec![CHECK_BYTES, CHECK_BYTES, CHECK_BYTES, 17]
    );
}

#[tokio::test]
async fn uncapped_spool_round_trips_large_native_history() {
    let control = BrokerWriterControl::new();
    let value = json!({"history": "x".repeat(17 * 1024 * 1024)});
    let payload = spool(value.clone(), &control).await.unwrap();
    assert_eq!(payload.into_value().await.unwrap(), value);
}

#[tokio::test]
async fn cancelled_before_serialization_never_starts_the_blocking_writer() {
    let control = BrokerWriterControl::new();
    control.cancel();
    let (started, mut writing) = oneshot::channel();
    let (_release, gate) = mpsc::channel();
    let dropped = Arc::new(AtomicBool::new(false));
    let sizes = Arc::new(Mutex::new(Vec::new()));
    let writer = GatedWriter {
        started: Some(started),
        release: gate,
        dropped: Arc::clone(&dropped),
        sizes: Arc::clone(&sizes),
    };
    assert!(
        serialize(json!({}), writer, &control)
            .await
            .err()
            .unwrap()
            .is::<WriterCancelled>()
    );
    assert_eq!(
        writing.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    );
    assert!(sizes.lock().unwrap().is_empty());
    assert!(dropped.load(Ordering::Acquire));
}
