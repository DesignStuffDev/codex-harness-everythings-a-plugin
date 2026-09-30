//! Lossless snapshots for explicitly selected, trusted component connections.
//!
//! These adapters preserve native in-memory facts across process boundaries. They
//! never authenticate provenance or grant authority. The host must bind admission
//! and replies to the selected component, owner, and connection epoch. Do not use
//! them for provider responses, user input, or persisted rollout deserialization.
//! Ordinary native serde remains the compatibility and trust boundary for those.

mod wire;

pub use wire::absolute_path;
pub use wire::dynamic_tool;
pub use wire::envelopes;
pub use wire::guardian_checkpoint;
pub use wire::image_generation_item;
pub use wire::model_context;
pub use wire::native_path;
pub use wire::optional_json;
pub use wire::permission_profile;
pub use wire::reasoning_effort;
pub use wire::response_item;
pub use wire::rollout_item;
pub use wire::thread_source;
pub use wire::token_usage;
pub use wire::turn_context;
