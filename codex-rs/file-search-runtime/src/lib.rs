//! Ownership and presentation for one selected file-search provider.
//!
//! Embeddings supply explicit native/process policy and immutable selection context.
//! Public handles are distinct from retained task ownership; explicit cleanup
//! acknowledges callbacks and accepted work, not simply dropped observers.

mod cli;
mod cli_reporter;
mod interactive_policy;
mod observation;
mod pending;
mod policy;
mod provider;
mod scope;
mod selection;
mod selection_guard;
mod selection_policy;
mod session;
mod standalone_policy;
mod startup;
mod state;

pub use cli::CliInterrupted;
pub use cli::CliSearchPolicy;
pub use cli::run_cli_with_context;
pub use pending::PendingFileSearchStart;
pub use pending::RuntimeSearchStartFuture;
pub use policy::RuntimePolicy;
pub use provider::FileSearchProvider;
pub use provider::FileSearchScopeFactory;
pub use scope::FileSearchScope;
pub use selection::SelectedImplementation;
pub use selection::SelectedSearchProvider;
pub use selection::select_provider;
pub use selection_policy::SelectionContext;
pub use selection_policy::SelectionPolicy;
pub use session::FileSearchSession;
pub use standalone_policy::standalone_policy;

#[cfg(test)]
mod selection_tests;

#[cfg(test)]
mod tests;

pub use interactive_policy::interactive_allocation;
pub use interactive_policy::interactive_policy;

#[cfg(test)]
#[path = "pending_fixture.rs"]
mod pending_fixture;

#[cfg(test)]
mod pending_start_tests;
