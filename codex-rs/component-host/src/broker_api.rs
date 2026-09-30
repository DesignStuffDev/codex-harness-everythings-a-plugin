//! Explicit host dependencies for persistent components. These types carry no credentials.

use std::any::Any;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::watch;
use tokio::time::Instant;

pub(super) const BROKER_VERSION: u32 = 1;
pub(super) const BROKER_CAPACITY: usize = 16;
pub(super) const MAX_GRANTS: usize = 32;
/// Initial leaf-service request parameter limit; history and responses are uncapped.
pub(super) const MAX_BROKER_REQUEST_BYTES: u64 = 8 * 1024 * 1024;

/// Boxed dependency work keeps implementations object-safe without an async-trait macro.
pub type DependencyFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Stable failures from a host dependency, separate from a broken transport.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyErrorCode {
    ServiceUnavailable,
    UnsupportedVersion,
    AuthorityRevoked,
    OwnerChanged,
    Cancelled,
    DeadlineExceeded,
    Busy,
    InvalidRequest,
    RequestTooLarge,
    ReentrantCall,
    ServiceFailure,
}

impl DependencyErrorCode {
    fn message(self) -> &'static str {
        match self {
            Self::ServiceUnavailable => "host dependency is unavailable",
            Self::UnsupportedVersion => "host dependency version is unsupported",
            Self::AuthorityRevoked => "host dependency authority was revoked",
            Self::OwnerChanged => "host dependency owner changed",
            Self::Cancelled => "host dependency operation was cancelled",
            Self::DeadlineExceeded => "host dependency deadline expired",
            Self::Busy => "host dependency capacity is exhausted",
            Self::InvalidRequest => "invalid host dependency request",
            Self::RequestTooLarge => "host dependency request exceeds the 8 MiB parameter limit",
            Self::ReentrantCall => "host dependency reentrancy is unsupported",
            Self::ServiceFailure => "host dependency operation failed",
        }
    }
}

/// Secret-safe transport failure. Diagnostics are canonical text, not provider payloads.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyError {
    pub code: DependencyErrorCode,
    message: String,
}

impl DependencyError {
    pub fn new(code: DependencyErrorCode) -> Self {
        Self {
            code,
            message: code.message().to_owned(),
        }
    }
}

impl fmt::Display for DependencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A remote peer's message is untrusted and can contain credential material.
        formatter.write_str(self.code.message())
    }
}

impl fmt::Debug for DependencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DependencyError")
            .field("code", &self.code)
            .finish()
    }
}

impl std::error::Error for DependencyError {}

/// The service and operation names explicitly allowed for one installed binding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostDependencyDescriptor {
    pub name: String,
    pub version: u32,
    pub operations: Vec<String>,
}

/// Ownership follows provider/account identity, separately from access-token rotation.
#[derive(Clone, Eq, PartialEq)]
pub struct DependencyOwner {
    pub generation: u64,
    pub identity: Option<String>,
}

impl fmt::Debug for DependencyOwner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DependencyOwner")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

/// Live host authority. Implementations must check current ownership and wake on revocation.
/// A token refresh that preserves the owner must not incorrectly revoke a catalog request.
pub trait HostDependencyAuthority: Send + Sync {
    fn check(&self, owner: &DependencyOwner) -> Result<(), DependencyError>;

    /// Complete only when this owner loses authority; unrelated changes keep waiting.
    fn revoked(&self, owner: DependencyOwner) -> DependencyFuture<'_, ()>;
}

/// An explicitly registered leaf service. Implementations may use host-native dependencies
/// but must not synchronously call back into a selected component from this operation.
/// This also applies to spawned helper tasks; task-local checks cannot infer their ancestry.
pub trait HostDependencyService: Send + Sync {
    fn call(
        &self,
        context: DependencyContext,
        method: String,
        params: Value,
    ) -> DependencyFuture<'_, Result<Value, DependencyError>>;
}

/// Explicit cancellation authority for one host operation. Dropping a clone does not cancel.
#[derive(Clone)]
pub struct DependencyCancellation {
    pub(super) sender: watch::Sender<bool>,
}

