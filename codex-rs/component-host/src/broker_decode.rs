//! Reverse request parsing is cancellable and remains owned until its worker joins.

use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use serde_json::Value;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::broker_api::DependencyContext;
use crate::broker_api::DependencyError;
use crate::broker_api::DependencyErrorCode;
use crate::broker_api::MAX_BROKER_REQUEST_BYTES;
use crate::session_wire::Payload;

pub(super) async fn decode(
    payload: Payload,
    context: &DependencyContext,
    cancelled: watch::Receiver<bool>,
) -> Result<Value, DependencyError> {
    let (file, bytes) = payload.into_parts();
    if bytes > MAX_BROKER_REQUEST_BYTES {
        return Err(DependencyError::new(DependencyErrorCode::RequestTooLarge));
    }
    // NamedTempFile owns the private spool until the parser (including any
    // cancellation path) has joined. No detached into_value task is used here.
    let reader = file
        .reopen()
        .map_err(|_| DependencyError::new(DependencyErrorCode::ServiceFailure))?;
    decode_reader(
        SpoolReader {
            reader,
            _file: file,
        },
        context,
        cancelled,
    )
    .await
}

struct SpoolReader {
    reader: File,
    _file: tempfile::NamedTempFile,
}

impl Read for SpoolReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buffer)
    }
}

struct CancelRead<R> {
    reader: R,
    stopped: Arc<AtomicBool>,
}

impl<R: Read> Read for CancelRead<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(std::io::Error::other("dependency decoding cancelled"));
        }
        let count = self.reader.read(buffer)?;
        if self.stopped.load(Ordering::Acquire) {
            return Err(std::io::Error::other("dependency decoding cancelled"));
        }
        Ok(count)
    }
}

struct DecodeWorker {
    stopped: Arc<AtomicBool>,
    task: JoinHandle<Result<Value, serde_json::Error>>,
}

impl Drop for DecodeWorker {
    fn drop(&mut self) {
        // An abnormal outer runtime abort still stops a running read and removes
        // queued blocking work. Normal cancellation always awaits its join below.
        self.stopped.store(true, Ordering::Release);
        self.task.abort();
    }
}

async fn decode_reader<R: Read + Send + 'static>(
    reader: R,
    context: &DependencyContext,
    mut cancelled: watch::Receiver<bool>,
) -> Result<Value, DependencyError> {
    context.check_authority()?;
    if *cancelled.borrow() {
        return Err(DependencyError::new(DependencyErrorCode::Cancelled));
    }
    let stopped = Arc::new(AtomicBool::new(false));
    let worker_stopped = Arc::clone(&stopped);
    let mut worker = DecodeWorker {
        stopped,
        task: tokio::task::spawn_blocking(move || {
            serde_json::from_reader(BufReader::with_capacity(
                8192,
                CancelRead {
                    reader,
                    stopped: worker_stopped,
                },
            ))
        }),
    };
    let cancellation = tokio::select! {
        biased;
        _ = cancelled.wait_for(|cancelled| *cancelled) =>
            DependencyError::new(DependencyErrorCode::Cancelled),
        _ = context.cancelled() => context.check_authority().err()
            .unwrap_or_else(|| DependencyError::new(DependencyErrorCode::AuthorityRevoked)),
        result = &mut worker.task => {
            context.check_authority()?;
            return result.map_err(|_| DependencyError::new(DependencyErrorCode::ServiceFailure))?
                .map_err(|_| DependencyError::new(DependencyErrorCode::InvalidRequest));
        }
    };
    worker.stopped.store(true, Ordering::Release);
    worker.task.abort(); // Cancels queued work; a running parser observes stopped.
    let _ = (&mut worker.task).await;
    Err(cancellation)
}

#[cfg(test)]
#[path = "broker_decode_tests.rs"]
mod tests;
