//! Chunked persistent-session messages with bounded memory and explicit send fences.
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::io::AsyncBufRead;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use crate::session_limits::SessionPayloadLimits;

mod assembly;
mod codec;
use assembly::MessageAssembler;
mod payload;
mod sending;
use codec::Frame;
use codec::read_frame_bytes;
pub(super) use payload::Payload;
use sending::Sending;

pub(super) const MAX_PENDING: usize = 64;
pub(super) const CHUNK_BYTES: usize = 192 * 1024;
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

pub(super) struct Outgoing {
    pub header: Header,
    pub value: Value,
    pub after_sent: Option<Arc<AtomicBool>>,
    pub sent: Option<Arc<AtomicBool>>,
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

pub(super) struct MessageReader<R> {
    reader: R,
    assembler: MessageAssembler,
}

impl<R: AsyncBufRead + Unpin> MessageReader<R> {
    pub(super) fn new(reader: R) -> Self {
        Self {
            reader,
            assembler: MessageAssembler::default(),
        }
    }

    pub(super) fn with_limits(mut self, limits: SessionPayloadLimits) -> Self {
        self.assembler = self.assembler.with_limits(limits);
        self
    }

    pub(super) async fn next(&mut self) -> Result<Option<Incoming>> {
        loop {
            let Some(line) = read_frame_bytes(&mut self.reader).await? else {
                self.assembler.finish()?;
                return Ok(None);
            };
            let frame: Frame = serde_json::from_slice(&line).context("invalid session frame")?;
            if let Some(incoming) = self.assembler.accept(frame).await? {
                return Ok(Some(incoming));
            }
        }
    }
}

/// Multiplex ordinary payloads and bounded controls without overtaking a cleanup fence.
pub(super) async fn write_messages<W: AsyncWrite + Unpin>(
    mut writer: W,
    mut regular: mpsc::Receiver<Outgoing>,
    mut control: mpsc::Receiver<Outgoing>,
    limits: SessionPayloadLimits,
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
            let mut sending = Sending::new(outgoing, /*control*/ true, limits).await?;
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
                Some(message) => active = Some(Sending::new(message, /*control*/ false, limits).await?),
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

#[cfg(test)]
#[path = "session_wire/raw_tests.rs"]
mod raw_tests;
