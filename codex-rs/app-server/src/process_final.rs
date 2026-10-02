//! Process-final authority is supplied by executable callers only.

use codex_core_plugins::PluginStartupProcessShutdown;
use codex_utils_process::process_shutdown::ProcessFinalCapability;
use std::future::Future;
use std::io;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::Instant;

pub(crate) fn begin(capability: Option<&ProcessFinalCapability>) {
    if let Some(capability) = capability {
        let deadline = capability.begin();
        // Admission closure and native stop precede every local drain.
        let _shutdown = PluginStartupProcessShutdown::begin(deadline.graceful());
    }
}

pub(crate) fn deadline_after(budget: Duration) -> Instant {
    let local = Instant::now() + budget;
    codex_utils_process::process_shutdown::current().map_or(local, |deadline| {
        local.min(Instant::from_std(deadline.graceful()))
    })
}

/// Session/processor drains may include accepted storage writes. During final
/// process stop their entire remaining grace replaces legacy embedded budgets.
pub(crate) fn drain_deadline(legacy_budget: Duration) -> Instant {
    codex_utils_process::process_shutdown::current().map_or_else(
        || Instant::now() + legacy_budget,
        |deadline| Instant::from_std(deadline.graceful()),
    )
}

pub(crate) async fn phase<T>(future: impl Future<Output = io::Result<T>>) -> io::Result<T> {
    match codex_utils_process::process_shutdown::current() {
        Some(deadline) => tokio::time::timeout_at(Instant::from_std(deadline.graceful()), future)
            .await
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "process shutdown phase timed out; cleanup is unconfirmed",
                )
            })?,
        None => future.await,
    }
}

pub(crate) async fn join<T>(mut handle: JoinHandle<T>) -> io::Result<T> {
    let outcome = match codex_utils_process::process_shutdown::current() {
        Some(deadline) => {
            match tokio::time::timeout_at(Instant::from_std(deadline.graceful()), &mut handle).await
            {
                Ok(outcome) => outcome,
                Err(_) => {
                    handle.abort();
                    // Observe the exact handle even if it finished concurrently
                    // with the deadline. Native blocking work can still outlive
                    // async cancellation; the watchdog stays armed.
                    let detail = match handle.await {
                        Err(error) if error.is_panic() => {
                            "shutdown task timed out and panicked; cleanup is unconfirmed"
                        }
                        Err(_) => {
                            "shutdown task timed out and was cancelled; cleanup is unconfirmed"
                        }
                        Ok(_) => {
                            "shutdown task completed after its deadline; cleanup is unconfirmed"
                        }
                    };
                    return Err(io::Error::new(io::ErrorKind::TimedOut, detail));
                }
            }
        }
        None => handle.await,
    };
    outcome.map_err(|_| io::Error::other("shutdown task failed"))
}

pub(crate) async fn finish<T>(
    result: io::Result<T>,
    capability: Option<&ProcessFinalCapability>,
) -> io::Result<T> {
    let Some(capability) = capability else {
        return result;
    };
    let deadline = capability.begin().graceful();
    let observed = PluginStartupProcessShutdown::begin(deadline)
        .wait_until(deadline)
        .await;
    let cleanup = if observed.is_complete() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "plugin startup task cleanup unconfirmed: {} callbacks pending, {} failed; {} featured tasks pending, {} panicked, {} join failures; {} HTTP constructors pending, {} failed",
            observed.curated.callbacks.pending,
            observed.curated.callbacks.failed,
            observed.featured.pending,
            observed.featured.panicked,
            observed.featured.join_failed,
            observed.constructors.pending,
            observed.constructors.failed,
        )))
    };
    crate::file_search_services::combine(result, cleanup)
}
