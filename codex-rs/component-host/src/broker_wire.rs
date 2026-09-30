//! Negotiated dependency-call framing over the persistent session's shared spool codec.
//!
//! One owned reader demultiplexes namespaces. The payload implementation and
//! assembly validation are shared with ordinary calls; only bounded headers live here.

use std::collections::BTreeMap;

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::io::AsyncBufRead;

use crate::broker_api::DependencyError;
use crate::broker_api::DependencyErrorCode;
use crate::broker_api::DependencyScope;
use crate::session_wire::Frame;
use crate::session_wire::Header;
use crate::session_wire::Incoming;
use crate::session_wire::MAX_PENDING;
use crate::session_wire::MessageAssembler;
use crate::session_wire::Payload;
use crate::session_wire::SessionComponent;
use crate::session_wire::read_frame_bytes;

pub(super) const MAX_BROKER_PENDING: usize = crate::broker_api::BROKER_CAPACITY;
const MAX_HANDLE_BYTES: usize = 256;

/// Routing and authority metadata; opaque handles are deliberately not Debug.
#[derive(Clone)]
pub(super) struct BrokerRequestHeader {
    pub id: u64,
    pub service: String,
    pub version: u32,
    pub method: String,
    pub authority: String,
    pub context: String,
    pub parent_id: u64,
    pub request_generation: u64,
    pub expected_owner_generation: u64,
}

