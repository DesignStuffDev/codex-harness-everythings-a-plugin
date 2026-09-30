use std::sync::Arc;
use std::time::Duration;

use codex_protocol::ThreadId;
use codex_thread_store::*;
use serde_json::json;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;

use super::StorageService;
use crate::CALL_METHOD;
use crate::PrepareForkRequest;
use crate::RELEASE_FORK_METHOD;
use crate::StorageRequest;

struct DelayedForkStore {
    inner: Arc<InMemoryThreadStore>,
    source_reservation: Arc<RwLock<()>>,
    started: Semaphore,
    complete: Semaphore,
}

macro_rules! delegate {
    ($method:ident, $input:ty, $output:ty) => {
        fn $method(&self, params: $input) -> ThreadStoreFuture<'_, $output> {
            self.inner.$method(params)
        }
    };
}

impl ThreadStore for DelayedForkStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    delegate!(create_thread, CreateThreadParams, ());
    delegate!(resume_thread, ResumeThreadParams, ());
    delegate!(append_items, AppendThreadItemsParams, ());
    delegate!(flush_thread, ThreadId, ());
    delegate!(shutdown_thread, ThreadId, ());
    delegate!(discard_thread, ThreadId, ());
    delegate!(load_history, LoadThreadHistoryParams, StoredThreadHistory);
    delegate!(read_thread, ReadThreadParams, StoredThread);
    delegate!(
        read_thread_by_rollout_path,
        ReadThreadByRolloutPathParams,
        StoredThread
    );
    delegate!(list_threads, ListThreadsParams, ThreadPage);
    delegate!(
        update_thread_metadata,
        UpdateThreadMetadataParams,
        Option<StoredThread>
    );
    delegate!(archive_thread, ArchiveThreadParams, ());
    delegate!(unarchive_thread, ArchiveThreadParams, StoredThread);

    fn persist_thread(&self, id: ThreadId, context: PersistContext) -> ThreadStoreFuture<'_, ()> {
        self.inner.persist_thread(id, context)
    }

    fn prepare_fork(&self, params: PrepareForkParams) -> ThreadStoreFuture<'_, PreparedFork> {
        Box::pin(async move {
            let reservation = Arc::clone(&self.source_reservation).read_owned().await;
            self.started.add_permits(1);
            self.complete.acquire().await.unwrap().forget();
            Ok(PreparedFork::new(
                params.thread_id,
                None,
                Arc::new(Vec::new()),
                reservation,
            ))
        })
    }

    fn delete_thread(&self, _params: DeleteThreadParams) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            let _exclusive = self.source_reservation.write().await;
            Ok(())
        })
    }
}

#[tokio::test]
async fn release_before_native_prepare_completes_drops_late_reservation() {
    let id = uuid::Uuid::new_v4().to_string();
    let backend = Arc::new(DelayedForkStore {
        inner: InMemoryThreadStore::for_id(id.clone()),
        source_reservation: Arc::new(RwLock::new(())),
        started: Semaphore::new(0),
        complete: Semaphore::new(0),
    });
    let service = Arc::new(StorageService::new(backend.clone()));
    let thread_id = ThreadId::new();
    let preparing_service = Arc::clone(&service);
    let preparing = tokio::spawn(async move {
        preparing_service
            .handle(
                CALL_METHOD,
                serde_json::to_value(StorageRequest::PrepareFork(PrepareForkRequest {
                    lease_id: "cancelled-prepare".to_owned(),
                    params: PrepareForkParams {
                        thread_id,
                        boundary: ForkBoundary::Latest,
                    },
                }))
                .unwrap(),
            )
            .await
    });
    backend.started.acquire().await.unwrap().forget();
    service
        .handle(RELEASE_FORK_METHOD, json!({"lease_id":"cancelled-prepare"}))
        .await
        .unwrap();
    let deleting_service = Arc::clone(&service);
    let mut deleting = tokio::spawn(async move {
        deleting_service
            .handle(
                CALL_METHOD,
                serde_json::to_value(StorageRequest::DeleteThread(DeleteThreadParams {
                    thread_id,
                }))
                .unwrap(),
            )
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut deleting)
            .await
            .is_err()
    );
    backend.complete.add_permits(1);
    preparing.await.unwrap().unwrap();
    tokio::time::timeout(Duration::from_secs(2), deleting)
        .await
        .expect("late native reservation must be released")
        .unwrap()
        .unwrap();
    assert!(service.leases.lock().await.entries.is_empty());
    InMemoryThreadStore::remove_id(&id);
}
