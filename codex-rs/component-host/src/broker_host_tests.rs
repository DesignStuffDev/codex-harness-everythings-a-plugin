#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio::time::timeout;

use super::*;
use crate::broker_api::DependencyFuture;
use crate::broker_api::DependencyOwner;

struct Authority(watch::Sender<DependencyOwner>);

impl HostDependencyAuthority for Authority {
    fn check(&self, owner: &DependencyOwner) -> std::result::Result<(), DependencyError> {
        if *self.0.borrow() == *owner {
            Ok(())
        } else {
            Err(DependencyError::new(DependencyErrorCode::OwnerChanged))
        }
    }

    fn revoked(&self, owner: DependencyOwner) -> DependencyFuture<'_, ()> {
        Box::pin(async move {
            let _ = self
                .0
                .subscribe()
                .wait_for(|current| *current != owner)
                .await;
        })
    }
}

#[derive(Default)]
struct Service {
    calls: AtomicUsize,
    entered: Notify,
    block: bool,
}

impl HostDependencyService for Service {
    fn call(
        &self,
        context: DependencyContext,
        method: String,
        params: Value,
    ) -> DependencyFuture<'_, std::result::Result<Value, DependencyError>> {
        Box::pin(async move {
            context.check_authority()?;
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.entered.notify_one();
            if self.block {
                std::future::pending::<()>().await;
            }
            Ok(json!({"method":method,"params":params,"owner":context.owner_generation()}))
        })
    }
}

fn owner(generation: u64) -> DependencyOwner {
    DependencyOwner {
        generation,
        identity: Some(format!("identity-{generation}")),
    }
}

fn fixture(
    block: bool,
) -> (
    BrokerHost,
    Arc<Authority>,
    Arc<Service>,
    BrokerRequestHeader,
) {
    let authority = Arc::new(Authority(watch::channel(owner(1)).0));
    let service = Arc::new(Service {
        block,
        ..Service::default()
    });
    let descriptor = HostDependencyDescriptor {
        name: "host.model_endpoint".to_owned(),
        version: 1,
        operations: vec!["fetch".to_owned(), "validate_owner".to_owned()],
    };
    let mut registry = HostDependencyRegistry::new();
    registry
        .register(descriptor.clone(), service.clone(), authority.clone())
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
            HostOperationScope {
                owner: owner(1),
                deadline: Instant::now() + Duration::from_secs(5),
                cancellation: DependencyCancellation::new(),
                request_context: Some(Arc::new("request-local-factory".to_owned())),
            },
        )
        .unwrap();
    let header = BrokerRequestHeader {
        id: 1,
        service: grant.name,
        version: grant.version,
        method: "fetch".to_owned(),
        authority: grant.authority,
        context: scope.handle,
        parent_id: 1,
        request_generation: scope.request_generation,
        expected_owner_generation: scope.owner_generation,
    };
    (host, authority, service, header)
}

#[tokio::test]
async fn successful_dependency_preserves_typed_payload_and_owner_context() {
    let (host, _, service, header) = fixture(false);
    let (_cancel, receiver) = watch::channel(false);
    let (value, context) = host
        .run(
            header,
            json!({"outcome":{"error":"rate_limited"}}),
            receiver,
        )
        .await
        .unwrap();
    assert_eq!(
        value,
        json!({"method":"fetch","params":{"outcome":{"error":"rate_limited"}},"owner":1})
    );
    assert_eq!(context.owner_identity(), Some("identity-1"));
    assert_eq!(
        context.request_context::<String>().map(String::as_str),
        Some("request-local-factory")
    );
    assert_eq!(service.calls.load(Ordering::SeqCst), 1);
    host.finish_parent(1);
    assert_eq!(
        context.check_authority().unwrap_err().code,
        DependencyErrorCode::Cancelled
    );
}

