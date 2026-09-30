use std::sync::Arc;

use codex_attachment_store_api::AttachmentStore;
use codex_attachment_store_inline::InlineAttachmentStore;
use codex_component_host::ComponentCatalog;

/// Carries attachment-store ownership through session creation and delegation.
/// Default selections resolve against the caller's isolation-filtered catalog;
/// explicitly injected stores remain exactly the caller's implementation.
#[derive(Clone)]
pub enum AttachmentStoreSelection {
    Default,
    Explicit(Arc<dyn AttachmentStore>),
}

impl From<Arc<dyn AttachmentStore>> for AttachmentStoreSelection {
    fn from(store: Arc<dyn AttachmentStore>) -> Self {
        Self::Explicit(store)
    }
}

impl AttachmentStoreSelection {
    /// Resolve once at session startup; active stores retain their binding.
    pub fn resolve(&self, catalog: &ComponentCatalog) -> anyhow::Result<Arc<dyn AttachmentStore>> {
        match self {
            Self::Default => Ok(crate::from_catalog(catalog)?.unwrap_or_else(|| self.baseline())),
            Self::Explicit(store) => Ok(Arc::clone(store)),
        }
    }

    /// The unconfigured native default, or the exact explicitly injected store.
    /// A manager's legacy getter cannot represent different per-session catalogs.
    pub fn baseline(&self) -> Arc<dyn AttachmentStore> {
        match self {
            Self::Default => Arc::new(InlineAttachmentStore),
            Self::Explicit(store) => Arc::clone(store),
        }
    }
}
