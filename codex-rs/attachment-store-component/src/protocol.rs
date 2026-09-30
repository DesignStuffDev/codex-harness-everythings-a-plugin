//! Version 1 wire types. Blob bytes and signed URLs must not enter diagnostic logs.

use std::path::PathBuf;

use codex_attachment_store_api::AttachmentMetadata;
use codex_attachment_store_api::AttachmentStoreError;
use codex_attachment_store_api::AttachmentStoreErrorKind;
use serde::Deserialize;
use serde::Serialize;

/// Matches the upstream maximum encoded prompt-image input, outside JSON frames.
pub const MAX_BLOB_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_ID_BYTES: usize = 4096;
pub const MAX_METADATA_STRING_BYTES: usize = 64 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Blob {
    pub path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UploadParams {
    pub thread_id: String,
    pub file_name: Option<String>,
    pub blob: Blob,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Uploaded {
    /// The adapter retains the original bytes; plugins cannot supply new inline bytes.
    Inline,
    /// Must identify a file the selected model provider can consume.
    File { file_id: String },
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolveParams {
    pub file_id: String,
    pub download_url_ttl_ms: Option<u64>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UrlLifetime {
    Expires { expires_at_unix_ms: u64 },
    Permanent,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Resolved {
    pub metadata: AttachmentMetadata,
    pub url_lifetime: Option<UrlLifetime>,
}

/// Deliberately excludes backend error messages, which can contain credentials.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotFound,
    InvalidAttachment,
    Backend,
}

impl From<AttachmentStoreErrorKind> for ErrorCode {
    fn from(kind: AttachmentStoreErrorKind) -> Self {
        match kind {
            AttachmentStoreErrorKind::NotFound => Self::NotFound,
            AttachmentStoreErrorKind::InvalidAttachment => Self::InvalidAttachment,
            AttachmentStoreErrorKind::Backend => Self::Backend,
        }
    }
}

impl From<ErrorCode> for AttachmentStoreError {
    fn from(code: ErrorCode) -> Self {
        let (kind, message) = match code {
            ErrorCode::NotFound => (
                AttachmentStoreErrorKind::NotFound,
                "attachment was not found",
            ),
            ErrorCode::InvalidAttachment => (
                AttachmentStoreErrorKind::InvalidAttachment,
                "attachment component rejected invalid data or metadata",
            ),
            ErrorCode::Backend => (
                AttachmentStoreErrorKind::Backend,
                "attachment component failed",
            ),
        };
        Self::new(kind, message)
    }
}

/// Each result has exactly one key: `ok` or `error`.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reply<T> {
    Ok(T),
    Error(ErrorCode),
}

impl<T> Reply<T> {
    pub fn into_result(self) -> Result<T, AttachmentStoreError> {
        match self {
            Self::Ok(value) => Ok(value),
            Self::Error(code) => Err(code.into()),
        }
    }
}
