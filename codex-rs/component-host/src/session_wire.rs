//! Chunked persistent-session messages with bounded memory and explicit send fences.
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tempfile::NamedTempFile;
use tokio::io::AsyncBufRead;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

pub(super) const MAX_PENDING: usize = 64;
pub(super) const CHUNK_BYTES: usize = 192 * 1024;
const MAX_CONTROL_BYTES: u64 = 64 * 1024;
const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// A language-neutral component identity addressed within a persistent session.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionComponent {
    pub kind: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub(super) enum Header {
    Request {
        id: u64,
        component: SessionComponent,
        method: String,
        is_control: bool,
    },
    Result {
        id: u64,
    },
    Error {
        id: u64,
        message: String,
    },
    Shutdown,
    ShutdownComplete,
}

pub(super) struct Incoming {
    pub header: Header,
    pub payload: Option<Payload>,
}

/// Private temporary storage avoids a logical JSON-size cap on native histories.
pub(super) struct Payload {
    file: NamedTempFile,
    bytes: u64,
}

impl Payload {
    pub(super) async fn from_value(value: Value) -> Result<Self> {
        tokio::task::spawn_blocking(move || {
            let mut file = NamedTempFile::new()?;
            serde_json::to_writer(&mut file, &value)?;
            let bytes = file.as_file().metadata()?.len();
            Ok(Self { file, bytes })
        })
        .await
        .context("serialize session payload task")?
    }

    pub(super) async fn into_value(self) -> Result<Value> {
        tokio::task::spawn_blocking(move || {
            serde_json::from_reader(self.file.reopen()?).context("decode session payload")
        })
        .await
        .context("deserialize session payload task")?
    }
}

pub(super) struct Outgoing {
    pub header: Header,
    pub value: Value,
    pub after_sent: Option<Arc<AtomicBool>>,
    pub sent: Option<Arc<AtomicBool>>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Frame {
    RequestStart {
        id: u64,
        component: SessionComponent,
        method: String,
        is_control: bool,
        bytes: u64,
    },
    ResultStart {
        id: u64,
        bytes: u64,
    },
    Chunk {
        id: u64,
        index: u64,
        data: String,
    },
    End {
        id: u64,
        chunks: u64,
    },
    Error {
        id: u64,
        message: String,
    },
    Shutdown,
    ShutdownComplete,
}

fn validate_component(component: &SessionComponent, method: &str) -> Result<()> {
    ensure!(
        !component.kind.is_empty() && component.kind.len() <= 128,
        "invalid session component kind"
    );
    ensure!(
        !component.name.is_empty() && component.name.len() <= 128,
        "invalid session component name"
    );
    ensure!(
        !method.is_empty() && method.len() <= 256,
        "invalid session method"
    );
    Ok(())
}

#[derive(Default)]
struct SeenIds {
    maximum: u64,
    missing: BTreeSet<u64>,
}

impl SeenIds {
    fn insert(&mut self, id: u64) -> Result<()> {
        ensure!(id > 0, "session message ID must be positive");
        if id > self.maximum {
            let gap = id - self.maximum - 1;
            ensure!(
                gap <= MAX_PENDING as u64 && self.missing.len() + gap as usize <= MAX_PENDING,
                "too many unseen session message IDs"
            );
            self.missing.extend(self.maximum + 1..id);
            self.maximum = id;
        } else {
            ensure!(self.missing.remove(&id), "duplicate session message ID");
        }
        Ok(())
    }
}

struct Assembly {
    header: Header,
    payload: Payload,
    writer: tokio::fs::File,
    received: u64,
    chunks: u64,
}

pub(super) struct MessageReader<R> {
    reader: R,
    active: BTreeMap<u64, Assembly>,
    seen: SeenIds,
}

impl<R: AsyncBufRead + Unpin> MessageReader<R> {
    pub(super) fn new(reader: R) -> Self {
        Self {
            reader,
            active: BTreeMap::new(),
            seen: SeenIds::default(),
        }
    }

    async fn start(&mut self, id: u64, header: Header, bytes: u64) -> Result<()> {
        ensure!(
            self.active.len() < MAX_PENDING,
            "too many concurrent session assemblies"
        );
        self.seen.insert(id)?;
        let file = NamedTempFile::new()?;
        let writer = tokio::fs::File::from_std(file.reopen()?);
        self.active.insert(
            id,
            Assembly {
                header,
                payload: Payload { file, bytes },
                writer,
                received: 0,
                chunks: 0,
            },
        );
        Ok(())
    }

