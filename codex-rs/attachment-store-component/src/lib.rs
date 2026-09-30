//! Process adapter for separately installed attachment stores.

mod adapter;
mod native;
pub mod protocol;
mod selection;

pub use adapter::ComponentAttachmentStore;
pub use adapter::from_catalog;
pub use native::run_native_stdio;
pub use selection::AttachmentStoreSelection;

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
