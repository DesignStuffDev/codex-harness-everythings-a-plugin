#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tokio::sync::Semaphore;
use tokio::sync::oneshot;
use tokio::time::Instant;
use tokio::time::timeout;

use super::*;
use crate::broker_api::DependencyContext;
use crate::broker_api::DependencyFuture;
use crate::broker_api::DependencyOwner;
use crate::broker_api::HostDependencyAuthority;
use crate::broker_api::HostDependencyDescriptor;
use crate::broker_api::HostDependencyService;
use crate::broker_api::HostOperationScope;
use crate::broker_completion::ParentCompletions;
use crate::broker_host::BrokerAcknowledgement;
use crate::broker_host::HostDependencyRegistry;
use crate::session::CALL_SLOTS;
use crate::session::Pending;
use crate::session::PendingState;
use crate::session::Shared;

struct Authority;
impl HostDependencyAuthority for Authority {
    fn check(&self, _: &DependencyOwner) -> Result<(), DependencyError> {
        Ok(())
    }
    fn revoked(&self, _: DependencyOwner) -> DependencyFuture<'_, ()> {
        Box::pin(std::future::pending())
    }
}

struct Gate {
    entered: Mutex<Option<oneshot::Sender<()>>>,
    dropping: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
    dropped: AtomicBool,
    calls: AtomicUsize,
}

struct DropGate(Arc<Gate>);
impl Drop for DropGate {
    fn drop(&mut self) {
        if let Some(sender) = self.0.dropping.lock().unwrap().take() {
            let _ = sender.send(());
        }
        let _ = self.0.release.lock().unwrap().recv();
        self.0.dropped.store(true, Ordering::Release);
    }
}

struct Service(Arc<Gate>);
impl HostDependencyService for Service {
    fn call(
        &self,
        _: DependencyContext,
        _: String,
        _: Value,
    ) -> DependencyFuture<'_, Result<Value, DependencyError>> {
        Box::pin(async move {
            let _drop = DropGate(Arc::clone(&self.0));
            self.0.calls.fetch_add(1, Ordering::Relaxed);
            if let Some(sender) = self.0.entered.lock().unwrap().take() {
                let _ = sender.send(());
            }
            std::future::pending().await
        })
    }
}

