//! Owned, cancellation-aware serialization for uncapped persistent message bodies.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use tempfile::NamedTempFile;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::session_wire::Payload;

const CHECK_BYTES: usize = 8192;

#[derive(Debug)]
pub(super) struct WriterCancelled;

impl std::fmt::Display for WriterCancelled {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("dependency writer cancelled")
    }
}

impl std::error::Error for WriterCancelled {}

/// Normal teardown signals this token and joins the writer; it must not abort it.
#[derive(Clone, Debug)]
pub(super) struct BrokerWriterControl {
    cancelled: watch::Sender<bool>,
}

impl BrokerWriterControl {
    pub(super) fn new() -> Self {
        Self {
            cancelled: watch::channel(false).0,
        }
    }

    pub(super) fn cancel(&self) {
        self.cancelled.send_replace(true);
    }

    pub(super) fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow()
    }

    pub(super) async fn cancelled(&self) {
        let mut receiver = self.cancelled.subscribe();
        let _ = receiver.wait_for(|cancelled| *cancelled).await;
    }
}

struct CancelWrite<W> {
    writer: W,
    stopped: Arc<AtomicBool>,
}

impl<W: Write> Write for CancelWrite<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        for chunk in bytes.chunks(CHECK_BYTES) {
            if self.stopped.load(Ordering::Acquire) {
                return Err(std::io::Error::other("dependency serialization cancelled"));
            }
            self.writer.write_all(chunk)?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(std::io::Error::other("dependency serialization cancelled"));
        }
        self.writer.flush()
    }
}

struct SerializeWorker<T> {
    stopped: Arc<AtomicBool>,
    task: JoinHandle<Result<T>>,
}

impl<T> Drop for SerializeWorker<T> {
    fn drop(&mut self) {
        // Forced outer abort cannot join. Signal a running worker and cancel work
        // not yet started; the connection must report cleanup/durability unknown.
        self.stopped.store(true, Ordering::Release);
        self.task.abort();
    }
}

async fn serialize<W: Write + Send + 'static>(
    value: Value,
    writer: W,
    control: &BrokerWriterControl,
) -> Result<W> {
    if control.is_cancelled() {
        return Err(WriterCancelled.into());
    }
    let stopped = Arc::new(AtomicBool::new(false));
    let worker_stopped = Arc::clone(&stopped);
    let mut worker = SerializeWorker {
        stopped,
        task: tokio::task::spawn_blocking(move || {
            let mut writer = CancelWrite {
                writer,
                stopped: worker_stopped,
            };
            // Do not expose serializer diagnostics or the value in a transport error.
            serde_json::to_writer(&mut writer, &value)
                .map_err(|_| anyhow::anyhow!("dependency payload serialization failed"))?;
            writer.flush().context("flush dependency payload spool")?;
            Ok(writer.writer)
        }),
    };
    tokio::select! {
        biased;
        _ = control.cancelled() => {},
        result = &mut worker.task => {
            // A cancellation racing a completed worker must not hide its panic.
            let result = result.context("dependency serializer worker failed")?;
            if control.is_cancelled() {
                return Err(WriterCancelled.into());
            }
            return result;
        }
    }
    worker.stopped.store(true, Ordering::Release);
    worker.task.abort();
    // Started blocking work cannot be aborted. Own and join it before returning.
    match (&mut worker.task).await {
        Ok(_) => {}
        Err(error) if error.is_cancelled() => {} // Confirmed never-started blocking work.
        Err(error) => {
            return Err(error).context("dependency serializer failed during cancellation");
        }
    }
    Err(WriterCancelled.into())
}

pub(super) async fn spool(value: Value, control: &BrokerWriterControl) -> Result<Payload> {
    let file = NamedTempFile::new().context("create dependency payload spool")?;
    let file = serialize(value, file, control).await?;
    let bytes = file
        .as_file()
        .metadata()
        .context("measure dependency payload spool")?
        .len();
    Ok(Payload::from_parts(file, bytes))
}

#[cfg(test)]
#[path = "broker_spool_tests.rs"]
mod tests;
