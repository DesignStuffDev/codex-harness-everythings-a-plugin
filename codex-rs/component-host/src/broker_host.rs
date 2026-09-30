//! Per-connection grants and owner-scoped leaf-service dispatch.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use anyhow::Result;
use anyhow::ensure;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::Semaphore;
use tokio::sync::watch;
use uuid::Uuid;

use crate::broker_api::BROKER_CAPACITY;
use crate::broker_api::BROKER_VERSION;
use crate::broker_api::DependencyCancellation;
use crate::broker_api::DependencyContext;
use crate::broker_api::DependencyError;
use crate::broker_api::DependencyErrorCode;
use crate::broker_api::DependencyScope;
use crate::broker_api::HostDependencyAuthority;
use crate::broker_api::HostDependencyDescriptor;
use crate::broker_api::HostDependencyService;
use crate::broker_api::HostOperationScope;
use crate::broker_api::MAX_GRANTS;
use crate::broker_api::OperationState;
use crate::broker_wire::BrokerRequestHeader;

tokio::task_local! {
    pub(super) static IN_DEPENDENCY_HANDLER: ();
}

#[derive(Clone)]
struct Registration {
    descriptor: HostDependencyDescriptor,
    implementation: Arc<dyn HostDependencyService>,
    authority: Arc<dyn HostDependencyAuthority>,
    required: bool,
}

/// Explicit dependencies available to one binding. Registration does not resolve
/// credentials or instantiate any ambient provider/network capability.
#[derive(Default)]
pub struct HostDependencyRegistry {
    registrations: BTreeMap<(String, u32), Registration>,
}

impl HostDependencyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        descriptor: HostDependencyDescriptor,
        implementation: Arc<dyn HostDependencyService>,
        authority: Arc<dyn HostDependencyAuthority>,
    ) -> Result<()> {
        self.register_with_requirement(descriptor, implementation, authority, true)
    }

    pub fn register_optional(
        &mut self,
        descriptor: HostDependencyDescriptor,
        implementation: Arc<dyn HostDependencyService>,
        authority: Arc<dyn HostDependencyAuthority>,
    ) -> Result<()> {
        self.register_with_requirement(descriptor, implementation, authority, false)
    }

    fn register_with_requirement(
        &mut self,
        descriptor: HostDependencyDescriptor,
        implementation: Arc<dyn HostDependencyService>,
        authority: Arc<dyn HostDependencyAuthority>,
        required: bool,
    ) -> Result<()> {
        ensure!(
            self.registrations.len() < MAX_GRANTS,
            "too many host dependency grants"
        );
        validate_descriptor(&descriptor)?;
        let key = (descriptor.name.clone(), descriptor.version);
        ensure!(
            !self.registrations.contains_key(&key),
            "duplicate host dependency service/version"
        );
        self.registrations.insert(
            key,
            Registration {
                descriptor,
                implementation,
                authority,
                required,
            },
        );
        Ok(())
    }
}

