//! Bounded logical message assembly and monotonic-ID duplicate tracking.
//! Declared request/reply limits are checked before any temporary spool exists.
use super::CHUNK_BYTES;
use super::Header;
use super::Incoming;
use super::MAX_PENDING;
use super::Payload;
use super::codec::Frame;
use super::validate_component;
use crate::session_limits::CONTROL_BYTES as MAX_CONTROL_BYTES;
use crate::session_limits::PayloadKind;
use crate::session_limits::SessionPayloadLimits;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;

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

#[derive(Default)]
pub(super) struct MessageAssembler {
    active: BTreeMap<u64, Assembly>,
    seen: SeenIds,
    limits: SessionPayloadLimits,
}

impl MessageAssembler {
    pub(super) fn with_limits(mut self, limits: SessionPayloadLimits) -> Self {
        self.limits = limits;
        self
    }

    /// EOF is valid only after every admitted assembly has terminated.
    pub(super) fn finish(&self) -> Result<()> {
        ensure!(self.active.is_empty(), "truncated session message");
        Ok(())
    }

    async fn start(&mut self, id: u64, header: Header, bytes: u64) -> Result<()> {
        match &header {
            Header::Request {
                is_control: false, ..
            } => {
                self.limits.check_incoming(PayloadKind::Request, bytes)?;
            }
            Header::Result { .. } => {
                self.limits.check_incoming(PayloadKind::Reply, bytes)?;
            }
            Header::Request {
                is_control: true, ..
            }
            | Header::Error { .. }
            | Header::Shutdown
            | Header::ShutdownComplete => {}
        }
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

    pub(super) async fn accept(&mut self, frame: Frame) -> Result<Option<Incoming>> {
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
        Ok(None)
    }
}