struct Release(Option<std::sync::mpsc::Sender<()>>);
impl Drop for Release {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

struct Fixture {
    host: BrokerHost,
    dispatcher: BrokerDispatcher,
    header: BrokerRequestHeader,
    responses: mpsc::Receiver<BrokerResponse>,
    gate: Arc<Gate>,
    entered: oneshot::Receiver<()>,
    dropping: oneshot::Receiver<()>,
    release: Release,
}

fn fixture() -> Fixture {
    let (entered_tx, entered) = oneshot::channel();
    let (dropping_tx, dropping) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let gate = Arc::new(Gate {
        entered: Mutex::new(Some(entered_tx)),
        dropping: Mutex::new(Some(dropping_tx)),
        release: Mutex::new(release_rx),
        dropped: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
    });
    let descriptor = HostDependencyDescriptor {
        name: "host.test".into(),
        version: 1,
        operations: vec!["read".into()],
    };
    let mut registry = HostDependencyRegistry::new();
    registry
        .register(
            descriptor.clone(),
            Arc::new(Service(Arc::clone(&gate))),
            Arc::new(Authority),
        )
        .unwrap();
    let host = BrokerHost::new(registry);
    let grant = host.offer().services.remove(0);
    host.acknowledge(BrokerAcknowledgement {
        version: 1,
        services: vec![descriptor],
    })
    .unwrap();
    let scope = host
        .scope(
            1,
            HostOperationScope::new(
                DependencyOwner {
                    generation: 1,
                    identity: None,
                },
                Instant::now() + Duration::from_secs(30),
            ),
        )
        .unwrap();
    let header = BrokerRequestHeader {
        id: 1,
        parent_id: 1,
        authority: grant.authority,
        service: grant.name,
        version: 1,
        method: "read".into(),
        context: scope.handle,
        request_generation: scope.request_generation,
        expected_owner_generation: 1,
    };
    let (responses_tx, responses) = mpsc::channel(BROKER_CAPACITY);
    let dispatcher = BrokerDispatcher::new(host.clone(), responses_tx);
    Fixture {
        host,
        dispatcher,
        header,
        responses,
        gate,
        entered,
        dropping,
        release: Release(Some(release_tx)),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parent_terminal_retains_response_slot_and_context_until_reverse_join_and_flush() {
    parent_terminal_waits_for_flush(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_response_flush_fails_parent_instead_of_publishing_received_success() {
    parent_terminal_waits_for_flush(true).await;
}

async fn parent_terminal_waits_for_flush(failed_flush: bool) {
    let mut fixture = fixture();
    let shared = Arc::new(Shared {
        pending: Mutex::new(PendingState::default()),
        regular_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
        control_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
        closing: AtomicBool::new(false),
        changed: Notify::new(),
        completion: watch::channel(None).0,
    });
    let (response, mut receiver) = oneshot::channel();
    let slot = shared.regular_slots.clone().try_acquire_owned().unwrap();
    shared.pending.lock().unwrap().calls.insert(
        1,
        Pending {
            response,
            _slot: slot,
        },
    );
    fixture
        .dispatcher
        .dispatch(
            fixture.header.clone(),
            Payload::from_value(json!({})).await.unwrap(),
        )
        .unwrap();
    timeout(Duration::from_secs(2), fixture.entered)
        .await
        .unwrap()
        .unwrap();
    let parents = ParentCompletions::new();
    parents
        .complete(
            1,
            Ok(Payload::from_value(json!({"parent":"finished"}))
                .await
                .unwrap()),
            Arc::clone(&shared),
            fixture.dispatcher.clone(),
        )
        .unwrap();
    timeout(Duration::from_secs(2), fixture.dropping)
        .await
        .unwrap()
        .unwrap();
    assert!(!fixture.dispatcher.is_idle());
    assert!(!parents.is_idle());
    assert!(fixture.host.scope_for_parent(1).is_some());
    assert!(shared.pending.lock().unwrap().calls.contains_key(&1));
    assert_eq!(shared.regular_slots.available_permits(), CALL_SLOTS - 1);
    assert!(matches!(
        receiver.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    assert!(
        parents
            .complete(
                1,
                Err("duplicate".into()),
                Arc::clone(&shared),
                fixture.dispatcher.clone()
            )
            .is_err()
    );
    fixture.header.id = 2;
    fixture
        .dispatcher
        .dispatch(
            fixture.header,
            Payload::from_value(json!({})).await.unwrap(),
        )
        .unwrap();
    let rejected = fixture.responses.recv().await.unwrap();
    let _ = rejected.completed.send(Ok(()));
    match rejected.message {
        BrokerOutgoing::Error { id: 2, error } => {
            assert_eq!(error.code, DependencyErrorCode::Cancelled)
        }
        _ => panic!("completed parent accepted another reverse handler"),
    }
    assert_eq!(fixture.gate.calls.load(Ordering::Relaxed), 1);
    drop(fixture.release);
    let reply = timeout(Duration::from_secs(2), fixture.responses.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(fixture.gate.dropped.load(Ordering::Acquire));
    assert!(!fixture.dispatcher.is_idle());
    assert!(fixture.host.scope_for_parent(1).is_some());
    assert_eq!(shared.regular_slots.available_permits(), CALL_SLOTS - 1);
    assert!(matches!(
        receiver.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    reply
        .completed
        .send(if failed_flush {
            Err("writer failed".into())
        } else {
            Ok(())
        })
        .unwrap();
    let result = timeout(Duration::from_secs(2), receiver)
        .await
        .unwrap()
        .unwrap();
    if failed_flush {
        assert!(result.err().unwrap().contains("response delivery failed"));
    } else {
        assert_eq!(
            result.unwrap().into_value().await.unwrap(),
            json!({"parent":"finished"})
        );
    }
    parents.drained().await;
    assert!(fixture.gate.dropped.load(Ordering::Acquire));
    assert!(fixture.dispatcher.is_idle());
    assert!(fixture.host.scope_for_parent(1).is_none());
    assert_eq!(shared.regular_slots.available_permits(), CALL_SLOTS);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_all_never_reports_idle_before_handler_destruction_and_response_flush() {
    let mut fixture = fixture();
    fixture
        .dispatcher
        .dispatch(
            fixture.header,
            Payload::from_value(json!({})).await.unwrap(),
        )
        .unwrap();
    timeout(Duration::from_secs(2), fixture.entered)
        .await
        .unwrap()
        .unwrap();
    fixture.dispatcher.cancel_all();
    timeout(Duration::from_secs(2), fixture.dropping)
        .await
        .unwrap()
        .unwrap();
    assert!(!fixture.dispatcher.is_idle());
    assert!(!fixture.dispatcher.stop_if_idle());
    assert!(fixture.host.scope_for_parent(1).is_some());
    drop(fixture.release);
    let reply = timeout(Duration::from_secs(2), fixture.responses.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(fixture.gate.dropped.load(Ordering::Acquire));
    assert!(!fixture.dispatcher.is_idle());
    reply.completed.send(Ok(())).unwrap();
    timeout(Duration::from_secs(2), fixture.dispatcher.drained())
        .await
        .unwrap();
    assert!(fixture.gate.dropped.load(Ordering::Acquire));
    assert!(fixture.dispatcher.stop_if_idle());
    fixture.dispatcher.release_all();
    assert!(fixture.host.scope_for_parent(1).is_none());
}
