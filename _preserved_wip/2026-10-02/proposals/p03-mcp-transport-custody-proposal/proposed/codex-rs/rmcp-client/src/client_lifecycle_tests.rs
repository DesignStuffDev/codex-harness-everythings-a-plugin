use super::*;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::transport_lifecycle::TransportCloseFailure;

struct ObservedTransport(Result<TransportCloseConfirmation, TransportCloseFailure>);

impl TransportCloseObserver for ObservedTransport {
    fn wait(&self) -> Pin<Box<dyn Future<Output = Result<TransportCloseConfirmation, TransportCloseFailure>> + Send + '_>> {
        Box::pin(async { self.0 })
    }
}

#[derive(Default)]
struct PendingProbe(AtomicUsize);

impl PendingShutdown for PendingProbe {
    fn begin_shutdown(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn is_registered(owner: &Arc<ClientLifecycle>) -> bool {
    CLIENTS.get_or_init(Default::default).lock().unwrap_or_else(PoisonError::into_inner)
        .iter().any(|candidate| Arc::ptr_eq(candidate, owner))
}

async fn bounded_close(owner: &Arc<ClientLifecycle>) -> anyhow::Result<ShutdownResult> {
    Ok(tokio::time::timeout(Duration::from_secs(5), owner.wait_closed()).await?)
}

#[tokio::test]
async fn late_generation_attaches_to_closed_roster_before_completion() -> anyhow::Result<()> {
    let owner = ClientLifecycle::new(tokio::runtime::Handle::current());
    let lease = owner.reserve()?;
    owner.begin_shutdown();
    assert!(matches!(owner.reserve(), Err(McpShutdownFailure::Closed)));
    let pending = Arc::new(PendingProbe::default());
    owner.attach_pending(pending.clone());
    owner.attach_transport(Arc::new(ObservedTransport(Ok(TransportCloseConfirmation::Confirmed))));
    assert_eq!(pending.0.load(Ordering::SeqCst), 1);
    assert!(tokio::time::timeout(Duration::from_millis(20), owner.wait_closed()).await.is_err());
    assert!(is_registered(&owner));
    drop(lease);
    assert_eq!(bounded_close(&owner).await?, Ok(McpShutdownConfirmation::Confirmed));
    assert!(!is_registered(&owner));
    Ok(())
}

#[tokio::test]
async fn cancelled_observer_does_not_cancel_final_owner() -> anyhow::Result<()> {
    let owner = ClientLifecycle::new(tokio::runtime::Handle::current());
    let lease = owner.reserve()?;
    assert!(tokio::time::timeout(Duration::from_millis(20), owner.wait_closed()).await.is_err());
    let first = owner.state.lock().unwrap_or_else(PoisonError::into_inner).completion.clone()
        .ok_or_else(|| anyhow::anyhow!("missing final owner"))?;
    drop(lease);
    assert_eq!(bounded_close(&owner).await?, Ok(McpShutdownConfirmation::Confirmed));
    let second = owner.state.lock().unwrap_or_else(PoisonError::into_inner).completion.clone()
        .ok_or_else(|| anyhow::anyhow!("missing repeated final owner"))?;
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(bounded_close(&owner).await?, Ok(McpShutdownConfirmation::Confirmed));
    Ok(())
}

#[tokio::test]
async fn unconfirmed_is_successful_operation_with_retained_evidence() -> anyhow::Result<()> {
    let owner = ClientLifecycle::new(tokio::runtime::Handle::current());
    owner.attach_transport(Arc::new(ObservedTransport(Ok(TransportCloseConfirmation::Unconfirmed))));
    assert_eq!(bounded_close(&owner).await?, Ok(McpShutdownConfirmation::Unconfirmed));
    owner.prune_clean().await;
    ClientLifecycle::observe_completed_clients().await;
    assert!(is_registered(&owner));
    assert_eq!(owner.state.lock().unwrap_or_else(PoisonError::into_inner).transports.len(), 1);
    Ok(())
}

#[tokio::test]
async fn known_close_error_remains_failed_and_registered() -> anyhow::Result<()> {
    let owner = ClientLifecycle::new(tokio::runtime::Handle::current());
    owner.attach_transport(Arc::new(ObservedTransport(Err(TransportCloseFailure::Transport))));
    assert_eq!(bounded_close(&owner).await?, Err(McpShutdownFailure::Transport));
    owner.prune_clean().await;
    assert!(is_registered(&owner));
    assert_eq!(owner.state.lock().unwrap_or_else(PoisonError::into_inner).transports.len(), 1);
    Ok(())
}

#[tokio::test]
async fn expected_operation_errors_retire_tasks_but_keep_original_result() -> anyhow::Result<()> {
    let owner = ClientLifecycle::new(tokio::runtime::Handle::current());
    let original = Arc::new(anyhow::anyhow!("expected unavailable optional server"));
    let outcome_error = Arc::clone(&original);
    let publisher = Arc::clone(&owner);
    let job = RetainedTask::spawn_published(owner.runtime(), async move {
        OperationCompletion::<()>(Err(outcome_error))
    }, move |task| publisher.track_job(task));
    let observed = tokio::time::timeout(Duration::from_secs(5), job.wait()).await??;
    assert!(observed.succeeded());
    match &observed.0 {
        Err(error) => assert!(Arc::ptr_eq(error, &original)),
        Ok(()) => anyhow::bail!("operation error disappeared"),
    }
    owner.prune_clean().await;
    assert!(owner.state.lock().unwrap_or_else(PoisonError::into_inner).jobs.is_empty());
    assert_eq!(bounded_close(&owner).await?, Ok(McpShutdownConfirmation::Confirmed));
    Ok(())
}

#[tokio::test]
async fn opportunistic_sweep_observes_drop_only_completion_without_closing_open_clients() -> anyhow::Result<()> {
    let owner = ClientLifecycle::new(tokio::runtime::Handle::current());
    let open = ClientLifecycle::new(tokio::runtime::Handle::current());
    owner.begin_shutdown();
    tokio::time::timeout(Duration::from_secs(5), async {
        while is_registered(&owner) {
            ClientLifecycle::observe_completed_clients().await;
            tokio::task::yield_now().await;
        }
    }).await?;
    assert!(is_registered(&open));
    assert!(!open.state.lock().unwrap_or_else(PoisonError::into_inner).closing);
    assert_eq!(bounded_close(&open).await?, Ok(McpShutdownConfirmation::Confirmed));
    Ok(())
}

#[tokio::test]
async fn clean_old_generation_rosters_are_pruned_between_attempts() -> anyhow::Result<()> {
    let owner = ClientLifecycle::new(tokio::runtime::Handle::current());
    for _ in 0..32 {
        owner.attach_transport(Arc::new(ObservedTransport(Ok(TransportCloseConfirmation::Confirmed))));
        owner.prune_clean().await;
        assert!(owner.state.lock().unwrap_or_else(PoisonError::into_inner).transports.is_empty());
    }
    assert_eq!(bounded_close(&owner).await?, Ok(McpShutdownConfirmation::Confirmed));
    Ok(())
}
