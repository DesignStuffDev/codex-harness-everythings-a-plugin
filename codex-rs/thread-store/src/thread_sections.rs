use serde::Deserialize;
use serde::Serialize;

/// Parameters for listing independently persisted thread sections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListThreadSectionsParams {
    /// Opaque cursor returned by a previous section listing.
    pub cursor: Option<String>,
    /// Maximum number of sections to return.
    pub limit: usize,
}

/// Parameters for creating a thread section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateThreadSectionParams {
    /// User-facing section name.
    pub name: String,
    pub appearance: Option<codex_state::ThreadSectionAppearance>,
}

/// Parameters for renaming a thread section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameThreadSectionParams {
    /// Stable identifier of the section to rename.
    pub section_id: String,
    /// Replacement user-facing section name.
    pub name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::types::optional_option"
    )]
    pub appearance: Option<Option<codex_state::ThreadSectionAppearance>>,
}

/// Parameters for deleting a thread section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteThreadSectionParams {
    /// Stable identifier of the section to delete.
    pub section_id: String,
}

/// An independently persisted thread section and its user-facing name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredThreadSection {
    /// Stable, server-owned section identifier.
    pub id: String,
    /// User-facing section name.
    pub name: String,
    pub appearance: Option<codex_state::ThreadSectionAppearance>,
}

/// A cursor-paginated page of independently persisted thread sections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredThreadSectionsPage {
    /// Sections returned for this page.
    pub sections: Vec<StoredThreadSection>,
    /// Opaque cursor to continue listing sections.
    pub next_cursor: Option<String>,
}