    pub(super) async fn next(&mut self) -> Result<Option<Incoming>> {
        loop {
            // fill_buf/consume keeps even a malicious newline-free input bounded.
            let mut line = Vec::new();
            loop {
                let buffer = self.reader.fill_buf().await?;
                if buffer.is_empty() {
                    ensure!(
                        line.is_empty() && self.active.is_empty(),
                        "truncated session message"
                    );
                    return Ok(None);
                }
                let length = buffer
                    .iter()
                    .position(|byte| *byte == b'\n')
                    .map_or(buffer.len(), |index| index + 1);
                ensure!(
                    line.len() + length <= MAX_FRAME_BYTES,
                    "session frame exceeds wire limit"
                );
                let complete = buffer[length - 1] == b'\n';
                line.extend_from_slice(&buffer[..length]);
                self.reader.consume(length);
                if complete {
                    break;
                }
            }
            let frame: Frame = serde_json::from_slice(&line).context("invalid session frame")?;
            match frame {
                Frame::RequestStart {
                    id,
                    component,
                    method,
                    is_control,
                    bytes,
                } => {
                    validate_component(&component, &method)?;
                    ensure!(
                        !is_control || bytes <= MAX_CONTROL_BYTES,
                        "session control payload exceeds limit"
                    );
                    self.start(
                        id,
                        Header::Request {
                            id,
                            component,
                            method,
                            is_control,
                        },
                        bytes,
                    )
                    .await?;
                }
                Frame::ResultStart { id, bytes } => {
                    self.start(id, Header::Result { id }, bytes).await?
                }
                Frame::Chunk { id, index, data } => {
                    ensure!(
                        data.len() <= CHUNK_BYTES.div_ceil(3) * 4,
                        "session chunk exceeds limit"
                    );
                    let decoded = STANDARD
                        .decode(data)
                        .context("invalid session chunk encoding")?;
                    ensure!(
                        !decoded.is_empty() && decoded.len() <= CHUNK_BYTES,
                        "invalid session chunk size"
                    );
                    let assembly = self
                        .active
                        .get_mut(&id)
                        .context("session chunk has no start")?;
                    ensure!(
                        index == assembly.chunks,
                        "session chunk index is not contiguous"
                    );
                    let received = assembly
                        .received
                        .checked_add(decoded.len() as u64)
                        .context("session payload byte overflow")?;
                    ensure!(
                        received <= assembly.payload.bytes,
                        "session payload exceeds declared length"
                    );
                    assembly.writer.write_all(&decoded).await?;
                    assembly.received = received;
                    assembly.chunks = assembly
                        .chunks
                        .checked_add(1)
                        .context("session chunk count overflow")?;
                }
                Frame::End { id, chunks } => {
                    let mut assembly = self
                        .active
                        .remove(&id)
                        .context("session end has no start")?;
                    ensure!(
                        assembly.received == assembly.payload.bytes && chunks == assembly.chunks,
                        "session end length or chunk count mismatch"
                    );
                    assembly.writer.flush().await?;
                    drop(assembly.writer);
                    return Ok(Some(Incoming {
                        header: assembly.header,
                        payload: Some(assembly.payload),
                    }));
                }
                Frame::Error { id, message } => {
                    if let Some(assembly) = self.active.remove(&id) {
                        ensure!(
                            matches!(assembly.header, Header::Result { .. }),
                            "error cannot replace a request assembly"
                        );
                    } else {
                        self.seen.insert(id)?;
                    }
                    return Ok(Some(Incoming {
                        header: Header::Error { id, message },
                        payload: None,
                    }));
                }
                Frame::Shutdown | Frame::ShutdownComplete => {
                    ensure!(
                        self.active.is_empty(),
                        "shutdown interrupted a session assembly"
                    );
                    let header = if matches!(frame, Frame::Shutdown) {
                        Header::Shutdown
                    } else {
                        Header::ShutdownComplete
                    };
                    return Ok(Some(Incoming {
                        header,
                        payload: None,
                    }));
                }
            }
        }
    }
}

async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, frame: Frame) -> Result<()> {
    let bytes = serde_json::to_vec(&frame)?;
    ensure!(
        bytes.len() < MAX_FRAME_BYTES,
        "session frame exceeds wire limit"
    );
    writer.write_all(&bytes).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}

struct Sending {
    outgoing: Outgoing,
    payload: Option<Payload>,
    reader: Option<tokio::fs::File>,
    started: bool,
    chunks: u64,
    transferred: u64,
}

impl Sending {
    async fn new(mut outgoing: Outgoing, control: bool) -> Result<Self> {
        if let Header::Request {
            component, method, ..
        } = &outgoing.header
        {
            validate_component(component, method)?;
        }
        let control = control
            || matches!(
                outgoing.header,
                Header::Request {
                    is_control: true,
                    ..
                }
            );
        match &outgoing.header {
            Header::Request { id, .. } | Header::Result { id } | Header::Error { id, .. } => {
                ensure!(*id > 0, "session message ID must be positive")
            }
            Header::Shutdown | Header::ShutdownComplete => {}
        }
        if let Header::Error { message, .. } = &outgoing.header {
            ensure!(
                message.len() < MAX_FRAME_BYTES,
                "session error exceeds wire limit"
            );
        }
        let payload = if matches!(
            outgoing.header,
            Header::Request { .. } | Header::Result { .. }
        ) {
            let payload = Payload::from_value(std::mem::take(&mut outgoing.value)).await?;
            ensure!(
                !control || payload.bytes <= MAX_CONTROL_BYTES,
                "session control payload exceeds limit"
            );
            Some(payload)
        } else {
            None
        };
        let reader = payload
            .as_ref()
            .map(|payload| payload.file.reopen().map(tokio::fs::File::from_std))
            .transpose()?;
        Ok(Self {
            outgoing,
            payload,
            reader,
            started: false,
            chunks: 0,
            transferred: 0,
        })
    }

