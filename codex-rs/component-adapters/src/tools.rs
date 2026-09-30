use std::sync::Arc;

use anyhow::Context;
use anyhow::ensure;
use codex_component_host::ComponentBinding;
use codex_component_host::ComponentCatalog;
use codex_extension_api::ExtensionData;
use codex_extension_api::ToolContributor;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use codex_tools::FunctionCallError;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolCall;
use codex_tools::ToolExecutor;
use codex_tools::ToolExecutorFuture;
use codex_tools::ToolName;
use codex_tools::ToolOutput;
use codex_tools::ToolPayload;
use codex_tools::ToolSpec;
use codex_tools::parse_tool_input_schema;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::truncate_text;
use serde::Deserialize;
use serde_json::json;

const MAX_OUTPUT_BYTES: usize = 16_384;
const MAX_DESCRIPTION_BYTES: usize = 2_048;
const MAX_SCHEMA_BYTES: usize = 8_192;

pub(crate) struct ComponentTools {
    executors: Vec<Arc<ComponentTool>>,
}

impl ComponentTools {
    pub(crate) fn from_catalog(catalog: &ComponentCatalog) -> anyhow::Result<Self> {
        let executors = catalog
            .components("tool")
            .into_iter()
            .map(|binding| {
                let name = &binding.spec.name;
                ensure!(
                    !name.is_empty()
                        && name.len() <= 64
                        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte)),
                    "component tool {name:?} must be a plain function name of at most 64 characters"
                );
                let replacement = catalog
                    .selected("tool", name)
                    .is_some_and(|selected| selected.plugin_id == binding.plugin_id);
                ensure!(
                    !matches!(name.as_str(), "exec_command" | "shell_command"),
                    "component tool {name} requires the execution backend contract; the tool adapter cannot replace its sandbox and approval behavior"
                );
                let metadata: ToolMetadata = serde_json::from_value(binding.spec.metadata.clone())
                    .with_context(|| format!("invalid metadata for component tool {name}"))?;
                ensure!(
                    metadata.description.len() <= MAX_DESCRIPTION_BYTES,
                    "component tool {name} description exceeds {MAX_DESCRIPTION_BYTES} bytes"
                );
                ensure!(
                    serde_json::to_vec(&metadata.input_schema)?.len() <= MAX_SCHEMA_BYTES,
                    "component tool {name} schema exceeds {MAX_SCHEMA_BYTES} bytes"
                );
                let spec = ToolSpec::Function(ResponsesApiTool {
                    name: name.clone(),
                    description: metadata.description,
                    strict: false,
                    defer_loading: None,
                    parameters: parse_tool_input_schema(&metadata.input_schema)?,
                    output_schema: None,
                });
                Ok(Arc::new(ComponentTool { binding, spec, replacement }))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Self { executors })
    }
}

#[derive(Deserialize)]
struct ToolMetadata {
    description: String,
    input_schema: serde_json::Value,
}

impl ToolContributor for ComponentTools {
    fn tools(
        &self,
        _session_store: &ExtensionData,
        _thread_store: &ExtensionData,
    ) -> Vec<Arc<dyn for<'call> ToolExecutor<ToolCall<'call>>>> {
        self.executors
            .iter()
            .map(|executor| executor.clone() as Arc<dyn for<'call> ToolExecutor<ToolCall<'call>>>)
            .collect()
    }
}

struct ComponentTool {
    binding: ComponentBinding,
    spec: ToolSpec,
    replacement: bool,
}

#[derive(Deserialize)]
struct InvokeResult {
    text: String,
    success: bool,
}

impl<'call> ToolExecutor<ToolCall<'call>> for ComponentTool {
    fn tool_name(&self) -> ToolName {
        ToolName::plain(&self.binding.spec.name)
    }

    fn spec(&self) -> ToolSpec {
        self.spec.clone()
    }

    fn replaces_existing_tool(&self) -> bool {
        self.replacement
    }

    fn handle<'a>(&'a self, call: ToolCall<'call>) -> ToolExecutorFuture<'a>
    where
        ToolCall<'call>: 'a,
    {
        Box::pin(async move {
            let arguments: serde_json::Value = serde_json::from_str(call.function_arguments()?)
                .map_err(|error| {
                    FunctionCallError::RespondToModel(bounded_text(
                        &format!("invalid component tool arguments: {error}"),
                        /*budget*/ 1_000,
                    ))
                })?;
            let result = self
                .binding
                .call(
                    "invoke",
                    json!({
                        "call_id": call.call_id,
                        "name": self.binding.spec.name,
                        "arguments": arguments,
                    }),
                )
                .await
                .map_err(|error| {
                    FunctionCallError::RespondToModel(bounded_text(
                        &format!("component tool failed: {error}"),
                        /*budget*/ 1_000,
                    ))
                })?;
            let result: InvokeResult = serde_json::from_value(result).map_err(|error| {
                FunctionCallError::RespondToModel(bounded_text(
                    &format!("invalid component tool response: {error}"),
                    /*budget*/ 1_000,
                ))
            })?;
            let budget = call
                .response_byte_budget(MAX_OUTPUT_BYTES)
                .min(MAX_OUTPUT_BYTES);
            let text = bounded_text(&result.text, budget);
            Ok(Box::new(ComponentToolOutput {
                text,
                success: result.success,
            }) as Box<dyn ToolOutput>)
        })
    }
}

fn bounded_text(text: &str, budget: usize) -> String {
    let mut text = truncate_text(text, TruncationPolicy::Bytes(budget));
    // Shared truncation can add its marker outside the requested budget. Enforce the
    // process boundary's hard limit after that formatting, on a UTF-8 boundary.
    let mut end = text.len().min(budget);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}

struct ComponentToolOutput {
    text: String,
    success: bool,
}

impl ToolOutput for ComponentToolOutput {
    fn log_output(&self) -> String {
        self.text.clone()
    }

    fn success_for_logging(&self) -> bool {
        self.success
    }

    fn contains_external_context(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, _payload: &ToolPayload) -> ResponseInputItem {
        ResponseInputItem::FunctionCallOutput {
            call_id: call_id.to_owned(),
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::Text(self.text.clone()),
                success: Some(self.success),
            },
        }
    }
}
