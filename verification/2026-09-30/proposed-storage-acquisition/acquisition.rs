//! A failed acquisition must not leave an unowned live writer in a usable store.

use codex_component_host::ComponentSession;
use codex_thread_store::ThreadStoreError;
use codex_thread_store::ThreadStoreResult;

use crate::CALL_METHOD;
use crate::StorageReply;
use crate::StorageRequest;
use crate::StorageResponse;
use crate::error::transport_error;

pub(crate) enum AcquisitionKind {
    Create,
    Resume,
}

struct AcquisitionFence<'a> {
    session: &'a ComponentSession,
    armed: bool,
}

impl Drop for AcquisitionFence<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.session.begin_close();
        }
    }
}

/// Preserve the configured wait deadline. An uncertain acquisition closes the
/// entire selected store, including its other handles: accepted operations are
/// drained under the existing bounded shutdown supervisor, never replayed.
/// A confirmed native application error leaves the healthy store available.
pub(crate) async fn acquire(
    session: &ComponentSession,
    request: StorageRequest,
    kind: AcquisitionKind,
) -> ThreadStoreResult<()> {
    let params = serde_json::to_value(request).map_err(transport_error)?;
    let mut fence = AcquisitionFence { session, armed: true };
    let value = session.call(CALL_METHOD, params).await.map_err(acquisition_error)?;
    let reply = serde_json::from_value(value).map_err(acquisition_error)?;
    match reply {
        StorageReply::Error(error) => {
            fence.armed = false;
            Err(error.into_native())
        }
        StorageReply::Ok(response) => {
            match (kind, *response) {
                (AcquisitionKind::Create, StorageResponse::CreateThread(()))
                | (AcquisitionKind::Resume, StorageResponse::ResumeThread(())) => {}
                _ => return Err(acquisition_error("storage response operation does not match acquisition")),
            }
            fence.armed = false;
            Ok(())
        }
    }
}

fn acquisition_error(error: impl std::fmt::Display) -> ThreadStoreError {
    transport_error(format!(
        "thread acquisition is uncertain; the selected store is closing to release unowned writers: {error}"
    ))
}
