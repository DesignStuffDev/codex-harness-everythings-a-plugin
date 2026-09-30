use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use codex_attachment_store_api::AttachmentMetadata;
use codex_attachment_store_api::AttachmentStore;
use codex_attachment_store_api::AttachmentStoreError;
use codex_attachment_store_api::AttachmentStoreResult;
use codex_attachment_store_api::ResolveFuture;
use codex_attachment_store_api::ResolveRequest;
use codex_attachment_store_api::UploadFuture;
use codex_attachment_store_api::UploadRequest;
use codex_attachment_store_api::UploadResult;
use codex_component_host::ComponentBinding;
use codex_component_host::ComponentCatalog;
use tempfile::TempDir;

use crate::protocol::Blob;
use crate::protocol::ErrorCode;
use crate::protocol::MAX_BLOB_BYTES;
use crate::protocol::MAX_ID_BYTES;
use crate::protocol::MAX_METADATA_STRING_BYTES;
use crate::protocol::Reply;
use crate::protocol::ResolveParams;
use crate::protocol::Resolved;
use crate::protocol::UploadParams;
use crate::protocol::Uploaded;
use crate::protocol::UrlLifetime;

/// Resolves only an explicit replacement selection. Installation alone is inert.
pub fn from_catalog(
    catalog: &ComponentCatalog,
) -> anyhow::Result<Option<Arc<dyn AttachmentStore>>> {
    catalog
        .selected("attachment_store", "default")
        .map(ComponentAttachmentStore::new)
        .transpose()
        .map(|store| store.map(|store| Arc::new(store) as Arc<dyn AttachmentStore>))
}

/// One immutable component binding. Persistent storage belongs to the plugin;
/// each operation owns and cancels its fresh process and temporary transfer.
pub struct ComponentAttachmentStore {
    binding: ComponentBinding,
}

impl ComponentAttachmentStore {
    pub fn new(binding: ComponentBinding) -> anyhow::Result<Self> {
        anyhow::ensure!(
            binding.spec.kind == "attachment_store"
                && binding.spec.name == "default"
                && binding.spec.contract_version == 1,
            "expected attachment_store:default contract version 1"
        );
        Ok(Self { binding })
    }
}

struct StagedBlob {
    _directory: TempDir,
    path: PathBuf,
}

// A blocking filesystem call cannot itself be aborted. Stop before beginning a
// queued transfer and between bounded writes when the upload future is dropped.
struct CancelStagingOnDrop(Arc<AtomicBool>);

impl Drop for CancelStagingOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

fn check_staging_cancelled(cancelled: &AtomicBool) -> std::io::Result<()> {
    if cancelled.load(Ordering::Acquire) {
        return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
    }
    Ok(())
}

fn write_staged_data(
    writer: &mut impl Write,
    bytes: &[u8],
    cancelled: &AtomicBool,
) -> std::io::Result<()> {
    for chunk in bytes.chunks(64 * 1024) {
        check_staging_cancelled(cancelled)?;
        writer.write_all(chunk)?;
    }
    check_staging_cancelled(cancelled)?;
    writer.flush()
}

impl StagedBlob {
    fn create(
        state_dir: PathBuf,
        request: UploadRequest,
        cancelled: Arc<AtomicBool>,
    ) -> AttachmentStoreResult<(Self, UploadRequest)> {
        let result = (|| -> std::io::Result<_> {
            check_staging_cancelled(&cancelled)?;
            std::fs::create_dir_all(state_dir.as_path())?;
            let directory = tempfile::Builder::new()
                .prefix(".attachment-transfer-")
                .tempdir_in(state_dir)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))?;
            }
            let path = directory.path().join("input.bin");
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&path)?;
            write_staged_data(&mut file, &request.data, &cancelled)?;
            check_staging_cancelled(&cancelled)?;
            let mut permissions = file.metadata()?.permissions();
            permissions.set_readonly(true);
            file.set_permissions(permissions)?;
            Ok(Self {
                _directory: directory,
                path,
            })
        })();
        result
            .map(|blob| (blob, request))
            .map_err(|_| ErrorCode::Backend.into())
    }
}

#[cfg(windows)]
impl Drop for StagedBlob {
    fn drop(&mut self) {
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            let _ = std::fs::set_permissions(&self.path, permissions);
        }
    }
}

