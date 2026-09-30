//! Independently supervised dependency handlers leave the physical reader unblocked.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::Result;
use anyhow::ensure;
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::broker_api::BROKER_CAPACITY;
use crate::broker_api::DependencyError;
use crate::broker_api::DependencyErrorCode;
use crate::broker_decode;
use crate::broker_host::BrokerHost;
use crate::broker_wire::BrokerRequestHeader;
use crate::broker_writer::BrokerOutgoing;
use crate::broker_writer::BrokerResponse;
use crate::session_wire::Payload;

struct ActiveCall {
    parent_id: u64,
    cancellation: watch::Sender<bool>,
    // This monitor owns and awaits the actual handler JoinHandle. It must never
    // be aborted during a normal drain, including when the caller disappears.
    monitor: Option<JoinHandle<()>>,
}

struct State {
    host: BrokerHost,
    calls: Mutex<BTreeMap<u64, ActiveCall>>,
    response_failures: Mutex<BTreeSet<u64>>,
    responses: mpsc::Sender<BrokerResponse>,
    closed: AtomicBool,
    changed: Notify,
}

#[derive(Clone)]
pub(super) struct BrokerDispatcher {
    inner: Arc<State>,
}

impl BrokerDispatcher {
    pub fn new(host: BrokerHost, responses: mpsc::Sender<BrokerResponse>) -> Self {
        Self {
            inner: Arc::new(State {
                host,
                calls: Mutex::new(BTreeMap::new()),
                response_failures: Mutex::new(BTreeSet::new()),
                responses,
                closed: AtomicBool::new(false),
                changed: Notify::new(),
            }),
        }
    }

    /// Admission reserves handler and response ownership synchronously. The same
    /// lock fences parent revocation, so no callback can appear after its drain.
    pub fn dispatch(&self, header: BrokerRequestHeader, payload: Payload) -> Result<()> {
        let response = self
            .inner
            .responses
            .clone()
            .try_reserve_owned()
            .map_err(|_| anyhow::anyhow!("dependency peer exceeded bounded response capacity"))?;
        let (cancellation, cancelled) = watch::channel(false);
        let id = header.id;
        let context = {
            let mut calls = self
                .inner
                .calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            ensure!(
                !self.inner.closed.load(Ordering::Acquire),
                "dependency dispatch is closed"
            );
            let context = match self.inner.host.context_for_request(&header) {
                Ok(context) => context,
                Err(error) => {
                    let (completed, _ignored) = oneshot::channel();
                    response.send(BrokerResponse {
                        message: BrokerOutgoing::Error { id, error },
                        completed,
                    });
                    return Ok(());
                }
            };
            ensure!(
                calls.len() < BROKER_CAPACITY,
                "dependency peer exceeded active call capacity"
            );
            ensure!(
                !calls.contains_key(&id),
                "duplicate active dependency request"
            );
            calls.insert(
                id,
                ActiveCall {
                    parent_id: header.parent_id,
                    cancellation,
                    monitor: None,
                },
            );
            context
        };
        let parent_id = header.parent_id;
        let host = self.inner.host.clone();
        let handler = tokio::spawn(async move {
            let params = broker_decode::decode(payload, &context, cancelled.clone()).await?;
            host.run(header, params, cancelled).await
        });
        let state = Arc::clone(&self.inner);
        let monitor = tokio::spawn(async move {
            // Join includes service-future destruction and the owned decoder's
            // cancellation join. Only then may the call disappear or its parent
            // complete. Dropping this monitor's handle does not abort it.
            let result = handler
                .await
                .unwrap_or_else(|_| Err(DependencyError::new(DependencyErrorCode::ServiceFailure)));
            let message = match result {
                Ok((result, authority)) => BrokerOutgoing::Result {
                    id,
                    result,
                    authority: Some(authority),
                },
                Err(error) => BrokerOutgoing::Error { id, error },
            };
            let (completed, flushed) = oneshot::channel();
            response.send(BrokerResponse { message, completed });
            if !matches!(flushed.await, Ok(Ok(()))) {
                state
                    .response_failures
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(parent_id);
            }
            // The response can still own a serializer or block in its physical
            // writer after the handler joins. Retain the parent until its final
            // flush (or supervised terminal writer failure) is acknowledged.
            state
                .calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&id);
            state.changed.notify_waiters();
        });
        let mut calls = self
            .inner
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(call) = calls.get_mut(&id) {
            call.monitor = Some(monitor);
        }
        Ok(())
    }

    pub fn cancel(&self, id: u64) {
        if let Some(call) = self
            .inner
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&id)
        {
            call.cancellation.send_replace(true);
        }
    }

    pub fn finish_parent(&self, id: u64) {
        let calls = self
            .inner
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.inner.host.revoke_parent(id);
        for call in calls.values() {
            if call.parent_id == id {
                call.cancellation.send_replace(true);
            }
        }
    }

    pub fn release_parent(&self, id: u64) {
        self.inner.host.release_parent(id);
    }

    pub fn parent_delivery_failed(&self, id: u64) -> bool {
        self.inner
            .response_failures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&id)
    }

    pub fn is_idle(&self) -> bool {
        self.inner
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
    }

    pub fn stop_if_idle(&self) -> bool {
        let calls = self
            .inner
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !calls.is_empty() {
            return false;
        }
        self.inner.closed.store(true, Ordering::Release);
        true
    }

    pub async fn drained(&self) {
        self.wait_for(None).await;
    }

    pub async fn drained_parent(&self, id: u64) {
        self.wait_for(Some(id)).await;
    }

    async fn wait_for(&self, parent: Option<u64>) {
        loop {
            let changed = self.inner.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let pending = self
                .inner
                .calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .values()
                .any(|call| parent.is_none_or(|id| call.parent_id == id));
            if !pending {
                return;
            }
            changed.await;
        }
    }

    /// Cancellation is cooperative and joined. Never clear ownership or abort a
    /// decoder-owning handler to pretend a forced teardown has drained.
    pub fn cancel_all(&self) {
        let calls = self
            .inner
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.inner.closed.store(true, Ordering::Release);
        self.inner.host.revoke_all();
        for call in calls.values() {
            call.cancellation.send_replace(true);
        }
    }

    pub fn release_all(&self) {
        self.inner.host.release_all();
        self.inner
            .response_failures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
}

#[cfg(test)]
#[path = "broker_dispatch_tests.rs"]
mod tests;
