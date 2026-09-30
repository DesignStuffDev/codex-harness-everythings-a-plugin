//! Lossless transport for already-authorized, selected engine components.
//!
//! These DTOs deliberately differ from provider JSON and persisted rollout JSON.
//! Host-only annotations are admitted only after the host checks the selected
//! component, request owner and connection epoch. This module must never be used
//! to deserialize model output, user JSON, or an unauthenticated rollout.
//!
//! No codec grants authority: preserving evidence is distinct from validating its
//! owner. The component adapter must reject stale or mismatched responses before
//! publishing reconstructed state.

macro_rules! remote_adapter {
    ($module:ident, $native:ty, $remote:path, $wire:literal) => {
        // Internal adapters use only the container shapes their native fields need.
        #[allow(
            dead_code,
            reason = "shared serde adapter family includes unused internal container shapes"
        )]
        pub mod $module {
            use super::*;

            type Native = $native;

            #[derive(serde::Serialize)]
            #[serde(transparent)]
            pub struct Borrowed<'a>(#[serde(with = $wire)] pub &'a $native);

            #[derive(serde::Deserialize)]
            #[serde(transparent)]
            pub struct Owned(#[serde(with = $wire)] pub $native);

            #[allow(
                clippy::ptr_arg,
                reason = "typed native serde adapter uses the native field type"
            )]
            pub fn serialize<S: serde::Serializer>(
                value: &$native,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                <$remote>::serialize(value, serializer)
            }

            pub fn deserialize<'de, D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<$native, D::Error> {
                <$remote>::deserialize(deserializer)
            }

            pub mod option {
                use super::*;
                pub fn serialize<S: serde::Serializer>(
                    value: &Option<$native>,
                    serializer: S,
                ) -> Result<S::Ok, S::Error> {
                    serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
                }
                pub fn deserialize<'de, D: serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> Result<Option<$native>, D::Error> {
                    <Option<Owned> as serde::Deserialize>::deserialize(deserializer)
                        .map(|value| value.map(|value| value.0))
                }
            }

            pub mod vec {
                use super::*;
                pub fn serialize<S: serde::Serializer>(
                    value: &[$native],
                    serializer: S,
                ) -> Result<S::Ok, S::Error> {
                    serializer.collect_seq(value.iter().map(Borrowed))
                }
                pub fn deserialize<'de, D: serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> Result<Vec<$native>, D::Error> {
                    <Vec<Owned> as serde::Deserialize>::deserialize(deserializer)
                        .map(|value| value.into_iter().map(|value| value.0).collect())
                }
            }

            pub mod option_vec {
                use super::Native;

                #[derive(serde::Serialize)]
                #[serde(transparent)]
                struct BorrowedVec<'a>(#[serde(with = "super::vec")] &'a Vec<Native>);

                #[derive(serde::Deserialize)]
                #[serde(transparent)]
                struct OwnedVec(#[serde(with = "super::vec")] Vec<Native>);

                pub fn serialize<S: serde::Serializer>(
                    value: &Option<Vec<Native>>,
                    serializer: S,
                ) -> Result<S::Ok, S::Error> {
                    serde::Serialize::serialize(&value.as_ref().map(BorrowedVec), serializer)
                }
                pub fn deserialize<'de, D: serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> Result<Option<Vec<Native>>, D::Error> {
                    <Option<OwnedVec> as serde::Deserialize>::deserialize(deserializer)
                        .map(|value| value.map(|value| value.0))
                }
            }
        }
    };
}

mod envelope;
mod events;
mod events_approvals;
mod events_file_changes;
mod events_items;
mod events_json;
mod events_nested;
mod events_paths;
mod events_permissions;
mod events_review;
mod events_scalars;
mod metadata;
mod paths;
mod response;
mod rollout;
mod security;
mod session;
mod usage;

pub use envelope::envelopes;
pub use envelope::guardian_checkpoint;
pub use events_items::image_generation_item;
pub use events_json::optional_value as optional_json;
pub use events_permissions::permission_profile;
pub use events_scalars::model_context;
pub use events_scalars::reasoning_effort;
pub use events_scalars::thread_source;
pub use paths::absolute as absolute_path;
pub use paths::native as native_path;
pub use response::response_item;
pub use rollout::rollout_item;
pub use session::dynamic_tool;
pub use session::turn_context;
pub use usage::token_usage;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "events_tests.rs"]
mod events_tests;

#[cfg(test)]
#[path = "events_paths_tests.rs"]
mod events_paths_tests;
