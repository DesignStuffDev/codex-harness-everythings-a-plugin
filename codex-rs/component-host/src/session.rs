//! Shared component calls retain ownership after an individual waiter is cancelled.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde_json::Value;
use tokio::sync::Notify;
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::time::timeout;

use crate::session_wire::Header;
use crate::session_wire::Outgoing;
use crate::session_wire::Payload;
use crate::session_wire::SessionComponent;

pub(super) const CALL_SLOTS: usize = 32;
type Completion = std::result::Result<Payload, String>;
pub(super) type FinalStatus = Option<std::result::Result<(), String>>;

/// A persistent process shared by calls. Cancelling a waiter never replays or
/// cancels an accepted operation. Failed connections require explicit recovery.
#[derive(Clone)]
pub struct ComponentSession {
    pub(super) inner: Arc<SessionInner>,
}

impl std::fmt::Debug for ComponentSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComponentSession")
            .field("component", &self.inner.component)
            .finish_non_exhaustive()
    }
}

pub(super) struct SessionInner {
    pub component: SessionComponent,
    pub regular: mpsc::Sender<Outgoing>,
    pub control: mpsc::Sender<Outgoing>,
    pub shared: Arc<Shared>,
    pub shutdown: watch::Sender<bool>,
    pub timeout: Duration,
}

impl Drop for SessionInner {
    fn drop(&mut self) {
        self.shared.stop_admitting();
        self.shutdown.send_replace(true);
    }
}

pub(super) struct Pending {
    pub response: oneshot::Sender<Completion>,
    pub _slot: OwnedSemaphorePermit,
}

#[derive(Default)]
pub(super) struct PendingState {
    pub next_id: u64,
    pub calls: HashMap<u64, Pending>,
    pub closed: Option<String>,
}

pub(super) struct Shared {
    pub pending: Mutex<PendingState>,
    pub regular_slots: Arc<Semaphore>,
    pub control_slots: Arc<Semaphore>,
    pub closing: AtomicBool,
    pub changed: Notify,
    pub completion: watch::Sender<FinalStatus>,
}

impl Shared {
    pub fn stop_admitting(&self) {
        self.closing.store(true, Ordering::Release);
        self.regular_slots.close();
        self.control_slots.close();
    }

    pub fn finish(&self, outcome: std::result::Result<(), String>) {
        self.stop_admitting();
        let message = outcome
            .as_ref()
            .err()
            .cloned()
            .unwrap_or_else(|| "component session closed".to_owned());
        let pending = {
            let mut state = self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closed = Some(message.clone());
            std::mem::take(&mut state.calls)
        };
        for (_, pending) in pending {
            let _ = pending.response.send(Err(message.clone()));
        }
        self.completion.send_replace(Some(outcome));
        self.changed.notify_one();
    }
}

/// A pre-reserved cleanup request. Drop queues it without blocking or competing
/// for ordinary call capacity. The paired request is always fully sent first.
pub struct DeferredControl {
    inner: Arc<SessionInner>,
    permit: Option<mpsc::OwnedPermit<Outgoing>>,
    slot: Option<OwnedSemaphorePermit>,
    after_sent: Arc<AtomicBool>,
    method: String,
    params: Option<Value>,
}

impl std::fmt::Debug for DeferredControl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeferredControl")
            .field("method", &self.method)
            .finish_non_exhaustive()
    }
}

impl DeferredControl {
    fn enqueue(&mut self) -> Result<oneshot::Receiver<Completion>> {
        let permit = self.permit.take().context("cleanup already queued")?;
        let slot = self.slot.take().context("cleanup slot missing")?;
        let params = self.params.take().context("cleanup parameters missing")?;
        let (response, receiver) = oneshot::channel();
        let mut state = self
            .inner
            .shared
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(reason) = &state.closed {
            bail!("component cleanup unavailable: {reason}");
        }
        state.next_id = state
            .next_id
            .checked_add(1)
            .context("component request IDs exhausted")?;
        let id = state.next_id;
        state.calls.insert(
            id,
            Pending {
                response,
                _slot: slot,
            },
        );
        permit.send(Outgoing {
            header: Header::Request {
                id,
                component: self.inner.component.clone(),
                method: self.method.clone(),
                is_control: true,
            },
            value: params,
            after_sent: Some(Arc::clone(&self.after_sent)),
            sent: None,
        });
        Ok(receiver)
    }

    /// Queue cleanup and wait for its acknowledgement. A timeout stops waiting;
    /// the cleanup remains owned by the session until completion or disconnect.
    pub async fn release(mut self) -> Result<()> {
        let response = self.enqueue()?;
        timeout(self.inner.timeout, async {
            let payload = response
                .await
                .context("component cleanup response lost")?
                .map_err(anyhow::Error::msg)?;
            payload.into_value().await?;
            Ok::<(), anyhow::Error>(())
        })
        .await
        .context("component cleanup wait timed out; outcome is unknown")?
    }
}

