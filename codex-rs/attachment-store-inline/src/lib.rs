//! Native inline attachment implementation, independent of engine orchestration.

use codex_attachment_store_api::*;

/// Leaves attachments inline instead of storing them.
pub struct InlineAttachmentStore;

impl AttachmentStore for InlineAttachmentStore {
    #[tracing::instrument(level = "trace", skip_all)]
    fn upload(&self, request: UploadRequest) -> UploadFuture<'_> {
        Box::pin(async move {
            Ok(UploadResult::Inline {
                bytes: request.data,
            })
        })
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn resolve<'a>(&'a self, request: ResolveRequest<'a>) -> ResolveFuture<'a> {
        let file_id = request.file_id;
        Box::pin(async move {
            Err(AttachmentStoreError::new(
                AttachmentStoreErrorKind::NotFound,
                format!("attachment `{file_id}` was not found"),
            ))
        })
    }
}
