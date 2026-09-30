use codex_protocol::security_risk::SecurityRiskScore;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(remote = "SecurityRiskScore", deny_unknown_fields)]
pub(crate) struct SecurityRiskScoreWire {
    #[serde(with = "scores")]
    scores: BTreeMap<String, f64>,
    call_id: Option<String>,
    #[serde(with = "super::events_json::optional_value")]
    action: Option<serde_json::Value>,
    sampled_at: Option<chrono::DateTime<chrono::Utc>>,
}

mod scores {
    use super::*;

    pub(super) fn serialize<S: serde::Serializer>(
        value: &BTreeMap<String, f64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_map(value.iter().map(|(key, value)| (key, value.to_bits())))
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<String, f64>, D::Error> {
        BTreeMap::<String, u64>::deserialize(deserializer).map(|values| {
            values
                .into_iter()
                .map(|(key, value)| (key, f64::from_bits(value)))
                .collect()
        })
    }
}
