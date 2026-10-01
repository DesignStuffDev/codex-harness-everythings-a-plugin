//! Implementation-neutral file-search values and asynchronous backend contracts.
//!
//! This crate owns no matcher, process, executor, catalog or session registry.
//! Implementations must retain accepted work independently of observing futures;
//! only an explicit successful close or shutdown acknowledges joined cleanup.
//! Component wire DTOs and same-OS path encoding belong in the process adapter.

mod backend;
mod budget;
mod error;
mod values;

pub use backend::QueryAccepted;
pub use backend::SearchBackend;
pub use backend::SearchBackendSession;
pub use backend::SearchFrame;
pub use backend::SearchFuture;
pub use backend::SearchOpen;
pub use backend::SearchPhase;
pub use backend::SearchPoll;
pub use backend::SearchQuery;
pub use backend::SearchStartFuture;
pub use budget::ProviderLimits;
pub use budget::ScopeLimits;
pub use budget::SearchBudget;
pub use error::SearchError;
pub use error::SearchErrorKind;
pub use error::SearchStartError;
pub use error::StartCleanup;
pub use values::FileMatch;
pub use values::FileSearchOptions;
pub use values::FileSearchResults;
pub use values::FileSearchSnapshot;
pub use values::MatchType;
pub use values::SessionReporter;

#[cfg(test)]
#[path = "contract_tests.rs"]
mod tests;
