//! Component-side dependency calls retain bounded response ownership after cancellation.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use anyhow::Result;
use anyhow::ensure;
use serde_json::Value;
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::Semaphore;
use tokio::sync::TryAcquireError;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::broker_api::BROKER_CAPACITY;
use crate::broker_api::BROKER_VERSION;
use crate::broker_api::DependencyError;
use crate::broker_api::DependencyErrorCode;
use crate::broker_api::DependencyScope;
use crate::broker_api::HostDependencyDescriptor;
use crate::broker_api::MAX_GRANTS;
use crate::broker_host::BrokerAcknowledgement;
use crate::broker_host::BrokerOffer;
use crate::broker_host::ServiceGrant;
use crate::broker_host::validate_descriptor;
use crate::broker_wire::BrokerRequestHeader;
use crate::broker_writer::BrokerCancellation;
use crate::broker_writer::BrokerOutgoing;
use crate::session_wire::Payload;

/// A dependency scope attached by the host to one admitted parent request.
/// Keep it with that request; completing the parent revokes its authority.
#[derive(Clone, Debug)]
pub struct DependencyRequestScope {
    pub(super) parent_id: u64,
    pub(super) scope: DependencyScope,
}

impl DependencyRequestScope {
    pub fn owner_generation(&self) -> u64 {
        self.scope.owner_generation
    }

    pub fn request_generation(&self) -> u64 {
        self.scope.request_generation
    }
}

type Completion = std::result::Result<Payload, DependencyError>;

struct Pending {
    response: oneshot::Sender<Completion>,
    _slot: OwnedSemaphorePermit,
    cancellation: CancellationPermit,
}

#[derive(Default)]
struct PendingState {
    next_id: u64,
    calls: BTreeMap<u64, Pending>,
    failure: Option<DependencyError>,
}

struct ClientState {
    grants: BTreeMap<(String, u32), ServiceGrant>,
    outgoing: mpsc::Sender<BrokerOutgoing>,
    cancellations: mpsc::Sender<BrokerCancellation>,
    slots: Arc<Semaphore>,
    pending: Mutex<PendingState>,
}

/// A cloneable component-side client for explicitly negotiated host dependencies.
/// Requests are read-only/cancellable in broker version one; no operation is retried.
#[derive(Clone)]
pub struct DependencyClient {
    inner: Arc<ClientState>,
}

impl std::fmt::Debug for DependencyClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DependencyClient")
            .finish_non_exhaustive()
    }
}

struct CancelOnDrop {
    id: u64,
    permit: CancellationPermit,
    after_sent: Arc<AtomicBool>,
}

type CancellationPermit = Arc<Mutex<Option<mpsc::OwnedPermit<BrokerCancellation>>>>;

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(permit) = self
            .permit
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            permit.send(BrokerCancellation {
                id: self.id,
                after_sent: Arc::clone(&self.after_sent),
            });
        }
    }
}

impl DependencyClient {
    pub(super) fn negotiate(
        offer: BrokerOffer,
        requirements: &[HostDependencyDescriptor],
        outgoing: mpsc::Sender<BrokerOutgoing>,
        cancellations: mpsc::Sender<BrokerCancellation>,
    ) -> std::result::Result<(Self, BrokerAcknowledgement), DependencyError> {
        if offer.version != BROKER_VERSION {
            return Err(DependencyError::new(
                DependencyErrorCode::UnsupportedVersion,
            ));
        }
        if offer.max_in_flight != BROKER_CAPACITY
            || offer.services.len() > MAX_GRANTS
            || requirements.len() > MAX_GRANTS
        {
            return Err(DependencyError::new(DependencyErrorCode::InvalidRequest));
        }
        let mut offered = BTreeMap::new();
        for grant in offer.services {
            let descriptor = HostDependencyDescriptor {
                name: grant.name.clone(),
                version: grant.version,
                operations: grant.operations.clone(),
            };
            validate_descriptor(&descriptor)
                .map_err(|_| DependencyError::new(DependencyErrorCode::InvalidRequest))?;
            if grant.authority.is_empty()
                || grant.authority.len() > 256
                || offered
                    .insert((grant.name.clone(), grant.version), grant)
                    .is_some()
            {
                return Err(DependencyError::new(DependencyErrorCode::InvalidRequest));
            }
        }
        let mut grants = BTreeMap::new();
        let mut services = Vec::new();
        for requirement in requirements {
            validate_descriptor(requirement)
                .map_err(|_| DependencyError::new(DependencyErrorCode::InvalidRequest))?;
            let key = (requirement.name.clone(), requirement.version);
            let Some(grant) = offered.get(&key) else {
                let code = if offered.keys().any(|(name, _)| name == &requirement.name) {
                    DependencyErrorCode::UnsupportedVersion
                } else {
                    DependencyErrorCode::ServiceUnavailable
                };
                return Err(DependencyError::new(code));
            };
            if !requirement
                .operations
                .iter()
                .all(|operation| grant.operations.contains(operation))
                || grants.insert(key, grant.clone()).is_some()
            {
                return Err(DependencyError::new(DependencyErrorCode::InvalidRequest));
            }
            services.push(HostDependencyDescriptor {
                name: grant.name.clone(),
                version: grant.version,
                operations: grant.operations.clone(),
            });
        }
        Ok((
            Self {
                inner: Arc::new(ClientState {
                    grants,
                    outgoing,
                    cancellations,
                    slots: Arc::new(Semaphore::new(BROKER_CAPACITY)),
                    pending: Mutex::new(PendingState::default()),
                }),
            },
            BrokerAcknowledgement {
                version: BROKER_VERSION,
                services,
            },
        ))
    }

