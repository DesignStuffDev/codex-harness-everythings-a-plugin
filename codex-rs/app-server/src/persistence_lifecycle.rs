//! Storage ownership outlives abortable request-processing tasks.

use std::io;
use std::sync::Arc;
use std::time::Duration;

use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreShutdownGuard;
use tokio::time::timeout_at;

/// Each owner fences the store when dropped, even if detached work retains
/// other store references. Normal paths also await and report the final drain.
pub(crate) struct StoreShutdownGuard {
    store: ThreadStoreShutdownGuard,
}

impl StoreShutdownGuard {
    pub(crate) fn new(store: Arc<dyn ThreadStore>) -> Self {
        Self {
            store: ThreadStoreShutdownGuard::new(store),
        }
    }

    pub(crate) fn begin_shutdown(&self) {
        self.store.begin_shutdown();
    }

    pub(crate) async fn finish<T>(&self, result: io::Result<T>) -> io::Result<T> {
        self.begin_shutdown();
        let shutdown = match timeout_at(
            crate::process_final::deadline_after(Duration::from_secs(120)),
            self.store.shutdown(),
        )
        .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(io::Error::other(
                "thread-store shutdown failed; accepted write outcomes may be unknown",
            )),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "thread-store shutdown timed out; accepted write outcomes may be unknown",
            )),
        };
        match (result, shutdown) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(primary), Err(cleanup)) => Err(io::Error::new(
                primary.kind(),
                format!("{primary}; {cleanup}"),
            )),
        }
    }
}