impl AttachmentStore for ComponentAttachmentStore {
    fn upload(&self, request: UploadRequest) -> UploadFuture<'_> {
        Box::pin(async move {
            if request.data.len() as u64 > MAX_BLOB_BYTES
                || request.thread_id.len() > MAX_ID_BYTES
                || request
                    .file_name
                    .as_ref()
                    .is_some_and(|name| name.len() > MAX_ID_BYTES)
            {
                return Err(ErrorCode::InvalidAttachment.into());
            }
            let state_dir = self.binding.state_dir.clone();
            let (staged, request) = {
                let cancelled = Arc::new(AtomicBool::new(false));
                let _cancel_on_drop = CancelStagingOnDrop(Arc::clone(&cancelled));
                tokio::task::spawn_blocking(move || {
                    StagedBlob::create(state_dir, request, cancelled)
                })
                .await
                .map_err(|_| AttachmentStoreError::from(ErrorCode::Backend))??
            };
            let params = UploadParams {
                thread_id: request.thread_id,
                file_name: request.file_name,
                blob: Blob {
                    path: staged.path.clone(),
                    size_bytes: request.data.len() as u64,
                },
            };
            let params = serde_json::to_value(params)
                .map_err(|_| AttachmentStoreError::from(ErrorCode::Backend))?;
            let response = self
                .binding
                .call("upload", params)
                .await
                .map_err(|_| AttachmentStoreError::from(ErrorCode::Backend))?;
            let uploaded = serde_json::from_value::<Reply<Uploaded>>(response)
                .map_err(|_| AttachmentStoreError::from(ErrorCode::InvalidAttachment))?
                .into_result()?;
            match uploaded {
                Uploaded::Inline => Ok(UploadResult::Inline {
                    bytes: request.data,
                }),
                Uploaded::File { file_id }
                    if !file_id.is_empty() && file_id.len() <= MAX_ID_BYTES =>
                {
                    Ok(UploadResult::File { file_id })
                }
                Uploaded::File { .. } => Err(ErrorCode::InvalidAttachment.into()),
            }
        })
    }

    fn resolve<'a>(&'a self, request: ResolveRequest<'a>) -> ResolveFuture<'a> {
        Box::pin(async move {
            if request.file_id.is_empty() || request.file_id.len() > MAX_ID_BYTES {
                return Err(ErrorCode::InvalidAttachment.into());
            }
            let ttl_ms = request
                .download_url_ttl
                .map(|ttl| {
                    u64::try_from(ttl.as_millis() + u128::from(ttl.subsec_nanos() % 1_000_000 != 0))
                        .map_err(|_| AttachmentStoreError::from(ErrorCode::InvalidAttachment))
                })
                .transpose()?;
            let params = serde_json::to_value(ResolveParams {
                file_id: request.file_id.to_owned(),
                download_url_ttl_ms: ttl_ms,
            })
            .map_err(|_| AttachmentStoreError::from(ErrorCode::Backend))?;
            let response = self
                .binding
                .call("resolve", params)
                .await
                .map_err(|_| AttachmentStoreError::from(ErrorCode::Backend))?;
            let resolved = serde_json::from_value::<Reply<Resolved>>(response)
                .map_err(|_| AttachmentStoreError::from(ErrorCode::InvalidAttachment))?
                .into_result()?;
            validate_resolution(resolved, request.download_url_ttl)
        })
    }
}

fn validate_resolution(
    resolved: Resolved,
    requested_ttl: Option<Duration>,
) -> AttachmentStoreResult<AttachmentMetadata> {
    let metadata = resolved.metadata;
    if [
        &metadata.file_name,
        &metadata.digest,
        &metadata.mime_type,
        &metadata.file_url,
    ]
    .iter()
    .any(|value| {
        value
            .as_ref()
            .is_some_and(|value| value.len() > MAX_METADATA_STRING_BYTES)
    }) || metadata
        .size_bytes
        .is_some_and(|size| size > MAX_BLOB_BYTES)
    {
        return Err(ErrorCode::InvalidAttachment.into());
    }
    match (
        requested_ttl,
        metadata.file_url.as_deref(),
        resolved.url_lifetime,
    ) {
        (None, None, None) => {}
        (Some(_), Some(url), Some(UrlLifetime::Permanent)) if !url.is_empty() => {}
        (Some(ttl), Some(url), Some(UrlLifetime::Expires { expires_at_unix_ms }))
            if !url.is_empty() =>
        {
            let expiry = UNIX_EPOCH.checked_add(Duration::from_millis(expires_at_unix_ms));
            let required = SystemTime::now().checked_add(ttl);
            if !matches!((expiry, required), (Some(expiry), Some(required)) if expiry >= required) {
                return Err(ErrorCode::InvalidAttachment.into());
            }
        }
        _ => return Err(ErrorCode::InvalidAttachment.into()),
    }
    Ok(metadata)
}

#[cfg(test)]
#[path = "staging_tests.rs"]
mod staging_tests;
