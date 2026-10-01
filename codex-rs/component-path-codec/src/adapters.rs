//! Private serde container helpers shared by the two native path adapters.

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

pub(crate) use remote_adapter;