    // One step is at most one physical frame, allowing urgent controls between chunks.
    async fn step<W: AsyncWrite + Unpin>(&mut self, writer: &mut W) -> Result<bool> {
        let id = match &self.outgoing.header {
            Header::Request { id, .. } | Header::Result { id } | Header::Error { id, .. } => *id,
            Header::Shutdown | Header::ShutdownComplete => 0,
        };
        let complete;
        if !self.started {
            let bytes = self.payload.as_ref().map_or(0, |payload| payload.bytes);
            let frame = match &self.outgoing.header {
                Header::Request {
                    id,
                    component,
                    method,
                    is_control,
                } => Frame::RequestStart {
                    id: *id,
                    component: component.clone(),
                    method: method.clone(),
                    is_control: *is_control,
                    bytes,
                },
                Header::Result { id } => Frame::ResultStart { id: *id, bytes },
                Header::Error { id, message } => Frame::Error {
                    id: *id,
                    message: message.clone(),
                },
                Header::Shutdown => Frame::Shutdown,
                Header::ShutdownComplete => Frame::ShutdownComplete,
            };
            write_frame(writer, frame).await?;
            self.started = true;
            complete = self.payload.is_none();
        } else if self.transferred
            < self
                .payload
                .as_ref()
                .context("missing outgoing payload")?
                .bytes
        {
            let mut chunk = vec![0; CHUNK_BYTES];
            let bytes = self
                .reader
                .as_mut()
                .context("missing spool reader")?
                .read(&mut chunk)
                .await?;
            ensure!(bytes > 0, "truncated outgoing payload spool");
            chunk.truncate(bytes);
            write_frame(
                writer,
                Frame::Chunk {
                    id,
                    index: self.chunks,
                    data: STANDARD.encode(chunk),
                },
            )
            .await?;
            self.transferred += bytes as u64;
            self.chunks += 1;
            complete = false;
        } else {
            write_frame(
                writer,
                Frame::End {
                    id,
                    chunks: self.chunks,
                },
            )
            .await?;
            complete = true;
        }
        if complete {
            writer.flush().await?;
            if let Some(sent) = &self.outgoing.sent {
                sent.store(true, Ordering::Release);
            }
        }
        Ok(complete)
    }
}

/// Multiplex ordinary payloads and bounded controls without overtaking a cleanup fence.
pub(super) async fn write_messages<W: AsyncWrite + Unpin>(
    mut writer: W,
    mut regular: mpsc::Receiver<Outgoing>,
    mut control: mpsc::Receiver<Outgoing>,
) -> Result<()> {
    let mut regular_open = true;
    let mut control_open = true;
    let mut active: Option<Sending> = None;
    let mut deferred = VecDeque::<Outgoing>::new();
    loop {
        while control_open && deferred.len() < MAX_PENDING {
            match control.try_recv() {
                Ok(message) => deferred.push_back(message),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => control_open = false,
            }
        }
        if let Some(position) = deferred.iter().position(|message| {
            message
                .after_sent
                .as_ref()
                .is_none_or(|sent| sent.load(Ordering::Acquire))
                && (active.is_none()
                    || !matches!(message.header, Header::Shutdown | Header::ShutdownComplete))
        }) {
            let outgoing = deferred
                .remove(position)
                .context("missing queued control")?;
            let mut sending = Sending::new(outgoing, /*control*/ true).await?;
            while !sending.step(&mut writer).await? {}
            continue;
        }
        if let Some(sending) = &mut active {
            if sending.step(&mut writer).await? {
                active = None;
            }
            continue;
        }
        tokio::select! {
            biased;
            message = control.recv(), if control_open && deferred.len() < MAX_PENDING => match message {
                Some(message) => deferred.push_back(message),
                None => control_open = false,
            },
            message = regular.recv(), if regular_open => match message {
                Some(message) => active = Some(Sending::new(message, /*control*/ false).await?),
                None => regular_open = false,
            },
            else => {
                ensure!(deferred.is_empty(), "session cleanup depends on a request that was never sent");
                writer.shutdown().await?;
                return Ok(());
            }
        }
    }
}

#[cfg(test)]
#[path = "session_wire_tests.rs"]
mod tests;
