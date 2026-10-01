//! One outgoing message's spool and physical-frame progress. The parent owns
//! queue arbitration; completion publishes its send fence only after flush.
use super::CHUNK_BYTES;
use super::Header;
use super::MAX_FRAME_BYTES;
use super::Outgoing;
use super::Payload;
use super::codec::Frame;
use super::codec::write_frame;
use super::validate_component;
use crate::session_limits::CONTROL_BYTES as MAX_CONTROL_BYTES;
use crate::session_limits::SessionPayloadLimits;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::sync::atomic::Ordering;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

pub(super) struct Sending {
    outgoing: Outgoing,
    payload: Option<Payload>,
    reader: Option<tokio::fs::File>,
    started: bool,
    chunks: u64,
    transferred: u64,
}

impl Sending {
    pub(super) async fn new(
        mut outgoing: Outgoing,
        control: bool,
        limits: SessionPayloadLimits,
    ) -> Result<Self> {
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
        // The public admission/respond paths reject local overflow while the
        // connection is still healthy. Repeat the check before creating a spool
        // so an internal caller cannot bypass the configured body bound.
        match &outgoing.header {
            Header::Request { .. } if control => {
                SessionPayloadLimits::check_control(&outgoing.value)?;
            }
            Header::Request { .. } => limits.check_request(&outgoing.value)?,
            Header::Result { .. } => limits.check_reply(&outgoing.value, control)?,
            Header::Error { .. } | Header::Shutdown | Header::ShutdownComplete => {}
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
    pub(super) async fn step<W: AsyncWrite + Unpin>(&mut self, writer: &mut W) -> Result<bool> {
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
