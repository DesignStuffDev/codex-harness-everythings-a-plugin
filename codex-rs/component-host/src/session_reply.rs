//! Response observation is separate from ownership of an admitted cleanup.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use tokio::sync::oneshot;
use tokio::time::timeout;

use crate::session::Completion;
use crate::session::SessionInner;
use crate::session_failure::SessionFailure;

/// The response to an already admitted operation. Dropping this observer never
/// cancels or replays that operation. Its paired [`crate::DeferredControl`] stays
/// independently owned, including when observing the response returns an error.
/// A response error or timeout makes no claim that cleanup has completed.
#[must_use = "observe the response or explicitly abandon it while retaining its cleanup guard"]
pub struct PendingComponentReply {
    pub(super) response: oneshot::Receiver<Completion>,
    pub(super) inner: Arc<SessionInner>,
}

impl std::fmt::Debug for PendingComponentReply {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PendingComponentReply")
            .field("component", &self.inner.component)
            .finish_non_exhaustive()
    }
}

impl PendingComponentReply {
    /// Observe the response, using the session timeout from the start of this
    /// wait. Cancellation or timeout abandons only this observer; accepted work
    /// remains supervised. Explicitly await the paired cleanup guard's `release`
    /// when an acknowledged cleanup boundary is required.
    pub async fn wait(self) -> Result<Value> {
        timeout(self.inner.timeout, async {
            let payload = self
                .response
                .await
                .context("component response lost; outcome is unknown")?
                .map_err(SessionFailure::into_error)?;
            payload.into_value().await
        })
        .await
        .context("component response wait timed out; accepted operation outcome is unknown")?
    }
}
