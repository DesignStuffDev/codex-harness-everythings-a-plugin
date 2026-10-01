//! Weak local search capability forwarding, without independent selection or
//! ownership of the embedded runtime's provider.

use std::io;

use codex_app_server::in_process::InProcessClientHandle;
use codex_file_search_runtime::FileSearchScopeFactory;

use crate::AppServerClient;
use crate::InProcessAppServerClient;

/// Capture before moving the public runtime handle into the facade worker.
/// Failure still drains that runtime and preserves both startup and cleanup
/// errors; callers must not silently select another search implementation.
pub(super) async fn capture(
    handle: InProcessClientHandle,
) -> io::Result<(InProcessClientHandle, FileSearchScopeFactory)> {
    match handle.file_search_scope_factory() {
        Ok(factory) => Ok((handle, factory)),
        Err(primary) => match handle.shutdown().await {
            Ok(()) => Err(primary),
            Err(cleanup) => Err(io::Error::new(
                primary.kind(),
                format!(
                    "{primary}; embedded runtime cleanup after search capability failure: {cleanup}"
                ),
            )),
        },
    }
}

impl InProcessAppServerClient {
    /// Scope capability from this exact embedded runtime, with no provider
    /// ownership. Missing or closed embedded services are explicit errors.
    pub fn file_search_scope_factory(&self) -> io::Result<FileSearchScopeFactory> {
        let factory = self.file_search_factory.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "in-process search provider is unavailable",
            )
        })?;
        factory.effective_policy().map_err(io::Error::other)?;
        Ok(factory.clone())
    }
}

impl AppServerClient {
    /// Only a remote transport has no local provider. An embedded provider
    /// failure must propagate rather than trigger local backend reselection.
    pub fn file_search_scope_factory(&self) -> io::Result<Option<FileSearchScopeFactory>> {
        match self {
            Self::InProcess(client) => client.file_search_scope_factory().map(Some),
            Self::Remote(_) => Ok(None),
        }
    }
}

#[cfg(test)]
#[path = "file_search_tests.rs"]
mod tests;
