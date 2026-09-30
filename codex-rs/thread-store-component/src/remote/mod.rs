//! Version 2 transport mirrors. These preserve typed state before the selected
//! backend applies its own persistence policy, rather than invoking rollout or
//! provider compatibility serde during a process hop.

// Families are generated together so nested DTOs use one authoritative mirror.
macro_rules! remote_adapter {
    ($module:ident, $native:ty, $remote:path, $wire:literal) => {
        #[allow(dead_code)]
        pub(crate) mod $module {
            use super::*;

            #[derive(Serialize)]
            #[serde(transparent)]
            pub(crate) struct Borrowed<'a>(#[serde(with = $wire)] pub(crate) &'a $native);

            #[derive(Deserialize)]
            #[serde(transparent)]
            pub(crate) struct Owned(#[serde(with = $wire)] pub(crate) $native);

            pub(crate) fn serialize<S: serde::Serializer>(
                value: &$native,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                <$remote>::serialize(value, serializer)
            }
            pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<$native, D::Error> {
                <$remote>::deserialize(deserializer)
            }
            pub(crate) mod option {
                use super::*;
                pub(crate) fn serialize<S: serde::Serializer>(
                    value: &Option<$native>,
                    serializer: S,
                ) -> Result<S::Ok, S::Error> {
                    serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
                }
                pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> Result<Option<$native>, D::Error> {
                    <Option<Owned> as serde::Deserialize>::deserialize(deserializer)
                        .map(|value| value.map(|value| value.0))
                }
            }
            pub(crate) mod vec {
                use super::*;
                pub(crate) fn serialize<S: serde::Serializer>(
                    value: &[$native],
                    serializer: S,
                ) -> Result<S::Ok, S::Error> {
                    serializer.collect_seq(value.iter().map(Borrowed))
                }
                pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> Result<Vec<$native>, D::Error> {
                    <Vec<Owned> as serde::Deserialize>::deserialize(deserializer)
                        .map(|value| value.into_iter().map(|value| value.0).collect())
                }
            }
        }
    };
}

mod clearable;
mod creation;
mod discovery;
mod history_arc;
mod metadata;
pub(crate) mod timeline;

pub(crate) use creation::append_thread_items_params;
pub(crate) use creation::create_thread_params;
pub(crate) use creation::resume_thread_params;
pub(crate) use creation::stored_model_context;
pub(crate) use creation::stored_thread_history;
pub(crate) use creation::thread_persistence_metadata;
pub(crate) use discovery::list_threads_params;
pub(crate) use discovery::read_thread_by_rollout_path_params;
pub(crate) use discovery::stored_thread;
pub(crate) use discovery::stored_thread_search_result;
pub(crate) use discovery::thread_page;
pub(crate) use discovery::thread_search_page;
pub(crate) use metadata::thread_metadata_patch;
pub(crate) use metadata::update_thread_metadata_params;
