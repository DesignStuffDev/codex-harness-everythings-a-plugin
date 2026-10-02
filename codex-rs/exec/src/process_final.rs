//! Executable-owned final cleanup, before the arg0 runtime is dropped.

use codex_core_plugins::startup_sync::CuratedProcessShutdown;
use codex_utils_process::process_shutdown::ProcessFinalCapability;

pub(crate) fn begin(capability: &ProcessFinalCapability) -> CuratedProcessShutdown {
    // Both registries are fenced synchronously. Repeated callers share the first
    // process deadline and cannot renew the shutdown budget.
    CuratedProcessShutdown::begin(capability.begin().graceful())
}

pub(crate) async fn finish(
    operation: anyhow::Result<()>,
    capability: &ProcessFinalCapability,
) -> anyhow::Result<()> {
    finish_with(operation, async {
        let deadline = capability.begin().graceful();
        let shutdown = begin(capability);
        let observed = shutdown.wait_until(deadline).await;
        // An optional sync attempt may fail while all of its ownership is clean.
        // Only incomplete callbacks/native ownership make process cleanup fail.
        if observed.is_complete() && observed.clean_native_ownership() {
            Ok(())
        } else {
            anyhow::bail!(
                "process shutdown did not confirm all curated plugin callbacks and native workers completed with clean ownership"
            )
        }
    })
    .await
}

async fn finish_with(
    operation: anyhow::Result<()>,
    cleanup: impl std::future::Future<Output = anyhow::Result<()>>,
) -> anyhow::Result<()> {
    match (operation, cleanup.await) {
        (Ok(()), cleanup) => cleanup,
        (Err(operation), Ok(())) => Err(operation),
        (Err(operation), Err(cleanup)) => {
            Err(operation.context(format!("process cleanup also failed: {cleanup:#}")))
        }
    }
}

#[cfg(test)]
#[path = "process_final_tests.rs"]
mod tests;
