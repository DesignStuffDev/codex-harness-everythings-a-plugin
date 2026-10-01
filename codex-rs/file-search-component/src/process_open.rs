//! Admission ordering and startup ownership for the process backend.

use std::sync::Arc;

use codex_component_host::DeferredControl;
use codex_component_host::PendingComponentReply;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use tokio::sync::oneshot;

use crate::FILE_SEARCH_CONTRACT_VERSION;
use crate::LeaseIdentity;
use crate::LeaseRequest;
use crate::OPEN_METHOD;
use crate::OpenRequest;
use crate::OpenResponse;
use crate::RELEASE_METHOD;
use crate::WireBudget;
use crate::WireCloseOutcome;
use crate::WireU64;

use super::decode::decode;
use super::decode::encode;
use super::decode::malformed;
use super::decode::transport_error;
use super::not_admitted;
use super::session::ProcessSession;
use super::state::CancelOnDrop;
use super::state::Lease;
use super::state::LeaseOwnerGuard;
use super::state::Provider;
use super::state::closed;
use super::state::lock;
use super::state::lost;
use super::state::uncertain;

type OpenResult = Result<Arc<dyn SearchBackendSession>, SearchStartError>;

pub(super) async fn open(provider: Arc<Provider>, request: SearchOpen) -> OpenResult {
    // Reject an obviously oversized caller-owned value before retaining or
    // cloning it. Exact codec/JSON size is additionally checked at admission.
    // Charging each entry also bounds vectors containing only empty strings.
    let mut remaining = provider.limits.max_roots_options_bytes as usize;
    for bytes in request
        .roots
        .iter()
        .map(|root| root.as_os_str().len())
        .chain(request.options.exclude.iter().map(String::len))
    {
        remaining = bytes
            .checked_add(1)
            .and_then(|bytes| remaining.checked_sub(bytes))
            .ok_or_else(|| {
                not_admitted(SearchError::new(
                    SearchErrorKind::ResourceExhausted,
                    "file-search roots and options exceed the retained input limit",
                ))
            })?;
    }
    let lease = provider.reserve(request).map_err(not_admitted)?;
    let mut cancellation = CancelOnDrop(Some(lease.stopping.clone()));
    let (sender, receiver) = oneshot::channel();
    let mut guard = LeaseOwnerGuard(Some(Arc::clone(&lease)));
    tokio::spawn(async move {
        run_open(provider, lease, sender).await;
        guard.0.take();
    });
    let result = receiver.await.unwrap_or_else(|_| {
        Err(SearchStartError {
            operation: lost(),
            cleanup: StartCleanup::Unconfirmed(lost()),
        })
    });
    cancellation.0.take();
    result
}

#[expect(
    clippy::await_holding_invalid_type,
    reason = "serialize epoch allocation and bounded transport admission only; replies and cleanup never hold this gate"
)]
async fn admit(
    provider: &Provider,
    lease: &Lease,
) -> Result<(PendingComponentReply, DeferredControl), SearchStartError> {
    let mut stopping = lease.stopping.subscribe();
    let gate = tokio::select! {
        biased;
        _ = stopping.wait_for(|stopping| *stopping) => return Err(not_admitted(closed())),
        gate = provider.admission.lock() => gate,
    };
    let identity = {
        let mut state = lock(&provider.state);
        if state.closing || *stopping.borrow() {
            return Err(not_admitted(closed()));
        }
        state.epoch = state.epoch.checked_add(1).ok_or_else(|| {
            not_admitted(SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "file-search session epochs exhausted",
            ))
        })?;
        LeaseIdentity {
            provider_id: provider.identity.provider_id.clone(),
            lease_id: uuid::Uuid::new_v4().to_string(),
            session_epoch: WireU64(state.epoch),
        }
    };
    let request =
        OpenRequest::from_native(identity.clone(), lease.request.clone()).map_err(not_admitted)?;
    request.validate(&provider.limits).map_err(not_admitted)?;
    let params = encode(&request).map_err(not_admitted)?;
    let release = encode(&LeaseRequest {
        contract_version: FILE_SEARCH_CONTRACT_VERSION,
        identity: identity.clone(),
    })
    .map_err(not_admitted)?;
    lease
        .identity
        .set(identity)
        .map_err(|_| not_admitted(malformed()))?;
    let admitted = tokio::select! {
        biased;
        _ = stopping.wait_for(|stopping| *stopping) => Err(not_admitted(closed())),
        result = provider.transport.start_with_cleanup(OPEN_METHOD, params, RELEASE_METHOD, release) => {
            result.map_err(|error| not_admitted(transport_error(&error)))
        }
    };
    // Admission, paired handle ownership and releasing the ordering gate are in
    // one uninterrupted poll. Startup response observation never holds the gate.
    drop(gate);
    admitted
}

