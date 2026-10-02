//! Executable-only shutdown authority scoped to one TUI invocation.
//!
//! Embedded library callers and recoverable startup retries must not close the
//! process-wide curated worker. Final cleanup begins before local server drains.

use crate::AppExitInfo;
use crate::ExitReason;
use codex_core_plugins::PluginStartupProcessShutdown;
use codex_utils_process::process_shutdown::ProcessFinalCapability;
use std::future::Future;

const UNCLEAN_OWNERSHIP: &str =
    "plugin startup/native HTTP construction cleanup incomplete; ownership is unconfirmed";

tokio::task_local! {
    static CAPABILITY: Option<ProcessFinalCapability>;
}

pub(crate) async fn scope<F: Future>(
    capability: Option<ProcessFinalCapability>,
    future: F,
) -> F::Output {
    CAPABILITY.scope(capability, future).await
}

/// Fence process admission at final-exit onset. Primary custody survives this
/// short-lived observer; executable cleanup observes it on the same deadline.
pub(crate) fn begin() {
    let _ = CAPABILITY.try_with(|capability| {
        if let Some(capability) = capability {
            PluginStartupProcessShutdown::begin(capability.begin().graceful());
        }
    });
}

/// The CLI may repair a state-db startup failure and rerun the TUI in-process.
pub(crate) fn begin_after_result<T>(result: &std::io::Result<T>) {
    if is_final_result(result) {
        begin();
    }
}

/// Observe final ownership while the TUI's tracing and telemetry guards are alive.
pub(crate) async fn finish(result: std::io::Result<AppExitInfo>) -> std::io::Result<AppExitInfo> {
    if !is_final_result(&result) {
        return result;
    }
    let deadline = CAPABILITY
        .try_with(|capability| {
            capability
                .as_ref()
                .map(|capability| capability.begin().graceful())
        })
        .ok()
        .flatten();
    let Some(deadline) = deadline else {
        return result;
    };
    let shutdown = PluginStartupProcessShutdown::begin(deadline);
    let observed = shutdown.wait_until(deadline).await;
    if observed.is_complete() && observed.curated.clean_native_ownership() {
        return result;
    }
    Err(attach_cleanup_error(result, UNCLEAN_OWNERSHIP.to_string()))
}

fn attach_cleanup_error(result: std::io::Result<AppExitInfo>, cleanup: String) -> std::io::Error {
    match result {
        Ok(info) => match info.exit_reason {
            ExitReason::Fatal(message) => {
                std::io::Error::other(anyhow::anyhow!(message).context(cleanup))
            }
            ExitReason::UserRequested
            | ExitReason::Archived(_)
            | ExitReason::TurnInterrupted
            | ExitReason::ThreadRemoved => std::io::Error::other(cleanup),
        },
        Err(error) => std::io::Error::new(error.kind(), anyhow::Error::new(error).context(cleanup)),
    }
}

fn is_final_result<T>(result: &std::io::Result<T>) -> bool {
    !result.as_ref().is_err_and(|err| {
        err.get_ref()
            .is_some_and(|err| err.is::<crate::LocalStateDbStartupError>())
    })
}

#[cfg(test)]
#[path = "process_final_tests.rs"]
mod tests;
