//! Real stdio peer gates for the process adapter's pending-start authority.
//! These do not prove cooperative construction in an installed native worker.

use codex_file_search_api::SearchStartCancellationOutcome;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use pretty_assertions::assert_eq;

use super::*;

#[tokio::test]
async fn cancellation_before_first_poll_admits_no_wire_work_and_refunds() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let pending = backend.begin_open(fixture.request()).unwrap();
    let control = pending.control();
    assert_eq!(lock(&backend.inner.state).leases.len(), 1);
    // The synchronous request precedes the retained owner's first poll.
    let cancelled = control.cancel_and_wait();
    assert_eq!(
        cancelled.await,
        SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::NotAdmitted,
        }
    );
    assert_eq!(
        pending.await.err().unwrap(),
        SearchStartError {
            operation: super::super::state::closed(),
            cleanup: StartCleanup::NotAdmitted,
        }
    );
    assert_eq!(fixture.records("open"), "");
    assert_eq!(fixture.records("release"), "");
    assert_eq!(lock(&backend.inner.state).leases.len(), 0);
    let replacement = backend.open(fixture.request()).await.unwrap();
    assert_eq!(replacement.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

#[tokio::test]
#[expect(
    clippy::await_holding_invalid_type,
    reason = "hold the admission gate to prove cancellation of an actually blocked start does not send wire work"
)]
async fn cancellation_while_waiting_for_admission_does_not_consume_an_epoch() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let admission = backend.inner.admission.lock().await;
    let pending = backend.begin_open(fixture.request()).unwrap();
    let control = pending.control();
    let lease = lock(&backend.inner.state).leases[0].clone();
    // admit subscribes immediately before selecting the held production mutex.
    // No other owner has subscribed at this stage; this is not a timed guess.
    tokio::time::timeout(Duration::from_secs(5), async {
        while lease.stopping.receiver_count() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let expected = SearchStartCancellationOutcome {
        operation: Ok(()),
        cleanup: StartCleanup::NotAdmitted,
    };
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(
        pending.await.err().unwrap().cleanup,
        StartCleanup::NotAdmitted
    );
    assert_eq!(fixture.records("epochs"), "");
    assert_eq!(fixture.records("release"), "");
    drop(admission);
    let replacement = backend.open(fixture.request()).await.unwrap();
    assert_eq!(fixture.records("epochs"), "1\n");
    assert_eq!(replacement.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

#[tokio::test]
async fn cancelling_one_preparing_start_keeps_sibling_and_provider_usable() {
    let fixture = Fixture::new(json!({"hold_open_epoch":"1"}));
    let backend = fixture.connect().await;
    let pending = backend.begin_open(fixture.request()).unwrap();
    let control = pending.control();
    // Confirm this ticket owns epoch1 before scheduling its sibling; spawn order
    // alone does not establish which task first acquires transport admission.
    fixture.wait("open", 1).await;
    let sibling = backend.open(fixture.request()).await.unwrap();
    fixture.wait("open", 2).await;
    assert_eq!(
        sibling
            .update_query(query(1, "before"))
            .await
            .unwrap()
            .id
            .get(),
        1
    );
    let expected = SearchStartCancellationOutcome {
        operation: Ok(()),
        cleanup: StartCleanup::Confirmed,
    };
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(
        pending.await.err().unwrap().cleanup,
        StartCleanup::Confirmed
    );
    assert_eq!(fixture.records("released_epochs"), "1\n");
    assert_eq!(
        sibling
            .update_query(query(2, "after"))
            .await
            .unwrap()
            .id
            .get(),
        2
    );
    let replacement = backend.open(fixture.request()).await.unwrap();
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(
        replacement
            .update_query(query(1, "new"))
            .await
            .unwrap()
            .id
            .get(),
        1
    );
    assert_eq!(fixture.records("released_epochs"), "1\n");
    assert_eq!(sibling.close().await, joined());
    assert_eq!(replacement.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

#[tokio::test]
async fn cancellation_after_ready_reply_but_before_handoff_rejects_ready() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let pending = backend.begin_open(fixture.request()).unwrap();
    let control = pending.control();
    let lease = lock(&backend.inner.state).leases[0].clone();
    // Current-thread execution plus no await from setting limits to sender.send
    // ensures the real ready result is queued before cancelling its observer.
    tokio::time::timeout(Duration::from_secs(5), async {
        while lease.limits.get().is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    control.request_cancel();
    assert_eq!(
        pending.finish().await.err().unwrap(),
        SearchStartError {
            operation: super::super::state::closed(),
            cleanup: StartCleanup::Confirmed,
        }
    );
    assert_eq!(
        control.cancel_and_wait().await,
        SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::Confirmed,
        }
    );
    assert_eq!(fixture.records("released_epochs"), "1\n");
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

#[tokio::test]
async fn surviving_control_closes_only_its_already_handed_off_session() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let pending = backend.begin_open(fixture.request()).unwrap();
    let control = pending.control();
    let session = pending.finish().await.unwrap();
    let sibling = backend.open(fixture.request()).await.unwrap();
    assert_eq!(
        control.cancel_and_wait().await,
        SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::Confirmed,
        }
    );
    assert_eq!(session.close().await, joined());
    assert_eq!(
        sibling
            .update_query(query(1, "alive"))
            .await
            .unwrap()
            .id
            .get(),
        1
    );
    assert_eq!(fixture.records("released_epochs"), "1\n");
    assert_eq!(sibling.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

enum Abandon {
    Ticket,
    Finish,
    CancelObserver,
}

#[tokio::test]
async fn abandoned_observers_keep_one_release_owner_and_charged_lease() {
    for abandon in [Abandon::Ticket, Abandon::Finish, Abandon::CancelObserver] {
        let fixture = Fixture::new(json!({"hold_open":true,"hold_release":true}));
        let backend = fixture.connect().await;
        let pending = backend.begin_open(fixture.request()).unwrap();
        let control = pending.control();
        fixture.wait("open", 1).await;
        let pending = match abandon {
            Abandon::Ticket => {
                drop(pending);
                None
            }
            Abandon::Finish => {
                drop(pending.finish());
                None
            }
            Abandon::CancelObserver => {
                drop(control.cancel_and_wait());
                Some(pending)
            }
        };
        // No further cancel is requested until the abandoned observer itself
        // causes the real paired RELEASE to arrive at the peer.
        fixture.wait("release_started", 1).await;
        assert_eq!(lock(&backend.inner.state).leases.len(), 1);
        assert_eq!(fixture.records("released_epochs"), "1\n");
        fixture.gate("release_gate");
        let expected = SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::Confirmed,
        };
        assert_eq!(control.cancel_and_wait().await, expected);
        assert_eq!(control.cancel_and_wait().await, expected);
        if let Some(pending) = pending {
            assert_eq!(
                pending.await.err().unwrap().cleanup,
                StartCleanup::Confirmed
            );
        }
        assert_eq!(lock(&backend.inner.state).leases.len(), 0);
        assert_eq!(fixture.records("released_epochs"), "1\n");
        assert_eq!(backend.shutdown().await, joined());
        fixture.assert_reaped();
    }
}

#[tokio::test]
async fn per_start_cancel_preserves_release_failure_and_cleanup_independently() {
    for unconfirmed in [false, true] {
        let fixture =
            Fixture::new(json!({"hold_open":true,"resource_error":true,"unconfirmed":unconfirmed}));
        let backend = fixture.connect().await;
        let pending = backend.begin_open(fixture.request()).unwrap();
        let control = pending.control();
        fixture.wait("open", 1).await;
        let operation = SearchError::new(
            SearchErrorKind::ResourceExhausted,
            "original resource exhaustion",
        );
        let cleanup = if unconfirmed {
            CloseCleanup::Unconfirmed(SearchError::new(
                SearchErrorKind::TransportLost,
                "cleanup proof unavailable",
            ))
        } else {
            CloseCleanup::Joined
        };
        let expected = SearchStartCancellationOutcome {
            operation: Err(operation.clone()),
            cleanup: cleanup.clone().into(),
        };
        assert_eq!(control.cancel_and_wait().await, expected);
        assert_eq!(
            pending.await.err().unwrap(),
            SearchStartError {
                operation: operation.clone(),
                cleanup: expected.cleanup.clone(),
            }
        );
        assert_eq!(control.cancel_and_wait().await, expected);
        assert_eq!(
            backend.shutdown().await,
            SearchCloseOutcome {
                operation: Err(operation),
                cleanup
            }
        );
        assert_eq!(
            lock(&backend.inner.state).leases.len(),
            usize::from(unconfirmed)
        );
        fixture.assert_reaped();
    }
}

#[tokio::test]
async fn remote_not_admitted_requires_paired_route_cleanup_proof() {
    for unconfirmed in [false, true] {
        let fixture = Fixture::new(
            json!({"open_error":true,"open_not_admitted":true,"resource_error":true,"unconfirmed":unconfirmed}),
        );
        let backend = fixture.connect().await;
        let pending = backend.begin_open(fixture.request()).unwrap();
        let control = pending.control();
        let operation = SearchError::new(SearchErrorKind::SearchFailed, "earlier startup failure");
        let cleanup = if unconfirmed {
            StartCleanup::Unconfirmed(SearchError::new(
                SearchErrorKind::TransportLost,
                "cleanup proof unavailable",
            ))
        } else {
            StartCleanup::NotAdmitted
        };
        assert_eq!(
            pending.await.err().unwrap(),
            SearchStartError {
                operation: operation.clone(),
                cleanup: cleanup.clone(),
            }
        );
        let expected = SearchStartCancellationOutcome {
            operation: Err(operation.clone()),
            cleanup,
        };
        assert_eq!(control.cancel_and_wait().await, expected);
        assert_eq!(control.cancel_and_wait().await, expected);
        let cleanup = match expected.cleanup {
            StartCleanup::NotAdmitted | StartCleanup::Confirmed => CloseCleanup::Joined,
            StartCleanup::Unconfirmed(error) => CloseCleanup::Unconfirmed(error),
        };
        assert_eq!(
            backend.shutdown().await,
            SearchCloseOutcome {
                operation: Err(operation),
                cleanup
            }
        );
        assert_eq!(
            lock(&backend.inner.state).leases.len(),
            usize::from(unconfirmed)
        );
        fixture.assert_reaped();
    }
}

#[tokio::test]
async fn dropped_unpolled_owner_publishes_uncertainty_and_keeps_reservation() {
    let fixture = Fixture::new(json!({}));
    // Keep the actual process provider on this live runtime so its independent
    // containment owner can still shut down and reap the peer normally.
    let backend = fixture.connect().await;
    let owner = Arc::clone(&backend);
    let request = fixture.request();
    let pending = tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let pending = {
            let _entered = runtime.enter();
            owner.begin_open(request).unwrap()
        };
        // This runtime was never driven. Drop must run the preconstructed
        // completion guard without executing the accepted startup task body.
        drop(runtime);
        pending
    })
    .await
    .unwrap();
    let control = pending.control();
    let lost = super::super::state::lost();
    let expected = SearchStartCancellationOutcome {
        operation: Err(lost.clone()),
        cleanup: StartCleanup::Unconfirmed(lost.clone()),
    };
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(
        pending.await.err().unwrap(),
        SearchStartError {
            operation: lost.clone(),
            cleanup: expected.cleanup.clone(),
        }
    );
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(fixture.records("open"), "");
    assert_eq!(fixture.records("release"), "");
    assert_eq!(lock(&backend.inner.state).leases.len(), 1);
    assert_eq!(
        backend.shutdown().await,
        SearchCloseOutcome {
            operation: Err(lost.clone()),
            cleanup: CloseCleanup::Unconfirmed(lost),
        }
    );
    assert_eq!(lock(&backend.inner.state).leases.len(), 1);
    fixture.assert_reaped();
}

#[tokio::test]
async fn pre_wire_admission_failure_survives_later_cancellation_observation() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let mut request = fixture.request();
    request.options.limit =
        std::num::NonZeroUsize::new(backend.negotiated_limits().max_matches as usize + 1).unwrap();
    let pending = backend.begin_open(request).unwrap();
    let control = pending.control();
    let operation = SearchError::new(
        SearchErrorKind::ResourceExhausted,
        "file-search match limit exceeds negotiated ceiling",
    );
    assert_eq!(
        pending.await.err().unwrap(),
        SearchStartError {
            operation: operation.clone(),
            cleanup: StartCleanup::NotAdmitted,
        }
    );
    assert_eq!(
        control.cancel_and_wait().await,
        SearchStartCancellationOutcome {
            operation: Err(operation.clone()),
            cleanup: StartCleanup::NotAdmitted,
        }
    );
    assert_eq!(fixture.records("open"), "");
    assert_eq!(fixture.records("release"), "");
    assert_eq!(lock(&backend.inner.state).leases.len(), 0);
    let replacement = backend.open(fixture.request()).await.unwrap();
    assert_eq!(replacement.close().await, joined());
    assert_eq!(
        backend.shutdown().await,
        SearchCloseOutcome {
            operation: Err(operation),
            cleanup: CloseCleanup::Joined,
        }
    );
    fixture.assert_reaped();
}

#[tokio::test]
async fn remote_closed_start_keeps_its_cause_over_later_release_and_cancel() {
    let fixture = Fixture::new(json!({
        "open_error":true,
        "open_closed_error":true,
        "resource_error":true,
        "hold_release":true,
    }));
    let backend = fixture.connect().await;
    let pending = backend.begin_open(fixture.request()).unwrap();
    let control = pending.control();
    // No caller cancellation has occurred. The remote OPEN independently fails
    // with ClosedLease and its automatic cleanup reaches the gated RELEASE.
    fixture.wait("release_started", 1).await;
    let cancelled = control.cancel_and_wait();
    fixture.gate("release_gate");
    let operation = SearchError::new(SearchErrorKind::ClosedLease, "earlier startup failure");
    let expected = SearchStartCancellationOutcome {
        operation: Err(operation.clone()),
        cleanup: StartCleanup::Confirmed,
    };
    assert_eq!(
        pending.await.err().unwrap(),
        SearchStartError {
            operation: operation.clone(),
            cleanup: StartCleanup::Confirmed,
        }
    );
    assert_eq!(cancelled.await, expected);
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(fixture.records("released_epochs"), "1\n");
    assert_eq!(
        backend.shutdown().await,
        SearchCloseOutcome {
            operation: Err(operation),
            cleanup: CloseCleanup::Joined,
        }
    );
    fixture.assert_reaped();
}