impl DependencyCancellation {
    pub fn new() -> Self {
        Self {
            sender: watch::channel(false).0,
        }
    }

    pub fn cancel(&self) {
        self.sender.send_replace(true);
    }
}

impl Default for DependencyCancellation {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DependencyCancellation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DependencyCancellation")
            .field("cancelled", &*self.sender.borrow())
            .finish()
    }
}

/// Host-issued ownership and lifetime supplied when admitting a scoped parent call.
/// Accepted work retains this scope after an individual call waiter disappears.
#[derive(Clone)]
pub struct HostOperationScope {
    pub owner: DependencyOwner,
    pub deadline: Instant,
    pub cancellation: DependencyCancellation,
    /// Host-only request dependencies, such as the live per-call HTTP client factory.
    /// This value is never serialized or included in diagnostics.
    pub request_context: Option<Arc<dyn Any + Send + Sync>>,
}

impl HostOperationScope {
    pub fn new(owner: DependencyOwner, deadline: Instant) -> Self {
        Self {
            owner,
            deadline,
            cancellation: DependencyCancellation::new(),
            request_context: None,
        }
    }

    pub fn with_request_context<T: Any + Send + Sync>(mut self, context: T) -> Self {
        self.request_context = Some(Arc::new(context));
        self
    }
}

impl fmt::Debug for HostOperationScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostOperationScope")
            .field("owner", &self.owner)
            .field("deadline", &self.deadline)
            .field("cancellation", &self.cancellation)
            .finish_non_exhaustive()
    }
}

/// Opaque context delivered to a component with a scoped parent request.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyScope {
    pub handle: String,
    pub request_generation: u64,
    pub owner_generation: u64,
}

impl fmt::Debug for DependencyScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DependencyScope")
            .field("request_generation", &self.request_generation)
            .field("owner_generation", &self.owner_generation)
            .finish_non_exhaustive()
    }
}

pub(super) struct OperationState {
    pub parent_id: u64,
    pub generation: u64,
    pub scope: HostOperationScope,
    pub finished: DependencyCancellation,
}

/// Validated request metadata for a host service. It never exposes another service's grant.
#[derive(Clone)]
pub struct DependencyContext {
    pub(super) operation: Arc<OperationState>,
    pub(super) authority: Arc<dyn HostDependencyAuthority>,
}

impl DependencyContext {
    pub fn owner_generation(&self) -> u64 {
        self.operation.scope.owner.generation
    }

    pub fn owner_identity(&self) -> Option<&str> {
        self.operation.scope.owner.identity.as_deref()
    }

    pub fn request_generation(&self) -> u64 {
        self.operation.generation
    }

    /// Borrow dependencies captured for this accepted parent operation only.
    pub fn request_context<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.operation
            .scope
            .request_context
            .as_ref()?
            .downcast_ref()
    }

    pub fn check_authority(&self) -> Result<(), DependencyError> {
        if *self.operation.scope.cancellation.sender.borrow()
            || *self.operation.finished.sender.borrow()
        {
            return Err(DependencyError::new(DependencyErrorCode::Cancelled));
        }
        if Instant::now() >= self.operation.scope.deadline {
            return Err(DependencyError::new(DependencyErrorCode::DeadlineExceeded));
        }
        self.authority.check(&self.operation.scope.owner)
    }

    /// Wakes on explicit parent cancellation, expiry, owner revocation or parent completion.
    pub async fn cancelled(&self) {
        let mut explicit = self.operation.scope.cancellation.sender.subscribe();
        let mut finished = self.operation.finished.sender.subscribe();
        tokio::select! {
            _ = explicit.wait_for(|cancelled| *cancelled) => {},
            _ = finished.wait_for(|cancelled| *cancelled) => {},
            _ = tokio::time::sleep_until(self.operation.scope.deadline) => {},
            _ = self.authority.revoked(self.operation.scope.owner.clone()) => {},
        }
    }
}

impl fmt::Debug for DependencyContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DependencyContext")
            .field("parent_id", &self.operation.parent_id)
            .field("request_generation", &self.operation.generation)
            .field("owner_generation", &self.owner_generation())
            .finish_non_exhaustive()
    }
}
