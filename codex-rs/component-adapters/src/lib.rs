//! Adapters from independently installed components to Codex's native extension contracts.
//!
//! Only bounded, owned request values cross the process boundary. Host capabilities, session
//! objects, credentials, and mutable history are never included implicitly.

mod context;
mod lifecycle;
mod tools;

use std::sync::Arc;

use codex_component_host::ComponentCatalog;
use codex_extension_api::ExtensionRegistryBuilder;

/// Registers the installed tool, contextual-input, and lifecycle components for one host runtime.
///
/// Selection is frozen by the catalog at startup; active adapters retain their bindings until
/// their host drops them. Executable files must remain available for those bindings.
pub fn install<C: Sync + 'static>(
    builder: &mut ExtensionRegistryBuilder<C>,
    catalog: &ComponentCatalog,
) -> anyhow::Result<()> {
    // Validate every tool before mutating the builder, so a malformed declaration cannot install
    // a partially registered component set.
    let tools = tools::ComponentTools::from_catalog(catalog)?;
    builder.tool_contributor(Arc::new(tools));
    for binding in catalog.components("context") {
        builder.turn_input_contributor(Arc::new(context::ComponentContext(binding)));
    }
    for mut binding in catalog.components("lifecycle") {
        // Stop/abort callbacks run before host completion notifications. Observers must never
        // consume the longer interactive-tool deadline while those notifications are pending.
        binding.timeout_ms = binding.timeout_ms.min(1_000);
        let contributor = Arc::new(lifecycle::ComponentLifecycle(binding));
        builder.thread_lifecycle_contributor(contributor.clone());
        builder.turn_lifecycle_contributor(contributor);
    }
    Ok(())
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
