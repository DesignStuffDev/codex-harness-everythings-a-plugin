//! Negotiated one-invocation envelopes over the shared bounded payload codec.

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
use codex_component_api::COMPONENT_API_VERSION;
use codex_component_api::STREAMING_SESSION_VERSION;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use tokio::io::AsyncBufRead;
use tokio::io::AsyncWrite;

use crate::ComponentBinding;
use crate::payload_work::JsonWorkClient;
use crate::process::read_frame;
use crate::process::write_frame;
use crate::session_wire::Frame;
use crate::session_wire::Header;
use crate::session_wire::MessageAssembler;
use crate::session_wire::Outgoing;
use crate::session_wire::Sending;
use crate::session_wire::read_frame_bytes;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Selection {
    kind: String,
    name: String,
    contract_version: u32,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Session {
    mode: String,
    version: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ready {
    r#type: String,
    api_version: u32,
    component: Selection,
    session: Session,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum StreamingFrame {
    MessageStart { id: u64, bytes: u64 },
    Chunk { id: u64, index: u64, data: String },
    End { id: u64, chunks: u64 },
}

pub(crate) struct InvocationTransport {
    selection: Option<Selection>,
    work: Option<JsonWorkClient>,
    assembly: Option<MessageAssembler>,
    next_id: Option<u64>,
}

impl InvocationTransport {
    pub(crate) fn new(binding: &ComponentBinding, work: Option<JsonWorkClient>) -> Self {
        let selection = work.as_ref().map(|_| Selection {
            kind: binding.spec.kind.clone(),
            name: binding.spec.name.clone(),
            contract_version: binding.spec.contract_version,
        });
        let assembly = work
            .as_ref()
            .map(|work| MessageAssembler::with_work(1, work.clone()));
        Self {
            selection,
            work,
            assembly,
            next_id: Some(1),
        }
    }

    pub(crate) async fn initialize<W: AsyncWrite + Unpin, R: AsyncBufRead + Unpin>(
        &self,
        binding: &ComponentBinding,
        writer: &mut W,
        reader: &mut R,
    ) -> Result<()> {
        let mut initialize = json!({
            "type":"initialize", "api_version":COMPONENT_API_VERSION,
            "plugin_id":binding.plugin_id, "config":binding.config, "state_dir":binding.state_dir,
        });
        if let Some(selection) = &self.selection {
            initialize["component"] = serde_json::to_value(selection)?;
            initialize["session"] = json!({"mode":"streaming","version":STREAMING_SESSION_VERSION});
        }
        write_frame(writer, initialize).await?;
        if let Some(selection) = &self.selection {
            let bytes = read_frame_bytes(reader)
                .await?
                .context("component handshake EOF")?;
            let ready: Ready = serde_json::from_slice(&bytes)
                .map_err(|_| anyhow::anyhow!("invalid streaming component handshake"))?;
            ensure!(
                ready.r#type == "ready"
                    && ready.api_version == COMPONENT_API_VERSION
                    && ready.component == *selection
                    && ready.session.mode == "streaming"
                    && ready.session.version == STREAMING_SESSION_VERSION,
                "component must acknowledge the selected contract and streaming session version"
            );
        } else {
            let ready = read_frame(reader).await.context("component handshake")?;
            ensure!(
                ready.get("type").and_then(Value::as_str) == Some("ready")
                    && ready.get("api_version").and_then(Value::as_u64)
                        == Some(u64::from(COMPONENT_API_VERSION)),
                "component handshake requires ready with api_version 1"
            );
        }
        Ok(())
    }

    pub(crate) async fn send_request<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut W,
        value: Value,
    ) -> Result<()> {
        let Some(work) = &self.work else {
            return write_frame(writer, value).await;
        };
        let payload = work.serialize(value).await?;
        let mut sending = Sending::with_work(
            Outgoing {
                header: Header::Result { id: 1 },
                value: Value::Null,
                after_sent: None,
                sent: None,
            },
            payload,
            work.clone(),
        )?;
        loop {
            let (frame, complete) = sending.next_frame().await?;
            let frame = match frame {
                Frame::ResultStart { id, bytes } => StreamingFrame::MessageStart { id, bytes },
                Frame::Chunk { id, index, data } => StreamingFrame::Chunk { id, index, data },
                Frame::End { id, chunks } => StreamingFrame::End { id, chunks },
                _ => bail!("invalid internal streaming frame"),
            };
            write_frame(writer, serde_json::to_value(frame)?).await?;
            if complete {
                sending.mark_sent();
                return Ok(());
            }
        }
    }

    pub(crate) async fn receive<R: AsyncBufRead + Unpin>(
        &mut self,
        reader: &mut R,
    ) -> Result<Value> {
        let Some(work) = &self.work else {
            return read_frame(reader).await;
        };
        let assembly = self
            .assembly
            .as_mut()
            .context("streaming assembly missing")?;
        loop {
            let Some(bytes) = read_frame_bytes(reader).await? else {
                assembly.finish()?;
                bail!("unexpected EOF from streaming component");
            };
            let frame: StreamingFrame = serde_json::from_slice(&bytes)
                .map_err(|_| anyhow::anyhow!("invalid streaming physical frame"))?;
            let frame = match frame {
                StreamingFrame::MessageStart { id, bytes } => {
                    ensure!(
                        Some(id) == self.next_id && bytes > 0,
                        "invalid streaming message ID or empty body"
                    );
                    assembly.finish()?;
                    ensure!(
                        !assembly.has_seen(id) && !assembly.is_active(id),
                        "duplicate streaming message ID"
                    );
                    self.next_id = id.checked_add(1);
                    Frame::ResultStart { id, bytes }
                }
                StreamingFrame::Chunk { id, index, data } => Frame::Chunk { id, index, data },
                StreamingFrame::End { id, chunks } => Frame::End { id, chunks },
            };
            if let Some(message) = assembly.push(frame).await? {
                let payload = message.payload.context("streaming payload missing")?;
                return work.decode(payload).await;
            }
        }
    }
    pub(crate) async fn finish<R: AsyncBufRead + Unpin>(&self, reader: &mut R) -> Result<()> {
        if self.work.is_some() {
            ensure!(read_frame_bytes(reader).await?.is_none(), "component emitted data after its terminal result");
        }
        Ok(())
    }

}
