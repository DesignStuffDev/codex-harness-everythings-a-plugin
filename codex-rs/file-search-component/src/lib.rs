//! Versioned process adapters for a selected, trusted file-search backend.
//!
//! This crate owns wire validation and accepted service work, not filesystem
//! policy, a native matcher, presentation state or Core configuration.

mod contract;
mod limits;
mod outcome;
mod process;
mod service;
mod service_admit;
mod service_lease;
mod service_open;
mod service_operations;
mod service_provider;
mod service_runner;
mod value_wire;

pub use contract::*;
pub use limits::*;
pub use outcome::*;
pub use process::ProcessSearchBackend;
pub use service::*;
pub use service_admit::ServiceLane;
pub use service_runner::run_stdio;
pub use value_wire::*;

#[cfg(test)]
mod service_tests;