    /// Invoke an allowlisted host operation. Dropping this future after admission queues
    /// cancellation, while the pending entry remains until a terminal response/disconnect.
    /// Saturated admission returns `Busy` immediately without consuming a request ID.
    pub async fn call(
        &self,
        scope: &DependencyRequestScope,
        service: &str,
        version: u32,
        method: &str,
        params: Value,
    ) -> std::result::Result<Value, DependencyError> {
        let grant = self
            .inner
            .grants
            .get(&(service.to_owned(), version))
            .ok_or_else(|| {
                DependencyError::new(
                    if self.inner.grants.keys().any(|(name, _)| name == service) {
                        DependencyErrorCode::UnsupportedVersion
                    } else {
                        DependencyErrorCode::ServiceUnavailable
                    },
                )
            })?;
        if !grant.operations.iter().any(|operation| operation == method) {
            return Err(DependencyError::new(DependencyErrorCode::InvalidRequest));
        }
        let (response, receiver) = oneshot::channel();
        let sent = Arc::new(AtomicBool::new(false));
        let _cancellation_guard = {
            let mut state = self
                .inner
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(error) = &state.failure {
                return Err(error.clone());
            }
            // Admission is synchronous and transactional: a full lane retains no
            // ownership, ID or waiter while holding the pending-state mutex.
            let slot = Arc::clone(&self.inner.slots)
                .try_acquire_owned()
                .map_err(|error| {
                    DependencyError::new(match error {
                        TryAcquireError::NoPermits => DependencyErrorCode::Busy,
                        TryAcquireError::Closed => DependencyErrorCode::ServiceFailure,
                    })
                })?;
            let cancel = self
                .inner
                .cancellations
                .clone()
                .try_reserve_owned()
                .map_err(|error| {
                    DependencyError::new(match error {
                        mpsc::error::TrySendError::Full(_) => DependencyErrorCode::Busy,
                        mpsc::error::TrySendError::Closed(_) => DependencyErrorCode::ServiceFailure,
                    })
                })?;
            let outgoing = self
                .inner
                .outgoing
                .clone()
                .try_reserve_owned()
                .map_err(|error| {
                    DependencyError::new(match error {
                        mpsc::error::TrySendError::Full(_) => DependencyErrorCode::Busy,
                        mpsc::error::TrySendError::Closed(_) => DependencyErrorCode::ServiceFailure,
                    })
                })?;
            let cancellation = Arc::new(Mutex::new(Some(cancel)));
            state.next_id = state
                .next_id
                .checked_add(1)
                .ok_or_else(|| DependencyError::new(DependencyErrorCode::ServiceFailure))?;
            let id = state.next_id;
            state.calls.insert(
                id,
                Pending {
                    response,
                    _slot: slot,
                    cancellation: Arc::clone(&cancellation),
                },
            );
            outgoing.send(BrokerOutgoing::Request {
                header: BrokerRequestHeader {
                    id,
                    service: service.to_owned(),
                    version,
                    method: method.to_owned(),
                    authority: grant.authority.clone(),
                    context: scope.scope.handle.clone(),
                    parent_id: scope.parent_id,
                    request_generation: scope.scope.request_generation,
                    expected_owner_generation: scope.scope.owner_generation,
                },
                params,
                sent: Arc::clone(&sent),
            });
            CancelOnDrop {
                id,
                permit: cancellation,
                after_sent: sent,
            }
        };
        let result = receiver
            .await
            .map_err(|_| DependencyError::new(DependencyErrorCode::AuthorityRevoked))?;
        result?
            .into_value()
            .await
            .map_err(|_| DependencyError::new(DependencyErrorCode::ServiceFailure))
    }

    pub(super) fn complete(&self, id: u64, result: Completion) -> Result<()> {
        let pending = self
            .inner
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .calls
            .remove(&id);
        ensure!(pending.is_some(), "unexpected dependency response ID");
        if let Some(pending) = pending {
            // Reader ownership releases capacity even if the answered waiter is never polled.
            pending
                .cancellation
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take();
            let _ = pending.response.send(result);
        }
        Ok(())
    }

    pub(super) fn fail(&self, error: DependencyError) {
        self.inner.slots.close();
        let pending = {
            let mut state = self
                .inner
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.failure = Some(error.clone());
            std::mem::take(&mut state.calls)
        };
        for (_, pending) in pending {
            pending
                .cancellation
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take();
            let _ = pending.response.send(Err(error.clone()));
        }
    }

    pub(super) fn pending_count(&self) -> usize {
        self.inner
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .calls
            .len()
    }
}

#[cfg(test)]
#[path = "broker_client_tests.rs"]
mod tests;
