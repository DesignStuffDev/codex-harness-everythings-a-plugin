//! Compatibility aliases for the shared same-OS component path codec.
//!
//! Preserve the state codec's internal and public adapter paths. PathUri and
//! ordinary provider/rollout serde remain separate formats and trust boundaries.

pub use codex_component_path_codec::absolute_path as absolute;
pub use codex_component_path_codec::native_path as native;

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
