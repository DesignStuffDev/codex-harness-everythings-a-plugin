//! Immutable process-start choices for a persistent component connection.

use std::path::PathBuf;

use crate::session_limits::SessionPayloadLimits;

/// The working directory applied once, when the component process is spawned.
/// This does not change the host's directory or establish a filesystem sandbox.
#[derive(Clone, Debug, Default)]
pub enum SessionWorkingDirectory {
    /// Preserve the existing package-directory startup behavior.
    #[default]
    PackageDirectory,
    /// Use an existing absolute directory. Relative roots inside the component
    /// then resolve from this immutable startup context. The executable must
    /// also be absolute so changing cwd cannot select a different entrypoint.
    ExplicitAbsolute(PathBuf),
}

/// Persistent launch settings; defaults preserve existing component behavior.
/// Package identity, entrypoint and durable-state locations remain in the
/// original binding. These options do not alter one-shot or streamed launches.
#[derive(Clone, Debug, Default)]
pub struct ComponentSessionOptions {
    pub payload_limits: SessionPayloadLimits,
    pub working_directory: SessionWorkingDirectory,
}
