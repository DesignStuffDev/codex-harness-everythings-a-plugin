//! Compatibility facade for the attachment API and native inline implementation.

pub use codex_attachment_store_api::*;
pub use codex_attachment_store_inline::InlineAttachmentStore;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
