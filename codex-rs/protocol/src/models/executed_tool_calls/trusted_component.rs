//! Explicit snapshots for an already-authorized engine component connection.
//!
//! This codec is never an input/provider/rollout decoder. The host must authenticate
//! the selected component and validate the request owner and connection epoch before
//! admitting these facts. Ordinary `ExecutedToolCall` deserialization keeps ignoring
//! result provenance and treating apparent truncation markers as untrusted arguments.

use super::ExecutedToolCall;
use super::ExecutedToolCallArguments;
use super::ExecutedToolCallTruncation;
use super::ToolResultMetadata;
use super::ToolResultSource;
use super::ToolResultSources;
use serde::Deserialize;
use serde::Serialize;

/// Lossless owned snapshot of host-recorded attempted-call evidence.
///
/// Deliberately has no `Debug`: raw arguments and result metadata can contain secrets.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedComponentExecutedToolCall {
    name: String,
    arguments: TrustedArguments,
    tool_result_sources: Option<Vec<TrustedSource>>,
    // The wrapper distinguishes captured JSON null from an absent capture.
    tool_result_metadata: Option<TrustedMetadata>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
enum TrustedArguments {
    Raw(serde_json::Value),
    Truncated(ExecutedToolCallTruncation),
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedSource {
    r#type: String,
    id: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedMetadata {
    value: serde_json::Value,
}

/// Invalid native source evidence; the error contains no captured data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTrustedComponentExecutedToolCall;

impl std::fmt::Display for InvalidTrustedComponentExecutedToolCall {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("trusted component call violates native result-source bounds")
    }
}

impl std::error::Error for InvalidTrustedComponentExecutedToolCall {}

impl From<&ExecutedToolCall> for TrustedComponentExecutedToolCall {
    fn from(call: &ExecutedToolCall) -> Self {
        Self {
            name: call.name.clone(),
            arguments: match &call.arguments {
                ExecutedToolCallArguments::Raw(value) => TrustedArguments::Raw(value.clone()),
                ExecutedToolCallArguments::Truncated { truncation } => {
                    TrustedArguments::Truncated(truncation.clone())
                }
            },
            tool_result_sources: call.tool_result_sources.as_ref().map(|sources| {
                sources
                    .iter()
                    .map(|source| TrustedSource {
                        r#type: source.r#type.clone(),
                        id: source.id.clone(),
                    })
                    .collect()
            }),
            tool_result_metadata: call.tool_result_metadata.0.as_ref().map(|value| {
                TrustedMetadata {
                    value: value.clone(),
                }
            }),
        }
    }
}

impl TryFrom<TrustedComponentExecutedToolCall> for ExecutedToolCall {
    type Error = InvalidTrustedComponentExecutedToolCall;

    fn try_from(snapshot: TrustedComponentExecutedToolCall) -> Result<Self, Self::Error> {
        let tool_result_sources = snapshot.tool_result_sources.map(|sources| {
            sources
                .into_iter()
                .map(|source| ToolResultSource {
                    r#type: source.r#type,
                    id: source.id,
                })
                .collect::<Vec<_>>()
        });
        if let Some(sources) = &tool_result_sources {
            // Native capture bounds and deduplicates sources. Reject violations;
            // do not silently rewrite the component's evidence during restoration.
            let bounded = ToolResultSources::new(sources.clone());
            if bounded.0.as_ref() != Some(sources) {
                return Err(InvalidTrustedComponentExecutedToolCall);
            }
        }
        Ok(Self {
            name: snapshot.name,
            arguments: match snapshot.arguments {
                TrustedArguments::Raw(value) => ExecutedToolCallArguments::Raw(value),
                TrustedArguments::Truncated(truncation) => {
                    ExecutedToolCallArguments::Truncated { truncation }
                }
            },
            tool_result_sources,
            tool_result_metadata: ToolResultMetadata(
                snapshot.tool_result_metadata.map(|metadata| metadata.value),
            ),
        })
    }
}

#[cfg(test)]
#[path = "trusted_component_tests.rs"]
mod tests;
