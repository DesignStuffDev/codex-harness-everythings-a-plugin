use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use codex_protocol::ThreadId;
use codex_thread_store::PreparedFork;
use codex_thread_store::StoredModelContext;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreError;
use codex_thread_store::ThreadStoreResult;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::MigrationLeaseRequest;
use crate::RELEASE_MIGRATION_METHOD;
use crate::StorageReply;
use crate::StorageRequest;
use crate::contract::CALL_METHOD;
use crate::contract::PrepareForkRequest;
use crate::contract::PreparedForkResponse;
use crate::contract::RELEASE_FORK_METHOD;
use crate::contract::ReleaseForkRequest;
use crate::migration_service::MigrationRegistry;

/// Implementation-neutral dispatcher. The process owns this service, all
/// accepted operation futures and its store until graceful shutdown completes.
/// Each handler must run concurrently so lease release can unblock deletion.
pub struct StorageService {
    pub(crate) store: Arc<dyn ThreadStore>,
    // Conservatively retain every opened ID. Removing after a completed native
    // shutdown races a concurrent resume which may have opened a new writer.
    pub(crate) open_threads: Mutex<HashSet<ThreadId>>,
    leases: Mutex<Leases>,
    pub(crate) migrations: MigrationRegistry,
}

#[derive(Default)]
struct Leases {
    closing: bool,
    entries: HashMap<String, ForkLease>,
}

enum ForkLease {
    Preparing,
    Ready(PreparedFork),
    // A paired control request can be handled before prepare starts or finishes.
    Released,
    // Keep failure until its guaranteed cleanup arrives; otherwise that cleanup
    // would look like a release preceding a not-yet-started prepare.
    Failed,
}

impl StorageService {
    pub fn new(store: Arc<dyn ThreadStore>) -> Self {
        Self {
            store,
            open_threads: Mutex::new(HashSet::new()),
            leases: Mutex::new(Leases::default()),
            migrations: MigrationRegistry::default(),
        }
    }

    /// Dispatch a typed request. Native application errors are serialized replies,
    /// while malformed messages and unknown methods are protocol errors.
    pub async fn handle(&self, method: &str, params: Value) -> Result<Value, String> {
        match method {
            CALL_METHOD => {
                let request: StorageRequest = serde_json::from_value(params)
                    .map_err(|err| format!("invalid thread-store request: {err}"))?;
                let reply = match self.execute(request).await {
                    Ok(response) => StorageReply::Ok(Box::new(response)),
                    Err(error) => StorageReply::Error(error.into()),
                };
                serde_json::to_value(reply).map_err(|err| err.to_string())
            }
            RELEASE_MIGRATION_METHOD => {
                let request: MigrationLeaseRequest = serde_json::from_value(params)
                    .map_err(|err| format!("invalid migration release: {err}"))?;
                self.migrations
                    .release(&request.lease_id)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(Value::Null)
            }
            RELEASE_FORK_METHOD => {
                let request: ReleaseForkRequest = serde_json::from_value(params)
                    .map_err(|err| format!("invalid fork release: {err}"))?;
                let mut leases = self.leases.lock().await;
                if !leases.closing {
                    match leases.entries.remove(&request.lease_id) {
                        Some(ForkLease::Ready(reservation)) => drop(reservation),
                        Some(ForkLease::Failed) => {}
                        Some(ForkLease::Preparing | ForkLease::Released) | None => {
                            leases.entries.insert(request.lease_id, ForkLease::Released);
                        }
                    }
                }
                Ok(Value::Null)
            }
            _ => Err(format!("unknown thread-store method: {method}")),
        }
    }

    pub(crate) async fn prepare_fork(
        &self,
        request: PrepareForkRequest,
    ) -> ThreadStoreResult<PreparedForkResponse> {
        {
            let mut leases = self.leases.lock().await;
            if leases.closing {
                return Err(closing_error());
            }
            match leases.entries.get(&request.lease_id) {
                None => {
                    leases
                        .entries
                        .insert(request.lease_id.clone(), ForkLease::Preparing);
                }
                Some(ForkLease::Released) => {}
                Some(ForkLease::Preparing | ForkLease::Ready(_) | ForkLease::Failed) => {
                    return Err(ThreadStoreError::Conflict {
                        message: "fork lease ID is already in use".to_owned(),
                    });
                }
            }
        }

        let prepared = self.store.prepare_fork(request.params).await;
        let mut leases = self.leases.lock().await;
        let previous = leases.entries.remove(&request.lease_id);
        if leases.closing {
            // Dropping a completed native reservation releases its source lock.
            drop(prepared);
            return Err(closing_error());
        }
        let released = matches!(previous, Some(ForkLease::Released));
        match prepared {
            Ok(prepared) => {
                let response = PreparedForkResponse {
                    source_thread_id: prepared.source_thread_id,
                    history_base: prepared.history_base,
                    model_context: StoredModelContext {
                        thread_id: prepared.source_thread_id,
                        items: prepared.model_context.as_ref().clone(),
                    },
                };
                if !released {
                    leases
                        .entries
                        .insert(request.lease_id, ForkLease::Ready(prepared));
                }
                Ok(response)
            }
            Err(error) => {
                if !released {
                    leases.entries.insert(request.lease_id, ForkLease::Failed);
                }
                Err(error)
            }
        }
    }

    /// Call before draining handlers on EOF/shutdown: an accepted delete may be
    /// waiting for one of these reservations. Pending prepares release on return.
    pub async fn release_all_forks(&self) {
        let mut leases = self.leases.lock().await;
        leases.closing = true;
        leases.entries.clear();
    }

    /// Fence and signal migrations before waiting for accepted request handlers.
    pub async fn begin_shutdown_migrations(&self) {
        self.migrations.begin_shutdown().await;
    }

    /// Join every retained migration after handlers settle, before state/store closure.
    pub async fn shutdown_migrations(&self) -> ThreadStoreResult<()> {
        self.migrations.finish_shutdown().await
    }

    /// Call after draining accepted handlers. Native shutdown preserves lazy
    /// preparations while fencing activated writers and surfacing write errors.
    pub async fn shutdown_writers(&self) -> ThreadStoreResult<()> {
        let thread_ids = self.open_threads.lock().await.drain().collect::<Vec<_>>();
        let mut failures = Vec::new();
        for thread_id in thread_ids {
            match self.store.shutdown_thread(thread_id).await {
                Ok(()) | Err(ThreadStoreError::ThreadNotFound { .. }) => {}
                Err(error) => failures.push(format!("{thread_id}: {error}")),
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(ThreadStoreError::Internal {
                message: format!("storage shutdown failed: {}", failures.join("; ")),
            })
        }
    }
}

fn closing_error() -> ThreadStoreError {
    ThreadStoreError::Conflict {
        message: "thread-store component is shutting down".to_owned(),
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
