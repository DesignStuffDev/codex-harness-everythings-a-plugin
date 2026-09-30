//! Trusted process transport DTOs. These deliberately bypass public rollout omissions.
//! External enum tags retain arbitrary-precision JSON values without Serde content buffering.

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ThreadSource")]
pub(crate) enum ThreadSourceWire {
    User,
    Subagent,
    GuardianReview,
    Feature(String),
    MemoryConsolidation,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::openai_models::ReasoningEffort")]
pub(crate) enum ReasoningEffortWire {
    None,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
    Ultra,
    Persistent,
    Custom(String),
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::items::ModelInvocationContext")]
pub(crate) struct ModelInvocationContextWire {
    model_slug: String,
    reasoning_effort: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::config_types::Settings")]
pub(crate) struct SettingsWire {
    model: String,
    #[serde(with = "super::events_scalars::optional_effort")]
    reasoning_effort: Option<codex_protocol::openai_models::ReasoningEffort>,
    developer_instructions: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::config_types::CollaborationMode")]
pub(crate) struct CollaborationModeWire {
    mode: codex_protocol::config_types::ModeKind,
    #[serde(with = "SettingsWire")]
    settings: codex_protocol::config_types::Settings,
}

pub(crate) mod optional_effort {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::ReasoningEffortWire")]
        &'a codex_protocol::openai_models::ReasoningEffort,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::ReasoningEffortWire")]
        codex_protocol::openai_models::ReasoningEffort,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::openai_models::ReasoningEffort>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::openai_models::ReasoningEffort>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod optional_thread_source {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::ThreadSourceWire")] &'a codex_protocol::protocol::ThreadSource,
    );
    #[derive(serde::Deserialize)]
    struct Owned(#[serde(with = "super::ThreadSourceWire")] codex_protocol::protocol::ThreadSource);

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::protocol::ThreadSource>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::protocol::ThreadSource>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod optional_model_context {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::ModelInvocationContextWire")]
        &'a codex_protocol::items::ModelInvocationContext,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::ModelInvocationContextWire")]
        codex_protocol::items::ModelInvocationContext,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::items::ModelInvocationContext>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::items::ModelInvocationContext>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod optional_collaboration {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::CollaborationModeWire")]
        &'a codex_protocol::config_types::CollaborationMode,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::CollaborationModeWire")]
        codex_protocol::config_types::CollaborationMode,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::config_types::CollaborationMode>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::config_types::CollaborationMode>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod f32_bits {
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde with adapters require borrowed values"
    )]
    pub(crate) fn serialize<S: serde::Serializer>(
        value: &f32,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.to_bits(), serializer)
    }
    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<f32, D::Error> {
        Ok(f32::from_bits(<u32 as serde::Deserialize>::deserialize(
            deserializer,
        )?))
    }
}

pub(crate) mod f64_bits {
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde with adapters require borrowed values"
    )]
    pub(crate) fn serialize<S: serde::Serializer>(
        value: &f64,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.to_bits(), serializer)
    }
    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<f64, D::Error> {
        Ok(f64::from_bits(<u64 as serde::Deserialize>::deserialize(
            deserializer,
        )?))
    }
}

remote_adapter!(
    reasoning_effort,
    codex_protocol::openai_models::ReasoningEffort,
    ReasoningEffortWire,
    "ReasoningEffortWire"
);
remote_adapter!(
    thread_source,
    codex_protocol::protocol::ThreadSource,
    ThreadSourceWire,
    "ThreadSourceWire"
);

remote_adapter!(
    model_context,
    codex_protocol::items::ModelInvocationContext,
    ModelInvocationContextWire,
    "ModelInvocationContextWire"
);