impl Drop for DeferredControl {
    fn drop(&mut self) {
        if self.permit.is_some() {
            // Pending owns the slot and drains the eventual response even though
            // this discarded receiver has no waiter. Disconnect releases leases.
            let _ = self.enqueue();
        }
    }
}

impl ComponentSession {
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        timeout(self.inner.timeout, async {
            let (response, _cleanup) = self.start(method, params, None).await?;
            let payload = response
                .await
                .context("component response lost; outcome is unknown")?
                .map_err(anyhow::Error::msg)?;
            payload.into_value().await
        })
        .await
        .context("component request wait timed out; accepted operation outcome is unknown")?
    }

    /// Reserve both ordinary and cleanup capacity before submitting a call.
    /// Cancellation before admission performs no operation; afterwards it queues
    /// cleanup and keeps the accepted operation supervised.
    pub async fn call_with_cleanup(
        &self,
        method: &str,
        params: Value,
        cleanup_method: &str,
        cleanup_params: Value,
    ) -> Result<(Value, DeferredControl)> {
        timeout(self.inner.timeout, async {
            let (response, cleanup) = self
                .start(
                    method,
                    params,
                    Some((cleanup_method.to_owned(), cleanup_params)),
                )
                .await?;
            let cleanup = cleanup.context("component cleanup reservation missing")?;
            let payload = response
                .await
                .context("component response lost; outcome is unknown")?
                .map_err(anyhow::Error::msg)?;
            Ok((payload.into_value().await?, cleanup))
        })
        .await
        .context("component request wait timed out; accepted operation outcome is unknown")?
    }

    async fn start(
        &self,
        method: &str,
        params: Value,
        cleanup: Option<(String, Value)>,
    ) -> Result<(oneshot::Receiver<Completion>, Option<DeferredControl>)> {
        let cleanup = match cleanup {
            Some((method, params)) => {
                // Validation before reserving/admitting avoids killing a healthy
                // service later for a cleanup that can never fit its control lane.
                anyhow::ensure!(
                    !method.is_empty() && method.len() <= 256,
                    "component cleanup method must contain 1 to 256 bytes"
                );
                anyhow::ensure!(
                    serde_json::to_vec(&params)?.len() <= 64 * 1024,
                    "component cleanup parameters exceed 64 KiB"
                );
                let slot = Arc::clone(&self.inner.shared.control_slots)
                    .acquire_owned()
                    .await
                    .context("component session is closing")?;
                let permit = self
                    .inner
                    .control
                    .clone()
                    .reserve_owned()
                    .await
                    .context("component control writer closed")?;
                Some((method, params, slot, permit))
            }
            None => None,
        };
        anyhow::ensure!(
            !method.is_empty() && method.len() <= 256,
            "component method must contain 1 to 256 bytes"
        );
        let slot = Arc::clone(&self.inner.shared.regular_slots)
            .acquire_owned()
            .await
            .context("component session is closing")?;
        let permit = self
            .inner
            .regular
            .clone()
            .reserve_owned()
            .await
            .context("component writer closed")?;
        let (response, receiver) = oneshot::channel();
        let sent = Arc::new(AtomicBool::new(false));
        let mut state = self
            .inner
            .shared
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        anyhow::ensure!(
            !self.inner.shared.closing.load(Ordering::Acquire),
            "component session is closing"
        );
        if let Some(reason) = &state.closed {
            bail!("component session unavailable: {reason}");
        }
        state.next_id = state
            .next_id
            .checked_add(1)
            .context("component request IDs exhausted")?;
        let id = state.next_id;
        state.calls.insert(
            id,
            Pending {
                response,
                _slot: slot,
            },
        );
        permit.send(Outgoing {
            header: Header::Request {
                id,
                component: self.inner.component.clone(),
                method: method.to_owned(),
                is_control: false,
            },
            value: params,
            after_sent: None,
            sent: Some(Arc::clone(&sent)),
        });
        let cleanup = cleanup.map(|(method, params, slot, permit)| DeferredControl {
            inner: Arc::clone(&self.inner),
            permit: Some(permit),
            slot: Some(slot),
            after_sent: sent,
            method,
            params: Some(params),
        });
        Ok((receiver, cleanup))
    }

    /// Fence admission and start supervised shutdown without waiting. This is
    /// idempotent and also closes admission through every retained session clone.
    /// Accepted work remains owned until drained or the shutdown deadline expires.
    pub fn begin_close(&self) {
        self.inner.shared.stop_admitting();
        self.inner.shutdown.send_replace(true);
    }

    /// Stop admission, drain accepted requests, and close the child. Release all
    /// application-owned cleanup guards first. Forced shutdown reports ambiguity.
    pub async fn close(&self) -> Result<()> {
        let mut completion = self.inner.shared.completion.subscribe();
        self.begin_close();
        loop {
            if let Some(outcome) = completion.borrow().clone() {
                return outcome.map_err(anyhow::Error::msg);
            }
            completion
                .changed()
                .await
                .context("component supervisor disappeared")?;
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
