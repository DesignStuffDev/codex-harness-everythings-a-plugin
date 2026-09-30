use std::io::BufRead;
use std::io::Write;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_attachment_store_api::AttachmentStore;
use codex_attachment_store_api::ResolveRequest;
use codex_attachment_store_api::UploadRequest;
use codex_attachment_store_api::UploadResult;
use codex_attachment_store_inline::InlineAttachmentStore;
use codex_component_api::MAX_FRAME_BYTES;
use serde_json::Value;
use serde_json::json;
use tokio::io::AsyncReadExt;

use crate::protocol::ErrorCode;
use crate::protocol::MAX_BLOB_BYTES;
use crate::protocol::MAX_ID_BYTES;
use crate::protocol::Reply;
use crate::protocol::ResolveParams;
use crate::protocol::Resolved;
use crate::protocol::UploadParams;
use crate::protocol::Uploaded;

/// Serve the same native inline store through the process protocol.
/// This entrypoint has no engine dependency or durable storage requirements.
pub async fn run_native_stdio() -> Result<()> {
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    let initialize = read_frame(&mut input)?;
    ensure!(
        initialize["type"] == "initialize" && initialize["api_version"] == 1,
        "invalid initialization"
    );
    send(&mut output, json!({"type":"ready","api_version":1}))?;
    let request = read_frame(&mut input)?;
    if request["type"] == "shutdown" {
        return Ok(());
    }
    ensure!(
        request["type"] == "request"
            && request["component"]["kind"] == "attachment_store"
            && request["component"]["name"] == "default",
        "invalid request"
    );
    let id = request["id"].as_u64().context("missing request id")?;
    let result = match request["method"].as_str() {
        Some("upload") => upload(request["params"].clone()).await?,
        Some("resolve") => resolve(request["params"].clone()).await?,
        _ => serde_json::to_value(Reply::<Value>::Error(ErrorCode::InvalidAttachment))?,
    };
    send(
        &mut output,
        json!({"type":"result","id":id,"result":result}),
    )?;
    // The runtime sends shutdown after consuming the final result.
    ensure!(
        read_frame(&mut input)?["type"] == "shutdown",
        "expected shutdown"
    );
    Ok(())
}

async fn upload(value: Value) -> Result<Value> {
    let params: UploadParams = serde_json::from_value(value)?;
    let result = async {
        if !params.blob.path.is_absolute()
            || params.blob.size_bytes > MAX_BLOB_BYTES
            || params.thread_id.len() > MAX_ID_BYTES
            || params
                .file_name
                .as_ref()
                .is_some_and(|name| name.len() > MAX_ID_BYTES)
        {
            return Err(ErrorCode::InvalidAttachment);
        }
        let metadata = tokio::fs::symlink_metadata(&params.blob.path)
            .await
            .map_err(|_| ErrorCode::Backend)?;
        if !metadata.is_file() || metadata.len() != params.blob.size_bytes {
            return Err(ErrorCode::InvalidAttachment);
        }
        // The host creates and holds this immutable private file throughout the call.
        let file = tokio::fs::File::open(params.blob.path)
            .await
            .map_err(|_| ErrorCode::Backend)?;
        let mut data = Vec::new();
        file.take(params.blob.size_bytes + 1)
            .read_to_end(&mut data)
            .await
            .map_err(|_| ErrorCode::Backend)?;
        if data.len() as u64 != params.blob.size_bytes {
            return Err(ErrorCode::InvalidAttachment);
        }
        let uploaded = InlineAttachmentStore
            .upload(UploadRequest {
                thread_id: params.thread_id,
                file_name: params.file_name,
                data,
            })
            .await
            .map_err(|error| ErrorCode::from(error.kind()))?;
        match uploaded {
            UploadResult::Inline { .. } => Ok(Uploaded::Inline),
            UploadResult::File { file_id } => Ok(Uploaded::File { file_id }),
        }
    }
    .await;
    serde_json::to_value(match result {
        Ok(uploaded) => Reply::Ok(uploaded),
        Err(code) => Reply::Error(code),
    })
    .map_err(Into::into)
}

async fn resolve(value: Value) -> Result<Value> {
    let params: ResolveParams = serde_json::from_value(value)?;
    let result = InlineAttachmentStore
        .resolve(ResolveRequest {
            file_id: &params.file_id,
            download_url_ttl: params.download_url_ttl_ms.map(Duration::from_millis),
        })
        .await;
    serde_json::to_value(match result {
        Ok(metadata) => Reply::Ok(Resolved {
            metadata,
            url_lifetime: None,
        }),
        Err(error) => Reply::Error(ErrorCode::from(error.kind())),
    })
    .map_err(Into::into)
}

fn read_frame(reader: &mut impl BufRead) -> Result<Value> {
    let mut bytes = Vec::new();
    let mut limited = std::io::Read::take(reader, (MAX_FRAME_BYTES + 1) as u64);
    limited.read_until(b'\n', &mut bytes)?;
    ensure!(
        bytes.len() <= MAX_FRAME_BYTES && bytes.last() == Some(&b'\n'),
        "invalid frame size"
    );
    serde_json::from_slice(&bytes).context("invalid frame")
}

fn send(output: &mut impl Write, frame: Value) -> Result<()> {
    let bytes = serde_json::to_vec(&frame)?;
    ensure!(bytes.len() < MAX_FRAME_BYTES, "result too large");
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
