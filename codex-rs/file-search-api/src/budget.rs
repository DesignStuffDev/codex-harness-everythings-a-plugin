use crate::SearchError;
use crate::SearchErrorKind;
use serde::Deserialize;
use serde::Serialize;
use std::num::NonZeroUsize;

/// Explicit per-session resource allocation, independent of the result limit.
///
/// There is deliberately no default. Backends must reserve before admission and
/// charge before allocating/inserting. Exhaustion fails explicitly; a truncated
/// index must never be reported as a complete walk. These are accounting bounds,
/// not a promise about total process RSS or interruptible filesystem operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchBudget {
    /// Maximum retained indexed entries, including directories.
    pub max_index_entries: NonZeroUsize,
    /// Maximum charged index bytes, including owned paths and matcher storage.
    /// A backend must document its accounting and reject unsupported budgets.
    pub max_index_bytes: NonZeroUsize,
    /// All session-owned OS workers, including traversal, matching and supervision.
    /// This is not just `FileSearchOptions::threads`; startup must account for
    /// implementation-specific additional workers before accepting the session.
    pub max_worker_threads: NonZeroUsize,
}

impl SearchBudget {
    /// Checks an allocation against explicit ceilings without clamping it.
    /// Callers still own atomic aggregate reservation and joined release.
    pub fn validate_within(&self, ceiling: &Self) -> Result<(), SearchError> {
        if self.max_index_entries > ceiling.max_index_entries
            || self.max_index_bytes > ceiling.max_index_bytes
            || self.max_worker_threads > ceiling.max_worker_threads
        {
            return Err(SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "file-search resource allocation exceeds the provider ceiling",
            ));
        }
        Ok(())
    }
}

/// Composition-wide ceilings for one selected provider and all of its scopes.
///
/// Session and resource reservations include Preparing, Ready and Closing work
/// and survive abandoned observers. An implementation must atomically reserve
/// aggregate allocations, reject overflow, and release only after confirmed
/// cleanup. This value describes policy; it does not itself own a semaphore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderLimits {
    /// Bounds scope metadata even if scopes have not admitted sessions.
    pub max_scopes: NonZeroUsize,
    pub max_sessions: NonZeroUsize,
    /// Aggregate allocation across all retained sessions, not a per-session cap.
    pub resources: SearchBudget,
}

/// A connection/client's retained-session allowance within a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeLimits {
    pub max_sessions: NonZeroUsize,
}

impl ScopeLimits {
    /// Checks a requested scope allowance; this does not reserve provider slots.
    pub fn validate_within(&self, provider: &ProviderLimits) -> Result<(), SearchError> {
        if self.max_sessions > provider.max_sessions {
            return Err(SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "file-search scope allowance exceeds the provider session ceiling",
            ));
        }
        Ok(())
    }
}
