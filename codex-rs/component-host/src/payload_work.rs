//! Invocation-owned JSON and spool work. A dropped waiter never owns a worker.

use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde_json::Value;
use tempfile::NamedTempFile;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::session_wire::Payload;

const CHECK_BYTES: usize = 8192;

#[derive(Debug)]
struct WorkCancelled;

impl std::fmt::Display for WorkCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("component payload work cancelled")
    }
}

impl std::error::Error for WorkCancelled {}

struct CheckedIo<T> {
    inner: T,
    stopped: Arc<AtomicBool>,
    cancellation_observed: bool,
}

impl<T> CheckedIo<T> {
    fn check(&mut self) -> std::io::Result<()> {
        if self.stopped.load(Ordering::Acquire) {
            self.cancellation_observed = true;
            return Err(std::io::Error::other(WorkCancelled));
        }
        Ok(())
    }
}

impl<T: Read> Read for CheckedIo<T> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.check()?;
        let count = buffer.len().min(CHECK_BYTES);
        self.inner.read(&mut buffer[..count])
    }
}

impl<T: Write> Write for CheckedIo<T> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.check()?;
        self.inner.write(&bytes[..bytes.len().min(CHECK_BYTES)])
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.check()?;
        self.inner.flush()
    }
}

enum Work {
    Create(u64),
    Serialize(Value),
    Decode(Payload),
    Read {
        file: File,
        offset: u64,
        count: usize,
    },
    Write {
        file: File,
        offset: u64,
        bytes: Vec<u8>,
    },
}

enum Output {
    Payload(Payload),
    Value(Value),
    Bytes(Vec<u8>),
    Written,
}

struct Job {
    work: Work,
    result: oneshot::Sender<Result<Output>>,
}

fn execute(work: Work, stopped: Arc<AtomicBool>) -> Result<Output> {
    if stopped.load(Ordering::Acquire) {
        return Err(WorkCancelled.into());
    }
    match work {
        Work::Create(bytes) => Ok(Output::Payload(Payload::from_parts(
            NamedTempFile::new().context("create component payload spool")?,
            bytes,
        ))),
        Work::Serialize(value) => {
            let mut writer = CheckedIo {
                inner: std::io::BufWriter::with_capacity(
                    CHECK_BYTES,
                    NamedTempFile::new().context("create component payload spool")?,
                ),
                stopped,
                cancellation_observed: false,
            };
            let serialized = serde_json::to_writer(&mut writer, &value);
            if writer.cancellation_observed {
                return Err(WorkCancelled.into());
            }
            serialized.map_err(|_| anyhow::anyhow!("serialize component payload failed"))?;
            writer.flush().context("flush component payload spool")?;
            let file = writer
                .inner
                .into_inner()
                .map_err(|_| anyhow::anyhow!("flush component payload spool failed"))?;
            let bytes = file.as_file().metadata()?.len();
            Ok(Output::Payload(Payload::from_parts(file, bytes)))
        }
        Work::Decode(payload) => {
            let (file, _) = payload.into_parts();
            let mut reader = std::io::BufReader::with_capacity(
                CHECK_BYTES,
                CheckedIo {
                    inner: file.reopen().context("open component payload spool")?,
                    stopped,
                    cancellation_observed: false,
                },
            );
            let decoded = serde_json::from_reader(&mut reader);
            if reader.get_ref().cancellation_observed {
                return Err(WorkCancelled.into());
            }
            // Keep the private spool owned until parsing (including failure) ends.
            drop(file);
            Ok(Output::Value(decoded.map_err(|_| {
                anyhow::anyhow!("invalid component logical JSON payload")
            })?))
        }
        Work::Read {
            mut file,
            offset,
            count,
        } => {
            file.seek(SeekFrom::Start(offset))?;
            let mut reader = CheckedIo {
                inner: file,
                stopped,
                cancellation_observed: false,
            };
            let mut bytes = vec![0; count];
            let mut filled = 0;
            while filled < count {
                let count = reader.read(&mut bytes[filled..])?;
                if count == 0 {
                    break;
                }
                filled += count;
            }
            bytes.truncate(filled);
            Ok(Output::Bytes(bytes))
        }
        Work::Write {
            mut file,
            offset,
            bytes,
        } => {
            file.seek(SeekFrom::Start(offset))?;
            let mut writer = CheckedIo {
                inner: file,
                stopped,
                cancellation_observed: false,
            };
            writer.write_all(&bytes)?;
            writer.flush()?;
            Ok(Output::Written)
        }
    }
}

/// Owns one controller and at most one blocking operation, including spool IO.
pub(crate) struct JsonWorkOwner {
    stopped: Arc<AtomicBool>,
    stop: watch::Sender<bool>,
    task: Option<JoinHandle<Result<()>>>,
}

