//! Shared logical-payload accounting. Wrappers retain their own wire schemas.
use super::*;
use crate::payload_work::JsonWorkClient;

struct SeenIds {
    limit: usize,
    maximum: u64,
    missing: BTreeSet<u64>,
}

impl SeenIds {
    fn insert(&mut self, id: u64) -> Result<()> {
        ensure!(id > 0, "session message ID must be positive");
        if id > self.maximum {
            let gap = id - self.maximum - 1;
            ensure!(
                gap <= self.limit as u64 && self.missing.len() + gap as usize <= self.limit,
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
    writer: Option<tokio::fs::File>,
    received: u64,
    chunks: u64,
}

pub(crate) struct MessageAssembler {
    active: BTreeMap<u64, Assembly>,
    seen: SeenIds,
    limit: usize,
    work: Option<JsonWorkClient>,
}

impl MessageAssembler {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            active: BTreeMap::new(),
            seen: SeenIds {
                maximum: 0,
                missing: BTreeSet::new(),
                limit,
            },
            limit,
            work: None,
        }
    }
    pub(crate) fn with_work(limit: usize, work: JsonWorkClient) -> Self {
        Self {
            work: Some(work),
            ..Self::new(limit)
        }
    }
    pub(crate) fn finish(&self) -> Result<()> {
        ensure!(self.active.is_empty(), "truncated session message");
        Ok(())
    }
    pub(crate) fn has_seen(&self, id: u64) -> bool {
        id > 0 && id <= self.seen.maximum && !self.seen.missing.contains(&id)
    }
    pub(crate) fn is_active(&self, id: u64) -> bool {
        self.active.contains_key(&id)
    }
    async fn start(&mut self, id: u64, header: Header, bytes: u64) -> Result<()> {
        ensure!(
            self.active.len() < self.limit,
            "too many concurrent session assemblies"
        );
        self.seen.insert(id)?;
        let payload = match &self.work {
            Some(work) => work.create(bytes).await?,
            None => Payload::from_parts(NamedTempFile::new()?, bytes),
        };
        let writer = if self.work.is_none() {
            Some(tokio::fs::File::from_std(payload.file.reopen()?))
        } else {
            None
        };
        self.active.insert(
            id,
            Assembly {
                header,
                payload,
                writer,
                received: 0,
                chunks: 0,
            },
        );
        Ok(())
    }

    pub(crate) async fn push(&mut self, frame: Frame) -> Result<Option<Incoming>> {
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
                match &self.work {
                    Some(work) => {
                        work.write(&assembly.payload, assembly.received, decoded)
                            .await?
                    }
                    None => {
                        assembly
                            .writer
                            .as_mut()
                            .context("missing assembly writer")?
                            .write_all(&decoded)
                            .await?
                    }
                }
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
                if let Some(mut writer) = assembly.writer.take() {
                    writer.flush().await?;
                }
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
