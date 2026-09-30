//! Native history engine extracted from OpenAI Codex. No dependency on codex-core.
//! Staged and unlinked pending native differential and process acceptance.
mod compact;
mod fragments;
mod guardian;
mod history;
mod normalize;
pub mod semantics;
mod token_estimation;
mod updates;
mod util;

pub use codex_context_replay::ReviewPolicy;
pub use codex_context_replay::WorldStateSnapshot;
pub use guardian::GuardianContextMode;
pub use history::ContextManager;
pub use history::is_user_turn_boundary;
pub use token_estimation::estimate_image_reference_bytes;
pub use token_estimation::estimate_item_token_count;
pub use updates::build_rendered_message;
pub use updates::merge_contextual_fragments;