#[derive(Clone)]
pub(crate) struct JsonWorkClient {
    stopped: Arc<AtomicBool>,
    jobs: mpsc::Sender<Job>,
}

impl JsonWorkOwner {
    pub(crate) fn start() -> (Self, JsonWorkClient) {
        let stopped = Arc::new(AtomicBool::new(false));
        let (stop, receiver) = watch::channel(false);
        let (jobs, incoming) = mpsc::channel(1);
        let task = tokio::spawn(run(incoming, receiver, Arc::clone(&stopped)));
        let client = JsonWorkClient {
            stopped: Arc::clone(&stopped),
            jobs,
        };
        (
            Self {
                stopped,
                stop,
                task: Some(task),
            },
            client,
        )
    }

    pub(crate) async fn stop_and_join(&mut self) -> Result<()> {
        self.stopped.store(true, Ordering::Release);
        self.stop.send_replace(true);
        if let Some(task) = self.task.as_mut() {
            let result = task.await.context("component payload controller failed")?;
            self.task.take();
            result?;
        }
        Ok(())
    }
}

impl Drop for JsonWorkOwner {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        self.stop.send_replace(true);
        // Do not abort the controller: it retains and joins its blocking handle.
        // Destroying the runtime can still prevent an acknowledgement.
    }
}

async fn run(
    mut jobs: mpsc::Receiver<Job>,
    mut stop: watch::Receiver<bool>,
    stopped: Arc<AtomicBool>,
) -> Result<()> {
    loop {
        let job = tokio::select! {
            biased;
            _ = stop.wait_for(|value| *value) => break,
            job = jobs.recv() => match job { Some(job) => job, None => break },
        };
        if job.result.is_closed() {
            continue;
        }
        let worker_stopped = Arc::clone(&stopped);
        let mut task = tokio::task::spawn_blocking(move || execute(job.work, worker_stopped));
        let result = tokio::select! {
            biased;
            _ = stop.wait_for(|value| *value) => {
                stopped.store(true, Ordering::Release);
                task.abort(); // Cancels only work which has not started.
                match task.await {
                    Ok(result) => result,
                    Err(error) if error.is_cancelled() => Err(WorkCancelled.into()),
                    Err(error) => Err(error).context("component payload worker failed"),
                }
            },
            result = &mut task => result.context("component payload worker failed")?,
        };
        if let Err(result) = job.result.send(result)
            && let Err(error) = result
            && !error.chain().any(|cause| cause.is::<WorkCancelled>())
        {
            return Err(error).context("abandoned component payload work failed");
        }
        if stopped.load(Ordering::Acquire) {
            break;
        }
    }
    jobs.close();
    while let Ok(job) = jobs.try_recv() {
        let _ = job.result.send(Err(WorkCancelled.into()));
    }
    Ok(())
}

impl JsonWorkClient {
    async fn submit(&self, work: Work) -> Result<Output> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(WorkCancelled.into());
        }
        let permit = self
            .jobs
            .reserve()
            .await
            .context("component payload worker closed")?;
        if self.stopped.load(Ordering::Acquire) {
            return Err(WorkCancelled.into());
        }
        let (result, received) = oneshot::channel();
        permit.send(Job { work, result });
        received
            .await
            .context("component payload worker lost result")?
    }

    pub(crate) async fn create(&self, bytes: u64) -> Result<Payload> {
        match self.submit(Work::Create(bytes)).await? {
            Output::Payload(payload) => Ok(payload),
            _ => bail!("invalid payload worker reply"),
        }
    }

    pub(crate) async fn serialize(&self, value: Value) -> Result<Payload> {
        match self.submit(Work::Serialize(value)).await? {
            Output::Payload(payload) => Ok(payload),
            _ => bail!("invalid payload worker reply"),
        }
    }

    pub(crate) async fn decode(&self, payload: Payload) -> Result<Value> {
        match self.submit(Work::Decode(payload)).await? {
            Output::Value(value) => Ok(value),
            _ => bail!("invalid payload worker reply"),
        }
    }

    pub(crate) async fn read(
        &self,
        payload: &Payload,
        offset: u64,
        count: usize,
    ) -> Result<Vec<u8>> {
        match self
            .submit(Work::Read {
                file: payload.handle()?,
                offset,
                count,
            })
            .await?
        {
            Output::Bytes(bytes) => Ok(bytes),
            _ => bail!("invalid payload worker reply"),
        }
    }

    pub(crate) async fn write(&self, payload: &Payload, offset: u64, bytes: Vec<u8>) -> Result<()> {
        match self
            .submit(Work::Write {
                file: payload.handle()?,
                offset,
                bytes,
            })
            .await?
        {
            Output::Written => Ok(()),
            _ => bail!("invalid payload worker reply"),
        }
    }
}
