use std::sync::Arc;

use crate::ThreadStore;
use crate::ThreadStoreFuture;

/// Owns a store's terminal lifetime independently of handles retained by tasks.
///
/// Create one guard in the outer scope that owns the store. This guard must not
/// be cloned or attached to individual users of a shared store: dropping the
/// owner invokes the backend's terminal shutdown hook. Process-backed stores
/// stop admission and initiate bounded cleanup for every handle; stores without
/// an owned external service retain their existing per-thread shutdown behavior.
/// Explicit shutdown observes the durability outcome; dropping the guard only
/// initiates cleanup and cannot promise that accepted work completed.
#[must_use = "retain the store owner until its terminal shutdown"]
pub struct ThreadStoreShutdownGuard {
    store: Arc<dyn ThreadStore>,
}

impl ThreadStoreShutdownGuard {
    pub fn new(store: Arc<dyn ThreadStore>) -> Self {
        Self { store }
    }

    /// Stop admission and initiate terminal cleanup without waiting.
    pub fn begin_shutdown(&self) {
        self.store.begin_shutdown_store();
    }

    /// Begin shutdown immediately, then return a future observing completion.
    /// Dropping this future before its first poll still leaves cleanup running.
    pub fn shutdown(&self) -> ThreadStoreFuture<'_, ()> {
        self.begin_shutdown();
        self.store.shutdown_store()
    }
}

impl Drop for ThreadStoreShutdownGuard {
    fn drop(&mut self) {
        self.begin_shutdown();
    }
}
