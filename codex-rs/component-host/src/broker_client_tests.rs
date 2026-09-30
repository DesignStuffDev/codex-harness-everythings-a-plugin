#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::future::Future;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::time::timeout;

use super::*;

struct Fixture {
    client: DependencyClient,
    scope: DependencyRequestScope,
    requests: mpsc::Receiver<BrokerOutgoing>,
    cancellations: mpsc::Receiver<BrokerCancellation>,
}

fn offer() -> BrokerOffer {
    BrokerOffer {
        version: 1,
        max_in_flight: BROKER_CAPACITY,
        services: vec![ServiceGrant {
            name: "host.model_endpoint".to_owned(),
            version: 1,
            operations: vec!["fetch".to_owned()],
            authority: "opaque-authority".to_owned(),
        }],
    }
}

fn requirement() -> HostDependencyDescriptor {
    HostDependencyDescriptor {
        name: "host.model_endpoint".to_owned(),
        version: 1,
        operations: vec!["fetch".to_owned()],
    }
}

impl Fixture {
    fn new() -> Self {
        let (outgoing, requests) = mpsc::channel(BROKER_CAPACITY);
        let (cancel, cancellations) = mpsc::channel(BROKER_CAPACITY);
        let (client, ack) =
            DependencyClient::negotiate(offer(), &[requirement()], outgoing, cancel).unwrap();
        assert_eq!(ack.services, vec![requirement()]);
        Self {
            client,
            requests,
            cancellations,
            scope: DependencyRequestScope {
                parent_id: 3,
                scope: DependencyScope {
                    handle: "opaque-context".to_owned(),
                    request_generation: 7,
                    owner_generation: 11,
                },
            },
        }
    }

    fn call(&self) -> tokio::task::JoinHandle<std::result::Result<Value, DependencyError>> {
        let client = self.client.clone();
        let scope = self.scope.clone();
        tokio::spawn(async move {
            client
                .call(&scope, "host.model_endpoint", 1, "fetch", json!({}))
                .await
        })
    }

    fn expect_immediate_rejection(&self, code: DependencyErrorCode) {
        let before = {
            let state = self.client.inner.pending.lock().unwrap();
            (state.next_id, state.calls.len())
        };
        let mut call =
            Box::pin(
                self.client
                    .call(&self.scope, "host.model_endpoint", 1, "fetch", json!({})),
            );
        let mut context = Context::from_waker(Waker::noop());
        assert_eq!(
            call.as_mut().poll(&mut context),
            Poll::Ready(Err(DependencyError::new(code)))
        );
        let state = self.client.inner.pending.lock().unwrap();
        assert_eq!((state.next_id, state.calls.len()), before);
    }
}

