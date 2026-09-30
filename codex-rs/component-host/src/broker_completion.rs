//! A received parent result retains admission ownership until its reverse work joins.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use anyhow::ensure;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::broker_dispatch::BrokerDispatcher;
use crate::session::CALL_SLOTS;
use crate::session::Shared;
use crate::session_wire::Payload;

struct State {
    calls: Mutex<BTreeMap<u64, Option<JoinHandle<()>>>>,
    changed: Notify,
}

#[derive(Clone)]
pub(super) struct ParentCompletions {
    inner: Arc<State>,
}

impl ParentCompletions {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(State {
                calls: Mutex::new(BTreeMap::new()),
                changed: Notify::new(),
            }),
        }
    }

    /// No await on the physical reader: each task retains its pending entry and
    /// original capacity permit until every reverse handler for this parent joins.
    pub fn complete(
        &self,
        id: u64,
        result: std::result::Result<Payload, String>,
        shared: Arc<Shared>,
        dispatcher: BrokerDispatcher,
    ) -> Result<()> {
        {
            let mut calls = self
                .inner
                .calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            ensure!(
                calls.len() < CALL_SLOTS * 2,
                "parent completion capacity exceeded"
            );
            ensure!(
                !calls.contains_key(&id),
                "duplicate parent terminal response"
            );
            ensure!(
                shared
                    .pending
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .calls
                    .contains_key(&id),
                "unexpected component response ID"
            );
            calls.insert(id, None);
            // Admission and revocation synchronize on the dispatcher's calls lock.
            dispatcher.finish_parent(id);
        }
        let state = Arc::clone(&self.inner);
        let task = tokio::spawn(async move {
            dispatcher.drained_parent(id).await;
            let result = if dispatcher.parent_delivery_failed(id) {
                Err(
                    "dependency response delivery failed; accepted operation outcomes are unknown"
                        .to_owned(),
                )
            } else {
                result
            };
            dispatcher.release_parent(id);
            let pending = shared
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .calls
                .remove(&id);
            if let Some(pending) = pending {
                let _ = pending.response.send(result);
            }
            shared.changed.notify_one();
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
        if let Some(slot) = calls.get_mut(&id) {
            *slot = Some(task);
        }
        Ok(())
    }

    pub fn is_idle(&self) -> bool {
        self.inner
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
    }

    pub async fn drained(&self) {
        loop {
            let changed = self.inner.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.is_idle() {
                return;
            }
            changed.await;
        }
    }
}
