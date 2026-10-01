//! Lossless native paths for explicitly selected, trusted same-OS components.
//!
//! These serde adapters preserve native bytes/code units and reject foreign
//! platform paths. They do not authenticate a sender, authorize filesystem
//! access, resolve ambient directories, or read the filesystem. A transport must
//! enforce its own input-size and authority limits before decoding.
//!
//! Ordinary path serde, provider JSON, persisted rollout JSON and PathUri keep
//! their existing formats; these adapters do not replace those trust boundaries.

mod adapters;
mod paths;

pub use paths::absolute as absolute_path;
pub use paths::native as native_path;
