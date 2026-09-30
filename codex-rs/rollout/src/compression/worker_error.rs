//! Distinguishes best-effort filesystem failures from failed worker supervision.

use std::fmt;
use std::io;

/// An awaited native compression pass failed.
///
/// Ordinary filesystem errors remain best-effort maintenance diagnostics. A
/// failed blocking task must fail the owning service's shutdown acknowledgement.
#[derive(Debug)]
pub enum RolloutCompressionWorkerError {
    Operation(io::Error),
    TaskJoin(tokio::task::JoinError),
}

impl fmt::Display for RolloutCompressionWorkerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operation(error) => {
                write!(formatter, "rollout compression operation failed: {error}")
            }
            Self::TaskJoin(_) => formatter.write_str("rollout compression worker task failed"),
        }
    }
}

impl std::error::Error for RolloutCompressionWorkerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Operation(error) => Some(error),
            Self::TaskJoin(error) => Some(error),
        }
    }
}

impl From<io::Error> for RolloutCompressionWorkerError {
    fn from(error: io::Error) -> Self {
        Self::Operation(error)
    }
}
