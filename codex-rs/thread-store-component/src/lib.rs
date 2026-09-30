//! Typed process boundary for the complete native thread persistence contract.
//!
//! The host keeps auxiliary state services separately. A local storage component
//! owns its live writers, pending metadata, durability barriers and fork leases.

mod acquisition;
mod adapter;
mod contract;
mod dispatch;
mod error;
mod remote;
mod service;
mod wire;

pub use adapter::ProcessThreadStore;
pub use contract::CALL_METHOD;
pub use contract::CopyAttachmentsParams;
pub use contract::LocalStoragePaths;
pub use contract::OPEN_METHOD;
pub use contract::PendingMetadataParams;
pub use contract::PersistParams;
pub use contract::PrepareForkRequest;
pub use contract::PreparedForkResponse;
pub use contract::RELEASE_FORK_METHOD;
pub use contract::ReleaseForkRequest;
pub use contract::StorageCapabilities;
pub use contract::StorageInitialization;
pub use contract::THREAD_STORE_CONTRACT_VERSION;
pub use error::StorageError;
pub use service::StorageService;
pub use wire::StorageReply;
pub use wire::StorageRequest;
pub use wire::StorageResponse;
