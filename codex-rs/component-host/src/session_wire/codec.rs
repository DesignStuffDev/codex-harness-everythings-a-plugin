//! Bounded physical frames. Callers decode raw bytes directly, preserving
//! duplicate-field rejection; this module never round-trips through Value.
use super::MAX_FRAME_BYTES;
use super::SessionComponent;
use anyhow::Result;
use anyhow::ensure;
use serde::Deserialize;
use serde::Serialize;
use tokio::io::AsyncBufRead;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Frame {
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

pub(super) async fn read_frame_bytes<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> Result<Option<Vec<u8>>> {
    // fill_buf/consume keeps even a malicious newline-free input bounded.
    let mut line = Vec::new();
    loop {
        let buffer = reader.fill_buf().await?;
        if buffer.is_empty() {
            ensure!(line.is_empty(), "truncated session message");
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
            break;
        }
    }
    Ok(Some(line))
}

pub(super) async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, frame: Frame) -> Result<()> {
    let bytes = serde_json::to_vec(&frame)?;
    ensure!(
        bytes.len() < MAX_FRAME_BYTES,
        "session frame exceeds wire limit"
    );
    writer.write_all(&bytes).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}
