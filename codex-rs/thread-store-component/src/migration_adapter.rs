//! Reserved cleanup follows an accepted migration even when its start reply is lost.

use codex_component_host::DeferredControl;
use codex_thread_store::RolloutMigrationOptions;
use codex_thread_store::RolloutMigrationReport;
use codex_thread_store::RolloutMigrationRun;
use codex_thread_store::RolloutMigrationSnapshot;
use codex_thread_store::ThreadStoreError;
use codex_thread_store::ThreadStoreFuture;
use codex_thread_store::ThreadStoreResult;

use crate::ProcessThreadStore;
use crate::StorageRequest;
use crate::StorageResponse;
use crate::adapter::decode_reply;
use crate::contract::CALL_METHOD;
use crate::contract::MigrationLeaseRequest;
use crate::contract::RELEASE_MIGRATION_METHOD;
use crate::contract::StartMigrationRequest;
use crate::error::transport_error;

pub(crate) async fn start(
    store: &ProcessThreadStore,
    options: RolloutMigrationOptions,
) -> ThreadStoreResult<Box<dyn RolloutMigrationRun>> {
    if !store.capabilities().manual_rollout_migration {
        return Err(ThreadStoreError::Unsupported { operation: "manual_rollout_migration" });
    }
    let lease = MigrationLeaseRequest { lease_id: uuid::Uuid::new_v4().to_string() };
    let params = serde_json::to_value(StorageRequest::StartMigration(StartMigrationRequest {
        lease_id: lease.lease_id.clone(), options,
    })).map_err(transport_error)?;
    let cleanup = serde_json::to_value(&lease).map_err(transport_error)?;
    let (value, release) = store.session.call_with_cleanup(
        CALL_METHOD, params, RELEASE_MIGRATION_METHOD, cleanup,
    ).await.map_err(transport_error)?;
    match decode_reply(value)? {
        StorageResponse::StartMigration(()) => Ok(Box::new(ProcessRun {
            store: store.clone(), lease, release: Some(release),
        })),
        _ => Err(transport_error("migration start response does not match request")),
    }
}

struct ProcessRun {
    store: ProcessThreadStore,
    lease: MigrationLeaseRequest,
    // Unique ownership; destructor queues the pre-reserved control request.
    release: Option<DeferredControl>,
}

impl RolloutMigrationRun for ProcessRun {
    fn snapshot(&self) -> ThreadStoreFuture<'_, RolloutMigrationSnapshot> {
        Box::pin(async {
            match self.store.request(StorageRequest::MigrationSnapshot(self.lease.clone())).await? {
                StorageResponse::MigrationSnapshot(snapshot) => Ok(snapshot),
                _ => Err(transport_error("migration snapshot response does not match request")),
            }
        })
    }

    fn report(&self) -> ThreadStoreFuture<'_, RolloutMigrationReport> {
        Box::pin(async {
            match self.store.request(StorageRequest::MigrationReport(self.lease.clone())).await? {
                StorageResponse::MigrationReport(report) => Ok(report),
                _ => Err(transport_error("migration report response does not match request")),
            }
        })
    }

    fn cancel(&self) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async {
            match self.store.request(StorageRequest::CancelMigration(self.lease.clone())).await? {
                StorageResponse::CancelMigration(()) => Ok(()),
                _ => Err(transport_error("migration cancellation response does not match request")),
            }
        })
    }

    fn close(mut self: Box<Self>) -> ThreadStoreFuture<'static, ()> {
        let release = self.release.take();
        Box::pin(async move {
            // Keep the store owner until the release acknowledgement is observed.
            let _owner = self;
            match release {
                Some(release) => release.release().await.map_err(transport_error),
                None => Err(transport_error("migration lease was already released")),
            }
        })
    }
}
