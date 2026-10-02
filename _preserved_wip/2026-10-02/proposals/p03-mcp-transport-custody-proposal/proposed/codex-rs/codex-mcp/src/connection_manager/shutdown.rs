//! A connection snapshot retains uncertainty as well as known failure.

use codex_async_utils::RetainedTaskOutcome;
use codex_rmcp_client::McpShutdownConfirmation;
use codex_rmcp_client::McpShutdownFailure;

pub(crate) type ShutdownResult = Result<McpShutdownConfirmation, McpShutdownFailure>;

pub(super) struct ConnectionShutdownOutcome(pub(super) ShutdownResult);

impl RetainedTaskOutcome for ConnectionShutdownOutcome {
    fn succeeded(&self) -> bool {
        matches!(self.0, Ok(McpShutdownConfirmation::Confirmed))
    }
}