pub(super) fn validate_descriptor(descriptor: &HostDependencyDescriptor) -> Result<()> {
    ensure!(
        !descriptor.name.is_empty() && descriptor.name.len() <= 128,
        "host dependency name must contain 1 to 128 bytes"
    );
    ensure!(
        descriptor.version > 0,
        "host dependency version must be positive"
    );
    ensure!(
        !descriptor.operations.is_empty() && descriptor.operations.len() <= 32,
        "host dependency must declare 1 to 32 operations"
    );
    let mut seen = BTreeSet::new();
    for operation in &descriptor.operations {
        ensure!(
            !operation.is_empty() && operation.len() <= 256,
            "host dependency operation must contain 1 to 256 bytes"
        );
        ensure!(
            seen.insert(operation),
            "duplicate host dependency operation"
        );
    }
    Ok(())
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ServiceGrant {
    pub name: String,
    pub version: u32,
    pub operations: Vec<String>,
    pub authority: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BrokerOffer {
    pub version: u32,
    pub max_in_flight: usize,
    pub services: Vec<ServiceGrant>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BrokerAcknowledgement {
    pub version: u32,
    pub services: Vec<HostDependencyDescriptor>,
}

struct HostState {
    grants: BTreeMap<String, Registration>,
    accepted: Mutex<BTreeSet<String>>,
    contexts: Mutex<BTreeMap<String, Arc<OperationState>>>,
    next_generation: AtomicU64,
    capacity: Arc<Semaphore>,
    closed: AtomicBool,
}

#[derive(Clone)]
pub(super) struct BrokerHost {
    inner: Arc<HostState>,
}

impl BrokerHost {
    pub fn new(registry: HostDependencyRegistry) -> Self {
        let grants = registry
            .registrations
            .into_values()
            .map(|registration| (format!("g-{}", Uuid::new_v4().simple()), registration))
            .collect();
        Self {
            inner: Arc::new(HostState {
                grants,
                accepted: Mutex::new(BTreeSet::new()),
                contexts: Mutex::new(BTreeMap::new()),
                next_generation: AtomicU64::new(0),
                capacity: Arc::new(Semaphore::new(BROKER_CAPACITY)),
                closed: AtomicBool::new(false),
            }),
        }
    }

    pub fn offer(&self) -> BrokerOffer {
        BrokerOffer {
            version: BROKER_VERSION,
            max_in_flight: BROKER_CAPACITY,
            services: self
                .inner
                .grants
                .iter()
                .map(|(handle, registration)| ServiceGrant {
                    name: registration.descriptor.name.clone(),
                    version: registration.descriptor.version,
                    operations: registration.descriptor.operations.clone(),
                    authority: handle.clone(),
                })
                .collect(),
        }
    }

    pub fn acknowledge(
        &self,
        acknowledgement: BrokerAcknowledgement,
    ) -> Result<(), DependencyError> {
        if acknowledgement.version != BROKER_VERSION {
            return Err(DependencyError::new(
                DependencyErrorCode::UnsupportedVersion,
            ));
        }
        if acknowledgement.services.len() > MAX_GRANTS {
            return Err(DependencyError::new(DependencyErrorCode::InvalidRequest));
        }
        let mut accepted = BTreeSet::new();
        for descriptor in &acknowledgement.services {
            validate_descriptor(descriptor)
                .map_err(|_| DependencyError::new(DependencyErrorCode::InvalidRequest))?;
            let Some((handle, registration)) =
                self.inner.grants.iter().find(|(_, registration)| {
                    registration.descriptor.name == descriptor.name
                        && registration.descriptor.version == descriptor.version
                })
            else {
                let code = if self
                    .inner
                    .grants
                    .values()
                    .any(|registration| registration.descriptor.name == descriptor.name)
                {
                    DependencyErrorCode::UnsupportedVersion
                } else {
                    DependencyErrorCode::ServiceUnavailable
                };
                return Err(DependencyError::new(code));
            };
            // An acknowledgement cannot expand or silently narrow the operation contract.
            if !accepted.insert(handle.clone())
                || descriptor.operations.iter().collect::<BTreeSet<_>>()
                    != registration
                        .descriptor
                        .operations
                        .iter()
                        .collect::<BTreeSet<_>>()
            {
                return Err(DependencyError::new(DependencyErrorCode::InvalidRequest));
            }
        }
        if self
            .inner
            .grants
            .iter()
            .any(|(handle, registration)| registration.required && !accepted.contains(handle))
        {
            return Err(DependencyError::new(
                DependencyErrorCode::ServiceUnavailable,
            ));
        }
        *self
            .inner
            .accepted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = accepted;
        Ok(())
    }

    pub fn scope(
        &self,
        parent_id: u64,
        scope: HostOperationScope,
    ) -> Result<DependencyScope, DependencyError> {
        if parent_id == 0 || *scope.cancellation.sender.borrow() {
            return Err(DependencyError::new(DependencyErrorCode::Cancelled));
        }
        if tokio::time::Instant::now() >= scope.deadline {
            return Err(DependencyError::new(DependencyErrorCode::DeadlineExceeded));
        }
        let mut contexts = self
            .inner
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(DependencyError::new(DependencyErrorCode::AuthorityRevoked));
        }
        if contexts.len() >= crate::session::CALL_SLOTS
            || contexts
                .values()
                .any(|context| context.parent_id == parent_id)
        {
            return Err(DependencyError::new(DependencyErrorCode::Busy));
        }
        let generation = self
            .inner
            .next_generation
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| DependencyError::new(DependencyErrorCode::ServiceFailure))?
            + 1;
        let handle = format!("c-{}", Uuid::new_v4().simple());
        let owner_generation = scope.owner.generation;
        contexts.insert(
            handle.clone(),
            Arc::new(OperationState {
                parent_id,
                generation,
                scope,
                finished: DependencyCancellation::new(),
            }),
        );
        Ok(DependencyScope {
            handle,
            request_generation: generation,
            owner_generation,
        })
    }

    pub fn finish_parent(&self, parent_id: u64) {
        self.revoke_parent(parent_id);
        self.release_parent(parent_id);
    }

    /// Close authority synchronously, retaining the owner/context until joins finish.
    pub fn revoke_parent(&self, parent_id: u64) {
        for context in self
            .inner
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
        {
            if context.parent_id == parent_id {
                context.finished.cancel();
            }
        }
    }

    /// Only the parent completion owner may call this after reverse handlers join.
    pub fn release_parent(&self, parent_id: u64) {
        self.inner
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|_, context| context.parent_id != parent_id);
    }

    pub fn scope_for_parent(&self, parent_id: u64) -> Option<DependencyScope> {
        self.inner
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .find(|(_, context)| context.parent_id == parent_id)
            .map(|(handle, context)| DependencyScope {
                handle: handle.clone(),
                request_generation: context.generation,
                owner_generation: context.scope.owner.generation,
            })
    }

    pub fn check_request(&self, request: &BrokerRequestHeader) -> Result<(), DependencyError> {
        self.validate(request).map(|_| ())
    }

    pub fn context_for_request(
        &self,
        request: &BrokerRequestHeader,
    ) -> Result<DependencyContext, DependencyError> {
        self.validate(request).map(|(_, context)| context)
    }

    pub fn revoke_all(&self) {
        self.inner.closed.store(true, Ordering::Release);
        self.inner.capacity.close();
        for context in self
            .inner
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
        {
            context.finished.cancel();
        }
    }

    pub fn release_all(&self) {
        self.inner
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    fn validate(
        &self,
        request: &BrokerRequestHeader,
    ) -> Result<(Registration, DependencyContext), DependencyError> {
        let accepted = self
            .inner
            .accepted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(&request.authority);
        let registration = self
            .inner
            .grants
            .get(&request.authority)
            .filter(|_| accepted)
            .ok_or_else(|| DependencyError::new(DependencyErrorCode::AuthorityRevoked))?;
        if registration.descriptor.name != request.service {
            return Err(DependencyError::new(
                DependencyErrorCode::ServiceUnavailable,
            ));
        }
        if registration.descriptor.version != request.version {
            return Err(DependencyError::new(
                DependencyErrorCode::UnsupportedVersion,
            ));
        }
        if !registration.descriptor.operations.contains(&request.method) {
            return Err(DependencyError::new(DependencyErrorCode::InvalidRequest));
        }
        let operation = self
            .inner
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&request.context)
            .cloned()
            .ok_or_else(|| DependencyError::new(DependencyErrorCode::AuthorityRevoked))?;
        if operation.parent_id != request.parent_id
            || operation.generation != request.request_generation
        {
            return Err(DependencyError::new(DependencyErrorCode::AuthorityRevoked));
        }
        if operation.scope.owner.generation != request.expected_owner_generation {
            return Err(DependencyError::new(DependencyErrorCode::OwnerChanged));
        }
        let context = DependencyContext {
            operation,
            authority: Arc::clone(&registration.authority),
        };
        context.check_authority()?;
        Ok((registration.clone(), context))
    }

    pub async fn run(
        &self,
        request: BrokerRequestHeader,
        params: Value,
        mut cancelled: watch::Receiver<bool>,
    ) -> Result<(Value, DependencyContext), DependencyError> {
        let _slot = Arc::clone(&self.inner.capacity)
            .try_acquire_owned()
            .map_err(|_| DependencyError::new(DependencyErrorCode::Busy))?;
        let (registration, context) = self.validate(&request)?;
        let result = IN_DEPENDENCY_HANDLER.scope((), async {
            let operation = registration.implementation.call(context.clone(), request.method, params);
            tokio::select! {
                biased;
                _ = cancelled.wait_for(|cancelled| *cancelled) =>
                    Err(DependencyError::new(DependencyErrorCode::Cancelled)),
                _ = context.cancelled() => Err(context.check_authority().err()
                    .unwrap_or_else(|| DependencyError::new(DependencyErrorCode::AuthorityRevoked))),
                result = operation => result,
            }
        }).await;
        context.check_authority()?;
        Ok((result?, context))
    }
}

#[cfg(test)]
#[path = "broker_host_tests.rs"]
mod tests;
