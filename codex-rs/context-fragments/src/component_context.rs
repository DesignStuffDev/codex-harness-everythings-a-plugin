use codex_protocol::models::ContentItemKind;

use crate::ContextualUserFragment;

// Includes attribution and markers. This is below the 1K-token manual-review threshold even
// for content whose tokenization approaches one token per byte.
const MAX_FRAGMENT_BYTES: usize = 1_000;
const START: &str = "<component_context>\n";
const END: &str = "\n</component_context>";

/// Bounded, attributed contextual data contributed by an installed engine component.
///
/// The role and classification are host-owned. Component text never becomes user authorization
/// or a developer-policy fragment merely because it came from an installed executable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentContextFragment {
    body: String,
}

impl ComponentContextFragment {
    pub fn new(plugin_id: &str, component_name: &str, text: &str) -> Self {
        let mut body = format!("Source: {plugin_id}/{component_name}\n{text}");
        let mut end = body.len().min(MAX_FRAGMENT_BYTES - START.len() - END.len());
        while !body.is_char_boundary(end) {
            end -= 1;
        }
        body.truncate(end);
        Self { body }
    }
}

impl ContextualUserFragment for ComponentContextFragment {
    fn role(&self) -> &'static str {
        "user"
    }

    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("components.context".to_owned())
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        (START, END)
    }

    fn body(&self) -> String {
        self.body.clone()
    }
}
