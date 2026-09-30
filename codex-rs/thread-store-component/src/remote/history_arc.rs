use std::sync::Arc;

use codex_component_state_codec::rollout_item;
use codex_rollout::RolloutItem;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize)]
#[serde(transparent)]
struct Borrowed<'a>(#[serde(with = "rollout_item::vec")] &'a [RolloutItem]);

#[derive(Deserialize)]
#[serde(transparent)]
struct Owned(#[serde(with = "rollout_item::vec")] Vec<RolloutItem>);

pub(super) fn serialize<S: serde::Serializer>(
    value: &Option<Arc<Vec<RolloutItem>>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    value
        .as_ref()
        .map(|value| Borrowed(value))
        .serialize(serializer)
}

pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Arc<Vec<RolloutItem>>>, D::Error> {
    Option::<Owned>::deserialize(deserializer).map(|value| value.map(|value| Arc::new(value.0)))
}
