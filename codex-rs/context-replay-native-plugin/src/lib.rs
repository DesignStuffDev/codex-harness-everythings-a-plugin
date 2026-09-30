//! Native replay worker. All history algorithms execute locally in this process.
//!
//! The service is stateless between requests and needs no host callback or
//! dependency grant. One accepted reconstruction runs at a time on a blocking
//! worker; transport readers/writers remain independent. Accepted work is drained
//! before graceful shutdown. The host supervisor bounds forced termination.

use codex_component_host::ComponentServer;
use codex_context_engine::ContextManager;
use codex_context_replay::RECONSTRUCT_METHOD;
use codex_context_replay::REPLAY_COMPONENT_KIND;
use codex_context_replay::REPLAY_COMPONENT_NAME;
use codex_context_replay::ReplayInput;
use codex_context_replay::reconstruct;

/// A canonical diagnostic that cannot contain request history or plugin data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayWorkerError;

impl std::fmt::Display for ReplayWorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("native context replay worker failed")
    }
}

impl std::error::Error for ReplayWorkerError {}

/// Serve the component's chunked persistent protocol on stdin/stdout.
///
/// Large histories use the shared transport's streamed physical frames. The
/// current reducer and JSON codec still materialize the complete owned history;
/// chunking does not claim a constant-memory reconstruction algorithm.
pub async fn run_stdio() -> Result<(), ReplayWorkerError> {
    let mut server = ComponentServer::stdio()
        .await
        .map_err(|_| ReplayWorkerError)?;
    while let Some(mut request) = server.next().await.map_err(|_| ReplayWorkerError)? {
        if request.is_control
            || request.component.kind != REPLAY_COMPONENT_KIND
            || request.component.name != REPLAY_COMPONENT_NAME
            || request.method != RECONSTRUCT_METHOD
        {
            request
                .respond(Err("invalid context replay request".to_owned()))
                .await
                .map_err(|_| ReplayWorkerError)?;
            continue;
        }
        let params = std::mem::take(&mut request.params);
        let result = tokio::task::spawn_blocking(move || {
            let input: ReplayInput = serde_json::from_value(params)
                .map_err(|_| "invalid context replay input".to_owned())?;
            let reconstructed = reconstruct::<ContextManager>(input);
            serde_json::to_value(reconstructed)
                .map_err(|_| "context replay output encoding failed".to_owned())
        })
        .await
        .unwrap_or_else(|_| Err("native context reconstruction failed".to_owned()));
        request
            .respond(result)
            .await
            .map_err(|_| ReplayWorkerError)?;
    }
    server.finish().await.map_err(|_| ReplayWorkerError)
}
