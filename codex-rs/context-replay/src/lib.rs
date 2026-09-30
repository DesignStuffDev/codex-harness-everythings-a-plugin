//! Reusable native rollout reconstruction with owned inputs and explicit history semantics.
//!
//! This crate has no dependency on codex-core or a process transport. The reducer is pure;
//! a caller can run it on a worker or inside an independently installed component. The
//! supplied ReplayHistory implementation owns retention, rollback, and legacy compaction.

mod contract;
mod component_contract;
mod reducer;
mod selection;
mod world_state;

pub use component_contract::RECONSTRUCT_METHOD;
pub use component_contract::REPLAY_COMPONENT_KIND;
pub use component_contract::REPLAY_COMPONENT_NAME;
pub use component_contract::REPLAY_CONTRACT_VERSION;
pub use contract::HistoryProjection;
pub use contract::ReplayHistory;
pub use contract::ReplayInput;
pub use contract::ReviewPolicy;
pub use contract::RolloutReconstruction;
pub use reducer::reconstruct;
pub use world_state::WorldStateSnapshot;

#[cfg(test)]
mod component_state_codec_tests;
