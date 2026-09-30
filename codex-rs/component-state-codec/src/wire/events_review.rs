//! Trusted process transport DTOs. These deliberately bypass public rollout omissions.
//! External enum tags retain arbitrary-precision JSON values without Serde content buffering.

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::RateLimitWindow")]
pub(crate) struct RateLimitWindowWire {
    #[serde(with = "super::events_scalars::f64_bits")]
    used_percent: f64,
    window_minutes: Option<i64>,
    resets_at: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::RateLimitSnapshot")]
pub(crate) struct RateLimitSnapshotWire {
    limit_id: Option<String>,
    limit_name: Option<String>,
    normal_model_slug: Option<String>,
    #[serde(with = "optional_window")]
    primary: Option<codex_protocol::protocol::RateLimitWindow>,
    #[serde(with = "optional_window")]
    secondary: Option<codex_protocol::protocol::RateLimitWindow>,
    credits: Option<codex_protocol::protocol::CreditsSnapshot>,
    individual_limit: Option<codex_protocol::protocol::SpendControlLimitSnapshot>,
    spend_control_reached: Option<bool>,
    plan_type: Option<codex_protocol::account::PlanType>,
    rate_limit_reached_type: Option<codex_protocol::protocol::RateLimitReachedType>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ReviewFinding")]
pub(crate) struct ReviewFindingWire {
    title: String,
    body: String,
    #[serde(with = "super::events_scalars::f32_bits")]
    confidence_score: f32,
    priority: i32,
    #[serde(with = "super::events_file_changes::ReviewCodeLocationWire")]
    code_location: codex_protocol::protocol::ReviewCodeLocation,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ReviewOutputEvent")]
pub(crate) struct ReviewOutputEventWire {
    #[serde(with = "findings")]
    findings: Vec<codex_protocol::protocol::ReviewFinding>,
    overall_correctness: String,
    overall_explanation: String,
    #[serde(with = "super::events_scalars::f32_bits")]
    overall_confidence_score: f32,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ExitedReviewModeEvent")]
pub(crate) struct ExitedReviewModeEventWire {
    turn_id: Option<String>,
    item_id: Option<String>,
    #[serde(with = "optional_review")]
    review_output: Option<codex_protocol::protocol::ReviewOutputEvent>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::items::ExitedReviewModeItem")]
pub(crate) struct ExitedReviewModeItemWire {
    id: String,
    #[serde(with = "optional_review")]
    review_output: Option<codex_protocol::protocol::ReviewOutputEvent>,
}

pub(crate) mod optional_window {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::RateLimitWindowWire")] &'a codex_protocol::protocol::RateLimitWindow,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::RateLimitWindowWire")] codex_protocol::protocol::RateLimitWindow,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::protocol::RateLimitWindow>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::protocol::RateLimitWindow>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod optional_rate_limits {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::RateLimitSnapshotWire")]
        &'a codex_protocol::protocol::RateLimitSnapshot,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::RateLimitSnapshotWire")] codex_protocol::protocol::RateLimitSnapshot,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::protocol::RateLimitSnapshot>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::protocol::RateLimitSnapshot>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod findings {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::ReviewFindingWire")] &'a codex_protocol::protocol::ReviewFinding,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::ReviewFindingWire")] codex_protocol::protocol::ReviewFinding,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &[codex_protocol::protocol::ReviewFinding],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(value.iter().map(Borrowed))
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<codex_protocol::protocol::ReviewFinding>, D::Error> {
        let value = <Vec<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.into_iter().map(|owned| owned.0).collect())
    }
}

pub(crate) mod optional_review {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::ReviewOutputEventWire")]
        &'a codex_protocol::protocol::ReviewOutputEvent,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::ReviewOutputEventWire")] codex_protocol::protocol::ReviewOutputEvent,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::protocol::ReviewOutputEvent>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::protocol::ReviewOutputEvent>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}