#[tokio::test]
async fn scope_and_grant_cannot_be_reused_for_another_parent_session_or_owner() {
    let (host, _, service, header) = fixture(false);
    let (other_host, _, _, _) = fixture(false);
    let (_cancel, receiver) = watch::channel(false);
    assert_eq!(
        other_host
            .run(header.clone(), json!({}), receiver.clone())
            .await
            .unwrap_err()
            .code,
        DependencyErrorCode::AuthorityRevoked
    );
    for (mut changed, expected) in [
        (header.clone(), DependencyErrorCode::AuthorityRevoked),
        (header.clone(), DependencyErrorCode::AuthorityRevoked),
        (header.clone(), DependencyErrorCode::OwnerChanged),
        (header.clone(), DependencyErrorCode::UnsupportedVersion),
        (header.clone(), DependencyErrorCode::InvalidRequest),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (mut header, code))| {
        match index {
            0 => header.parent_id += 1,
            1 => header.request_generation += 1,
            2 => header.expected_owner_generation += 1,
            3 => header.version += 1,
            4 => header.method = "arbitrary_url".to_owned(),
            _ => unreachable!(),
        }
        (header, code)
    }) {
        changed.id = 2;
        assert_eq!(
            host.run(changed, json!({}), receiver.clone())
                .await
                .unwrap_err()
                .code,
            expected
        );
    }
    assert_eq!(service.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn live_owner_change_cancels_in_flight_service_without_replay() {
    let (host, authority, service, header) = fixture(true);
    let (_cancel, receiver) = watch::channel(false);
    let operation = tokio::spawn(async move { host.run(header, json!({}), receiver).await });
    timeout(Duration::from_secs(1), service.entered.notified())
        .await
        .unwrap();
    authority.0.send_replace(owner(2));
    let error = timeout(Duration::from_secs(1), operation)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, DependencyErrorCode::OwnerChanged);
    assert_eq!(service.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn reverse_capacity_is_bounded_and_parent_finish_releases_all_calls() {
    let (host, _, service, header) = fixture(true);
    let (_cancel, receiver) = watch::channel(false);
    let mut tasks = JoinSet::new();
    for id in 1..=BROKER_CAPACITY as u64 {
        let host = host.clone();
        let mut header = header.clone();
        header.id = id;
        let receiver = receiver.clone();
        tasks.spawn(async move { host.run(header, json!({}), receiver).await });
        timeout(Duration::from_secs(1), service.entered.notified())
            .await
            .unwrap();
    }
    assert_eq!(
        host.run(header, json!({}), receiver)
            .await
            .unwrap_err()
            .code,
        DependencyErrorCode::Busy
    );
    host.finish_parent(1);
    timeout(Duration::from_secs(1), async {
        while let Some(result) = tasks.join_next().await {
            assert_eq!(
                result.unwrap().unwrap_err().code,
                DependencyErrorCode::Cancelled
            );
        }
    })
    .await
    .unwrap();
    assert_eq!(service.calls.load(Ordering::SeqCst), BROKER_CAPACITY);
}

#[tokio::test]
async fn cancellation_and_deadline_are_owned_explicitly() {
    let (host, _, service, header) = fixture(true);
    let (cancel, receiver) = watch::channel(false);
    let operation = tokio::spawn(async move { host.run(header, json!({}), receiver).await });
    timeout(Duration::from_secs(1), service.entered.notified())
        .await
        .unwrap();
    cancel.send_replace(true);
    assert_eq!(
        timeout(Duration::from_secs(1), operation)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .code,
        DependencyErrorCode::Cancelled
    );
    let (host, _, _, _) = fixture(false);
    assert_eq!(
        host.scope(
            2,
            HostOperationScope {
                owner: owner(1),
                deadline: Instant::now(),
                cancellation: DependencyCancellation::new(),
                request_context: None
            }
        )
        .unwrap_err()
        .code,
        DependencyErrorCode::DeadlineExceeded
    );
}

#[test]
fn missing_required_acknowledgement_fails_before_activation() {
    let (host, _, _, _) = fixture(false);
    assert_eq!(
        host.acknowledge(BrokerAcknowledgement {
            version: 1,
            services: vec![]
        })
        .unwrap_err()
        .code,
        DependencyErrorCode::ServiceUnavailable
    );
    assert_eq!(
        host.acknowledge(BrokerAcknowledgement {
            version: 2,
            services: vec![]
        })
        .unwrap_err()
        .code,
        DependencyErrorCode::UnsupportedVersion
    );
}

#[test]
fn offered_service_wrong_version_is_distinct_from_missing_service() {
    let (host, _, _, _) = fixture(false);
    let mut descriptor = HostDependencyDescriptor {
        name: "host.model_endpoint".into(),
        version: 2,
        operations: vec!["fetch".into(), "validate_owner".into()],
    };
    assert_eq!(
        host.acknowledge(BrokerAcknowledgement {
            version: 1,
            services: vec![descriptor.clone()]
        })
        .unwrap_err()
        .code,
        DependencyErrorCode::UnsupportedVersion
    );
    descriptor.name = "host.unavailable".into();
    assert_eq!(
        host.acknowledge(BrokerAcknowledgement {
            version: 1,
            services: vec![descriptor]
        })
        .unwrap_err()
        .code,
        DependencyErrorCode::ServiceUnavailable
    );
}

#[test]
fn revoked_connection_rejects_new_scopes_and_keeps_old_owner_until_join_release() {
    let (host, _, _, header) = fixture(false);
    host.revoke_all();
    assert!(host.scope_for_parent(1).is_some());
    assert_eq!(
        host.check_request(&header).unwrap_err().code,
        DependencyErrorCode::Cancelled
    );
    assert_eq!(
        host.scope(
            2,
            HostOperationScope::new(owner(1), Instant::now() + Duration::from_secs(5))
        )
        .unwrap_err()
        .code,
        DependencyErrorCode::AuthorityRevoked
    );
    host.release_all();
    assert!(host.scope_for_parent(1).is_none());
}
