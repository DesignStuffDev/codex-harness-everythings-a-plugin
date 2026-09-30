//! Explicit field presence, independent of JSON omission/default behavior.

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
enum Field<T> {
    Unchanged,
    Clear,
    Set(T),
}

pub(super) fn serialize<T: Serialize, S: serde::Serializer>(
    value: &Option<Option<T>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        None => Field::<&T>::Unchanged,
        Some(None) => Field::Clear,
        Some(Some(value)) => Field::Set(value),
    }
    .serialize(serializer)
}

pub(super) fn deserialize<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error> {
    Field::<T>::deserialize(deserializer).map(|field| match field {
        Field::Unchanged => None,
        Field::Clear => Some(None),
        Field::Set(value) => Some(Some(value)),
    })
}

macro_rules! scalar_field {
    ($module:ident, $native:ty) => {
        pub(super) mod $module {
            use codex_component_state_codec::$module::Borrowed;
            use codex_component_state_codec::$module::Owned;

            pub(crate) fn serialize<S: serde::Serializer>(
                value: &Option<Option<$native>>,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                super::serialize(
                    &value.as_ref().map(|value| value.as_ref().map(Borrowed)),
                    serializer,
                )
            }
            pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Option<Option<$native>>, D::Error> {
                super::deserialize::<Owned, D>(deserializer)
                    .map(|value| value.map(|value| value.map(|value| value.0)))
            }
        }
    };
}

scalar_field!(
    reasoning_effort,
    codex_protocol::openai_models::ReasoningEffort
);
scalar_field!(thread_source, codex_protocol::protocol::ThreadSource);
