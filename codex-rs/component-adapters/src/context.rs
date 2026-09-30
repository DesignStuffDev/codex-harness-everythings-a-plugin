use std::sync::Arc;

use codex_component_host::ComponentBinding;
use codex_context_fragments::ComponentContextFragment;
use codex_context_fragments::ContextualUserFragment;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionFuture;
use codex_extension_api::ExtensionMetrics;
use codex_extension_api::TurnInputContext;
use codex_extension_api::TurnInputContributor;
use serde::Deserialize;
use serde_json::json;

pub(crate) struct ComponentContext(pub(crate) ComponentBinding);

#[derive(Deserialize)]
struct ContextResult {
    text: String,
}

impl TurnInputContributor for ComponentContext {
    fn contribute<'a>(
        &'a self,
        input: TurnInputContext<'a>,
        _extension_metrics: Option<Arc<dyn ExtensionMetrics>>,
        session_store: &'a ExtensionData,
        thread_store: &'a ExtensionData,
        _turn_store: &'a ExtensionData,
    ) -> ExtensionFuture<'a, Vec<Box<dyn ContextualUserFragment + Send>>> {
        Box::pin(async move {
            let result = self
                .0
                .call(
                    "contribute",
                    json!({
                        "session_id": session_store.level_id(),
                        "thread_id": thread_store.level_id(),
                        "turn_id": input.turn_id,
                    }),
                )
                .await
                .and_then(|value| Ok(serde_json::from_value::<ContextResult>(value)?));
            match result {
                Ok(result) if !result.text.trim().is_empty() => {
                    vec![Box::new(ComponentContextFragment::new(
                        &self.0.plugin_id,
                        &self.0.spec.name,
                        &result.text,
                    ))
                        as Box<dyn ContextualUserFragment + Send>]
                }
                Ok(_) => Vec::new(),
                Err(error) => {
                    tracing::warn!(plugin_id = %self.0.plugin_id, %error, "component context unavailable");
                    Vec::new()
                }
            }
        })
    }
}