#[tokio::test]
async fn cancellation_reserves_capacity_until_response_and_preserves_send_fence() {
    let mut fixture = Fixture::new();
    let waiting = fixture.call();
    let BrokerOutgoing::Request { header, sent, .. } = fixture.requests.recv().await.unwrap()
    else {
        panic!("request expected")
    };
    assert_eq!(
        (
            header.id,
            header.parent_id,
            header.request_generation,
            header.expected_owner_generation
        ),
        (1, 3, 7, 11)
    );
    waiting.abort();
    assert!(waiting.await.unwrap_err().is_cancelled());
    let cancellation = fixture.cancellations.recv().await.unwrap();
    assert_eq!(cancellation.id, 1);
    assert!(Arc::ptr_eq(&sent, &cancellation.after_sent));
    assert!(!cancellation.after_sent.load(Ordering::Acquire));
    assert_eq!(fixture.client.pending_count(), 1);
    fixture
        .client
        .complete(1, Err(DependencyError::new(DependencyErrorCode::Cancelled)))
        .unwrap();
    assert_eq!(fixture.client.pending_count(), 0);
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn terminal_response_disarms_cancel_and_preserves_service_failure_payload() {
    let mut fixture = Fixture::new();
    let waiting = fixture.call();
    let BrokerOutgoing::Request { header, .. } = fixture.requests.recv().await.unwrap() else {
        panic!("request expected")
    };
    let value = json!({"outcome":"error","error":{"code":"rate_limited","retry_after_ms":1000}});
    fixture
        .client
        .complete(
            header.id,
            Ok(Payload::from_value(value.clone()).await.unwrap()),
        )
        .unwrap();
    assert_eq!(waiting.await.unwrap().unwrap(), value);
    assert!(fixture.cancellations.try_recv().is_err());
    assert_eq!(fixture.client.pending_count(), 0);
}

#[tokio::test]
async fn abandoned_callers_cannot_exceed_reverse_capacity_or_trigger_replay() {
    let mut fixture = Fixture::new();
    for expected in 1..=BROKER_CAPACITY as u64 {
        let waiting = fixture.call();
        let BrokerOutgoing::Request { header, .. } = fixture.requests.recv().await.unwrap() else {
            panic!("request expected")
        };
        assert_eq!(header.id, expected);
        waiting.abort();
        assert!(waiting.await.unwrap_err().is_cancelled());
        assert_eq!(fixture.cancellations.recv().await.unwrap().id, expected);
    }
    assert_eq!(fixture.client.pending_count(), BROKER_CAPACITY);
    fixture.expect_immediate_rejection(DependencyErrorCode::Busy);
    assert!(fixture.requests.try_recv().is_err());
    fixture
        .client
        .fail(DependencyError::new(DependencyErrorCode::AuthorityRevoked));
    assert_eq!(fixture.client.pending_count(), 0);
    assert_eq!(
        fixture.call().await.unwrap().unwrap_err().code,
        DependencyErrorCode::AuthorityRevoked
    );
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn cancellation_backlog_rejects_without_retaining_slots_or_consuming_ids() {
    let mut fixture = Fixture::new();
    for id in 1..=BROKER_CAPACITY as u64 {
        let waiting = fixture.call();
        let BrokerOutgoing::Request { header, .. } = fixture.requests.recv().await.unwrap() else {
            panic!("request expected")
        };
        assert_eq!(header.id, id);
        waiting.abort();
        assert!(waiting.await.unwrap_err().is_cancelled());
        fixture
            .client
            .complete(
                id,
                Err(DependencyError::new(DependencyErrorCode::Cancelled)),
            )
            .unwrap();
    }
    assert_eq!(fixture.client.pending_count(), 0);
    fixture.expect_immediate_rejection(DependencyErrorCode::Busy);
    assert_eq!(
        fixture.client.inner.slots.available_permits(),
        BROKER_CAPACITY
    );
    assert_eq!(fixture.client.inner.outgoing.capacity(), BROKER_CAPACITY);
    assert_eq!(fixture.cancellations.recv().await.unwrap().id, 1);
    let waiting = fixture.call();
    let BrokerOutgoing::Request { header, .. } = fixture.requests.recv().await.unwrap() else {
        panic!("request expected")
    };
    assert_eq!(header.id, BROKER_CAPACITY as u64 + 1);
    let failure = DependencyError::new(DependencyErrorCode::ServiceFailure);
    fixture
        .client
        .complete(header.id, Err(failure.clone()))
        .unwrap();
    assert_eq!(waiting.await.unwrap(), Err(failure));
    for id in 2..=BROKER_CAPACITY as u64 {
        assert_eq!(fixture.cancellations.recv().await.unwrap().id, id);
    }
    assert!(fixture.cancellations.try_recv().is_err());
}

#[tokio::test]
async fn full_writer_releases_tentative_slot_and_cancel_reservations() {
    let mut fixture = Fixture::new();
    let reservations: Vec<_> = (0..BROKER_CAPACITY)
        .map(|_| {
            fixture
                .client
                .inner
                .outgoing
                .clone()
                .try_reserve_owned()
                .unwrap()
        })
        .collect();
    fixture.expect_immediate_rejection(DependencyErrorCode::Busy);
    assert_eq!(
        fixture.client.inner.slots.available_permits(),
        BROKER_CAPACITY
    );
    assert_eq!(
        fixture.client.inner.cancellations.capacity(),
        BROKER_CAPACITY
    );
    assert!(fixture.requests.try_recv().is_err());
    drop(reservations);
    let waiting = fixture.call();
    let BrokerOutgoing::Request { header, .. } = fixture.requests.recv().await.unwrap() else {
        panic!("request expected")
    };
    assert_eq!(header.id, 1);
    let failure = DependencyError::new(DependencyErrorCode::ServiceFailure);
    fixture
        .client
        .complete(header.id, Err(failure.clone()))
        .unwrap();
    assert_eq!(waiting.await.unwrap(), Err(failure));
    assert!(fixture.cancellations.try_recv().is_err());
}

#[tokio::test]
async fn closed_admission_lanes_report_service_failure_without_admission() {
    let fixture = Fixture::new();
    fixture.client.inner.slots.close();
    fixture.expect_immediate_rejection(DependencyErrorCode::ServiceFailure);

    let mut fixture = Fixture::new();
    fixture.cancellations.close();
    fixture.expect_immediate_rejection(DependencyErrorCode::ServiceFailure);
    assert_eq!(
        fixture.client.inner.slots.available_permits(),
        BROKER_CAPACITY
    );

    let mut fixture = Fixture::new();
    fixture.requests.close();
    fixture.expect_immediate_rejection(DependencyErrorCode::ServiceFailure);
    assert_eq!(
        fixture.client.inner.slots.available_permits(),
        BROKER_CAPACITY
    );
    assert_eq!(
        fixture.client.inner.cancellations.capacity(),
        BROKER_CAPACITY
    );
}

#[tokio::test]
async fn drained_responses_release_cancel_capacity_even_if_waiters_are_not_polled() {
    let mut fixture = Fixture::new();
    let mut unpolled = Vec::new();
    for _ in 0..BROKER_CAPACITY + 1 {
        let mut call = Box::pin(fixture.client.call(
            &fixture.scope,
            "host.model_endpoint",
            1,
            "fetch",
            json!({}),
        ));
        std::future::poll_fn(|context| {
            assert!(call.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        let request = timeout(Duration::from_secs(1), fixture.requests.recv())
            .await
            .unwrap()
            .unwrap();
        let BrokerOutgoing::Request { header, .. } = request else {
            panic!("request expected")
        };
        fixture
            .client
            .complete(
                header.id,
                Err(DependencyError::new(DependencyErrorCode::ServiceFailure)),
            )
            .unwrap();
        unpolled.push(call);
    }
    assert_eq!(fixture.client.pending_count(), 0);
    drop(unpolled);
    assert!(fixture.cancellations.try_recv().is_err());
}

#[tokio::test]
async fn unavailable_or_unlisted_dependency_is_rejected_without_admission() {
    let fixture = Fixture::new();
    for (service, version, method, expected) in [
        (
            "host.arbitrary_http",
            1,
            "fetch",
            DependencyErrorCode::ServiceUnavailable,
        ),
        (
            "host.model_endpoint",
            2,
            "fetch",
            DependencyErrorCode::UnsupportedVersion,
        ),
        (
            "host.model_endpoint",
            1,
            "arbitrary_url",
            DependencyErrorCode::InvalidRequest,
        ),
    ] {
        assert_eq!(
            fixture
                .client
                .call(&fixture.scope, service, version, method, json!({}))
                .await
                .unwrap_err()
                .code,
            expected
        );
    }
    assert_eq!(fixture.client.pending_count(), 0);
}

#[test]
fn missing_required_dependency_and_versions_fail_negotiation() {
    for (mut offer, expected) in [
        (offer(), DependencyErrorCode::ServiceUnavailable),
        (offer(), DependencyErrorCode::UnsupportedVersion),
    ] {
        if expected == DependencyErrorCode::ServiceUnavailable {
            offer.services.clear();
        } else {
            offer.version = 2;
        }
        let (outgoing, _) = mpsc::channel(BROKER_CAPACITY);
        let (cancel, _) = mpsc::channel(BROKER_CAPACITY);
        let error =
            DependencyClient::negotiate(offer, &[requirement()], outgoing, cancel).unwrap_err();
        assert_eq!(error.code, expected);
    }
}

#[test]
fn remote_error_text_and_opaque_owner_are_not_diagnostic_output() {
    let error: DependencyError =
        serde_json::from_value(json!({"code":"service_failure","message":"SECRET_REMOTE_VALUE"}))
            .unwrap();
    assert_eq!(error.to_string(), "host dependency operation failed");
    assert!(!format!("{error:?}").contains("SECRET_REMOTE_VALUE"));
    let scope = DependencyScope {
        handle: "SECRET_HANDLE".to_owned(),
        request_generation: 1,
        owner_generation: 2,
    };
    assert!(!format!("{scope:?}").contains("SECRET_HANDLE"));
}
