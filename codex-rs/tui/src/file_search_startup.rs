//! Compose local picker search from the final transport, after onboarding and
//! client replacement. Remote transports still search this machine's filesystem.

use std::io;
use std::path::Path;

use codex_file_search_runtime::SelectionContext;
use color_eyre::eyre::WrapErr;

use crate::app_server_session::AppServerSession;
use crate::file_search::FileSearchRuntime;
use crate::file_search::FileSearchSource;

pub(crate) async fn start(
    app_server: &AppServerSession,
    codex_home: &Path,
) -> color_eyre::Result<FileSearchRuntime> {
    let source = match app_server
        .file_search_scope_factory()
        .wrap_err("embedded file-search capability is unavailable")?
    {
        Some(factory) => FileSearchSource::Embedded(factory),
        // None is an actual remote connection, never an embedded failure. Keep
        // existing local filesystem semantics with an explicitly owned selected
        // provider. Selection failure is propagated without a native fallback.
        None => FileSearchSource::Local(SelectionContext {
            codex_home: codex_home.to_owned(),
            base_dir: std::env::current_dir()
                .wrap_err("file-search process base directory is unavailable")?,
        }),
    };
    FileSearchRuntime::start(source)
        .await
        .wrap_err("failed to start local file search")
}

/// Fence picker presentation before the embedded provider's global shutdown.
/// The outer runtime owner independently observes scope/local-provider cleanup.
pub(crate) async fn finish<T>(
    runtime: &FileSearchRuntime,
    app_server: AppServerSession,
    result: color_eyre::Result<T>,
) -> color_eyre::Result<T> {
    // This path ends the TUI, including startup errors and the event-loop exit.
    // Ordinary embedded-provider replacement never calls this finalizer.
    crate::process_final::begin();
    runtime.request_shutdown();
    combine(result, app_server.shutdown().await)
}

pub(crate) fn combine<T>(
    result: color_eyre::Result<T>,
    cleanup: io::Result<()>,
) -> color_eyre::Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(cleanup)) => Err(cleanup).wrap_err("App Server shutdown failed"),
        (Err(error), Err(cleanup)) => {
            Err(error.wrap_err(format!("App Server shutdown also failed: {cleanup}")))
        }
    }
}