pub(super) enum BrokerIncoming {
    Ordinary {
        message: Incoming,
        scope: Option<DependencyScope>,
    },
    Request {
        header: BrokerRequestHeader,
        payload: Payload,
    },
    Result {
        id: u64,
        payload: Payload,
    },
    Error {
        id: u64,
        error: DependencyError,
    },
    Cancel {
        id: u64,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum BrokerFrame {
    BrokerRequestStart {
        id: u64,
        service: String,
        version: u32,
        method: String,
        authority: String,
        context: String,
        parent_id: u64,
        request_generation: u64,
        expected_owner_generation: u64,
        bytes: u64,
    },
    BrokerResultStart {
        id: u64,
        bytes: u64,
    },
    BrokerChunk {
        id: u64,
        index: u64,
        data: String,
    },
    BrokerEnd {
        id: u64,
        chunks: u64,
    },
    BrokerError {
        id: u64,
        code: DependencyErrorCode,
        message: String,
    },
    BrokerCancel {
        id: u64,
    },
}

#[derive(Deserialize)]
struct FrameType {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopedRequestStart {
    #[serde(rename = "type")]
    _kind: String,
    id: u64,
    component: SessionComponent,
    method: String,
    is_control: bool,
    bytes: u64,
    broker_context: Option<DependencyScope>,
}

fn validate_handle(handle: &str) -> Result<()> {
    ensure!(
        !handle.is_empty() && handle.len() <= MAX_HANDLE_BYTES,
        "invalid dependency authority or operation handle"
    );
    Ok(())
}

fn validate_header(header: &BrokerRequestHeader) -> Result<()> {
    ensure!(header.id > 0 && header.parent_id > 0, "invalid dependency request ID");
    ensure!(header.version > 0, "invalid dependency service version");
    ensure!(
        !header.service.is_empty() && header.service.len() <= 128,
        "invalid dependency service name"
    );
    ensure!(
        !header.method.is_empty() && header.method.len() <= 256,
        "invalid dependency method"
    );
    validate_handle(&header.authority)?;
    validate_handle(&header.context)
}

/// Used only by an owned reader task; cancelling an in-progress `next` discards
/// parsing state. Public client/server waits instead receive through a channel.
pub(super) struct BrokerMessageReader<R> {
    reader: R,
    enabled: bool,
    ordinary: MessageAssembler,
    broker: MessageAssembler,
    scopes: BTreeMap<u64, DependencyScope>,
    requests: BTreeMap<u64, BrokerRequestHeader>,
}

impl<R: AsyncBufRead + Unpin> BrokerMessageReader<R> {
    pub(super) fn new(reader: R, enabled: bool) -> Self {
        Self {
            reader,
            enabled,
            ordinary: MessageAssembler::new(MAX_PENDING),
            broker: MessageAssembler::new(MAX_BROKER_PENDING),
            scopes: BTreeMap::new(),
            requests: BTreeMap::new(),
        }
    }

    pub(super) async fn next(&mut self) -> Result<Option<BrokerIncoming>> {
        loop {
            let Some(bytes) = read_frame_bytes(&mut self.reader).await? else {
                self.ordinary.finish()?;
                self.broker.finish()?;
                return Ok(None);
            };
            let tag: FrameType = serde_json::from_slice(&bytes)
                .map_err(|_| anyhow::anyhow!("invalid session frame type"))?;
            if tag.kind.starts_with("broker_") {
                ensure!(self.enabled, "dependency broker was not negotiated");
                let frame: BrokerFrame = serde_json::from_slice(&bytes)
                    .map_err(|_| anyhow::anyhow!("invalid dependency broker frame"))?;
                if let Some(message) = self.broker_frame(frame).await? {
                    return Ok(Some(message));
                }
                continue;
            }
            let (frame, scope) = if self.enabled && tag.kind == "request_start" {
                let start: ScopedRequestStart = serde_json::from_slice(&bytes)
                    .map_err(|_| anyhow::anyhow!("invalid scoped session request"))?;
                (Frame::RequestStart { id: start.id, component: start.component, method: start.method,
                    is_control: start.is_control, bytes: start.bytes }, start.broker_context)
            } else {
                (serde_json::from_slice(&bytes)
                    .map_err(|_| anyhow::anyhow!("invalid session frame"))?, None)
            };
            if let Some(scope) = scope {
                validate_handle(&scope.handle)?;
                let Frame::RequestStart { id, .. } = &frame else {
                    anyhow::bail!("dependency scope requires a request start");
                };
                ensure!(self.scopes.len() < MAX_PENDING, "too many scoped request assemblies");
                ensure!(self.scopes.insert(*id, scope).is_none(), "duplicate dependency operation scope");
            }
            if matches!(frame, Frame::Shutdown | Frame::ShutdownComplete) {
                self.broker.finish()?;
            }
            if let Some(message) = self.ordinary.push(frame).await? {
                let scope = match message.header {
                    Header::Request { id, .. } => self.scopes.remove(&id),
                    Header::Result { .. } | Header::Error { .. }
                    | Header::Shutdown | Header::ShutdownComplete => None,
                };
                return Ok(Some(BrokerIncoming::Ordinary { message, scope }));
            }
        }
    }

    async fn broker_frame(&mut self, frame: BrokerFrame) -> Result<Option<BrokerIncoming>> {
        let mut failure = None;
        let normalized = match frame {
            BrokerFrame::BrokerRequestStart { id, service, version, method, authority, context,
                parent_id, request_generation, expected_owner_generation, bytes } => {
                let header = BrokerRequestHeader { id, service, version, method, authority, context,
                    parent_id, request_generation, expected_owner_generation };
                validate_header(&header)?;
                ensure!(self.requests.len() < MAX_BROKER_PENDING, "too many dependency request assemblies");
                let normalized = Frame::RequestStart {
                    id,
                    component: SessionComponent { kind: "dependency".to_owned(), name: header.service.clone() },
                    method: header.method.clone(),
                    is_control: false,
                    bytes,
                };
                ensure!(self.requests.insert(id, header).is_none(), "duplicate dependency request ID");
                normalized
            }
            BrokerFrame::BrokerResultStart { id, bytes } => Frame::ResultStart { id, bytes },
            BrokerFrame::BrokerChunk { id, index, data } => Frame::Chunk { id, index, data },
            BrokerFrame::BrokerEnd { id, chunks } => Frame::End { id, chunks },
            BrokerFrame::BrokerError { id, code, message: _ } => {
                failure = Some(DependencyError::new(code));
                Frame::Error { id, message: String::new() }
            }
            BrokerFrame::BrokerCancel { id } => {
                ensure!(self.broker.has_seen(id) && !self.broker.is_active(id),
                    "dependency cancellation requires an admitted request ID");
                return Ok(Some(BrokerIncoming::Cancel { id }));
            }
        };
        let Some(message) = self.broker.push(normalized).await? else {
            return Ok(None);
        };
        let incoming = match message.header {
            Header::Request { id, .. } => BrokerIncoming::Request {
                header: self.requests.remove(&id).context("dependency request header missing")?,
                payload: message.payload.context("dependency request payload missing")?,
            },
            Header::Result { id } => BrokerIncoming::Result {
                id,
                payload: message.payload.context("dependency result payload missing")?,
            },
            Header::Error { id, .. } => BrokerIncoming::Error {
                id,
                error: failure.context("dependency error metadata missing")?,
            },
            Header::Shutdown | Header::ShutdownComplete => unreachable!("broker has no shutdown frames"),
        };
        Ok(Some(incoming))
    }
}

/// Convert a shared spool frame into the broker namespace. Per-message metadata
/// is kept outside payload spools and is attached only to its start/error frame.
pub(super) fn broker_frame_value(
    frame: Frame,
    request: Option<&BrokerRequestHeader>,
    failure: Option<&DependencyError>,
) -> Result<Value> {
    let broker = match frame {
        Frame::RequestStart { id, bytes, .. } => {
            let header = request.context("dependency request header missing")?;
            validate_header(header)?;
            ensure!(header.id == id, "dependency request header ID mismatch");
            BrokerFrame::BrokerRequestStart {
                id, service: header.service.clone(), version: header.version,
                method: header.method.clone(), authority: header.authority.clone(),
                context: header.context.clone(), parent_id: header.parent_id,
                request_generation: header.request_generation,
                expected_owner_generation: header.expected_owner_generation, bytes,
            }
        }
        Frame::ResultStart { id, bytes } => BrokerFrame::BrokerResultStart { id, bytes },
        Frame::Chunk { id, index, data } => BrokerFrame::BrokerChunk { id, index, data },
        Frame::End { id, chunks } => BrokerFrame::BrokerEnd { id, chunks },
        Frame::Error { id, .. } => {
            let error = failure.context("dependency error metadata missing")?;
            BrokerFrame::BrokerError { id, code: error.code, message: error.to_string() }
        }
        Frame::Shutdown | Frame::ShutdownComplete => anyhow::bail!("broker has no shutdown frames"),
    };
    serde_json::to_value(broker).context("encode dependency broker frame")
}

pub(super) fn scoped_frame_value(frame: Frame, scope: Option<&DependencyScope>) -> Result<Value> {
    let is_request_start = matches!(frame, Frame::RequestStart { .. });
    let mut value = serde_json::to_value(frame).context("encode session frame")?;
    if is_request_start && let Some(scope) = scope {
        validate_handle(&scope.handle)?;
        value["broker_context"] = serde_json::to_value(scope)?;
    }
    Ok(value)
}

pub(super) fn cancel_frame_value(id: u64) -> Result<Value> {
    ensure!(id > 0, "invalid dependency cancellation ID");
    serde_json::to_value(BrokerFrame::BrokerCancel { id }).context("encode dependency cancellation")
}

#[cfg(test)]
#[path = "broker_wire_tests.rs"]
mod tests;
