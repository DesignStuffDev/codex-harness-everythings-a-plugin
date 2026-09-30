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
    pub(crate) fn from_parts(file: NamedTempFile, bytes: u64) -> Self {
        Self { file, bytes }
    }
    pub(crate) fn into_parts(self) -> (NamedTempFile, u64) {
        (self.file, self.bytes)
    }
    pub(crate) fn handle(&self) -> std::io::Result<std::fs::File> {
        self.file.as_file().try_clone()
    }

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
pub(crate) enum Frame {
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

#[path = "wire_assembly.rs"]
mod assembly;
pub(crate) use assembly::MessageAssembler;

pub(super) struct MessageReader<R> {
    reader: R,
    assembly: MessageAssembler,
}

impl<R: AsyncBufRead + Unpin> MessageReader<R> {
    pub(super) fn new(reader: R) -> Self {
        Self {
            reader,
            assembly: MessageAssembler::new(MAX_PENDING),
        }
    }

    pub(super) async fn next(&mut self) -> Result<Option<Incoming>> {
        while let Some(bytes) = read_frame_bytes(&mut self.reader).await? {
            let frame: Frame = serde_json::from_slice(&bytes).context("invalid session frame")?;
            if let Some(message) = self.assembly.push(frame).await? {
                return Ok(Some(message));
            }
        }
        self.assembly.finish()?;
        Ok(None)
    }
}

/// A single owner must retain this future until a complete line is received.
pub(crate) async fn read_frame_bytes<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let buffer = reader.fill_buf().await?;
        if buffer.is_empty() {
            ensure!(line.is_empty(), "truncated session frame");
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
        reader.consume(length);
        if complete {
            return Ok(Some(line));
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

#[path = "wire_sending.rs"]
mod sending;
pub(crate) use sending::Sending;

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