async fn run_open(provider: Arc<Provider>, lease: Arc<Lease>, sender: oneshot::Sender<OpenResult>) {
    let (reply, cleanup) = match admit(&provider, &lease).await {
        Ok(handles) => handles,
        Err(error) => {
            lease.request_close();
            lease.finish(SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Joined,
            });
            let _ = sender.send(Err(error));
            return;
        }
    };
    let mut stopping = lease.stopping.subscribe();
    let opened = tokio::select! {
        biased;
        _ = stopping.wait_for(|stopping| *stopping) => Err(SearchStartError {
            operation: closed(), cleanup: StartCleanup::Unconfirmed(lost()),
        }),
        result = reply.wait() => match result {
            Ok(value) => {
                let decoded = lease.identity.get().ok_or_else(malformed)
                    .and_then(|identity| decode::<_, OpenResponse, SearchStartError>(value, OPEN_METHOD, identity));
                match decoded {
                    Ok(result) => result,
                    Err(error) => {
                        provider.fail(error.clone());
                        Err(SearchStartError { operation: error, cleanup: StartCleanup::Unconfirmed(lost()) })
                    }
                }
            }
            Err(error) => {
                let error = transport_error(&error);
                provider.fail(error.clone());
                Err(SearchStartError { operation: error, cleanup: StartCleanup::Unconfirmed(lost()) })
            }
        }
    }.and_then(|response| {
        response.limits.validate_against(&provider.limits).map_err(|operation| SearchStartError {
            operation, cleanup: StartCleanup::Unconfirmed(lost()),
        })?;
        if response.initial_cursor.0 != 0
            || response.budget != WireBudget::from_native(lease.request.budget)
            || (response.limits.max_matches as usize) < lease.request.options.limit.get()
        {
            let operation = malformed();
            provider.fail(operation.clone());
            return Err(SearchStartError { operation, cleanup: StartCleanup::Unconfirmed(lost()) });
        }
        lease.limits.set(response.limits).map_err(|_| SearchStartError {
            operation: malformed(), cleanup: StartCleanup::Unconfirmed(lost()),
        })?;
        Ok(())
    });
    match opened {
        Ok(()) => {
            let session: Arc<dyn SearchBackendSession> = Arc::new(ProcessSession {
                inner: Arc::clone(&lease),
            });
            let _ = sender.send(Ok(session));
            let _ = stopping.wait_for(|stopping| *stopping).await;
            lease.request_close();
            let outcome = release_lease(cleanup, &lease).await;
            if matches!(&outcome.cleanup, CloseCleanup::Unconfirmed(_)) {
                provider.request_shutdown();
            }
            lease.drain_operations().await;
            lease.finish(outcome);
        }
        Err(mut error) => {
            lease.request_close();
            let outcome = release_lease(cleanup, &lease).await;
            lease.drain_operations().await;
            // An explicit NotAdmitted receipt already proves no native work.
            // Still consume the guard so rejected-open metadata can be retired.
            if error.cleanup == StartCleanup::NotAdmitted {
                lease.finish(SearchCloseOutcome {
                    operation: Ok(()),
                    cleanup: CloseCleanup::Joined,
                });
            } else {
                if matches!(&outcome.cleanup, CloseCleanup::Unconfirmed(_)) {
                    provider.request_shutdown();
                }
                error.cleanup = outcome.cleanup.clone().into();
                if error.operation.kind() != SearchErrorKind::ClosedLease {
                    lease.fail(error.operation.clone());
                }
                lease.finish(outcome);
            }
            let _ = sender.send(Err(error));
        }
    }
}

async fn release_lease(cleanup: DeferredControl, lease: &Lease) -> SearchCloseOutcome {
    let Some(identity) = lease.identity.get() else {
        return uncertain(malformed());
    };
    match cleanup.release_reply().await {
        Ok(value) => {
            match decode::<_, WireCloseOutcome, SearchError>(value, RELEASE_METHOD, identity) {
                Ok(Ok(outcome)) => outcome.into(),
                Ok(Err(error)) | Err(error) => uncertain(error),
            }
        }
        Err(error) => uncertain(transport_error(&error)),
    }
}
