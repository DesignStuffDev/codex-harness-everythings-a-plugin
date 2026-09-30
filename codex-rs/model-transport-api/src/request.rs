use crate::WireError;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// Normalized native Responses JSON, without inventing a second prompt schema.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRequest {
    pub thread_id: String,
    pub request: Value,
}

impl std::fmt::Debug for ModelRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ModelRequest").finish_non_exhaustive()
    }
}

impl ModelRequest {
    pub fn from_value(value: Value) -> Result<Self, WireError> {
        let request: Self = serde_json::from_value(value).map_err(|_| WireError::Request)?;
        if !request.request.is_object() {
            return Err(WireError::Request);
        }
        Ok(request)
    }

    pub fn into_value(self) -> Result<Value, WireError> {
        if !self.request.is_object() {
            return Err(WireError::Request);
        }
        serde_json::to_value(self).map_err(|_| WireError::Request)
    }
}

/// Preserve the native request serializer, including its intentional trust policy.
#[cfg(feature = "native")]
pub fn encode_request(
    thread_id: codex_protocol::ThreadId,
    request: codex_api::ResponsesApiRequest,
) -> Result<ModelRequest, WireError> {
    Ok(ModelRequest {
        thread_id: thread_id.to_string(),
        request: serde_json::to_value(request).map_err(|_| WireError::Request)?,
    })
}
