//! Transport composition only. The supplied factory owns the selected matcher;
//! this runner depends on neither codex-core nor a concrete search engine.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_component_host::ComponentServer;
use codex_file_search_api::CloseCleanup;
use tokio::task::JoinSet;

use crate::SearchBackendFactory;
use crate::SearchService;
use crate::ServiceEnvironment;
use crate::ServiceLimits;

/// Serve a separately packaged search implementation on persistent stdio.
/// Startup cwd is captured once; the executable must be launched in its explicit
/// provider base directory. Every admitted operation is drained before return.
pub async fn run_stdio(
    factory: Arc<dyn SearchBackendFactory>,
    ceilings: ServiceLimits,
) -> Result<()> {
    let mut server = ComponentServer::stdio_with_limits(crate::payload_limits()).await?;
    let initialization = server.initialization();
    let service = SearchService::new(
        factory,
        ServiceEnvironment {
            startup_dir: std::env::current_dir().context("read file-search startup directory")?,
            plugin_config: initialization.config.clone(),
            state_dir: initialization.state_dir.clone(),
        },
        ceilings,
    )?;
    let mut handlers = JoinSet::new();
    let read_result: Result<()> = async {
        loop {
            tokio::select! {
                received = server.next() => match received {
                    Ok(Some(mut request)) => {
                        let admitted = if request.component.kind == "file_search" && request.component.name == "default" {
                            // Never move admission into the spawned handler. The
                            // reader order is the Preparing/release ordering fence.
                            service.admit(&request.method, request.is_control.into(), std::mem::take(&mut request.params))
                        } else {
                            Err("component must select file_search/default".to_owned())
                        };
                        handlers.spawn(async move {
                            let response = match admitted {
                                Ok(pending) => pending.wait().await,
                                Err(error) => Err(error),
                            };
                            request.respond(response).await
                        });
                    }
                    Ok(None) => break Ok(()),
                    Err(error) => break Err(error),
                },
                completed = handlers.join_next(), if !handlers.is_empty() => {
                    if let Some(completed) = completed {
                        completed.context("file-search response handler panicked")??;
                    }
                }
            }
        }
    }.await;
    // Includes EOF, malformed transport, a failed response and normal protocol
    // shutdown. Accepted owners are independent of dropped response observers.
    let outcome = service.shutdown().await;
    let mut handler_error = None;
    while let Some(completed) = handlers.join_next().await {
        if let Err(error) = completed
            .context("file-search response handler panicked")
            .and_then(|result| result)
            && handler_error.is_none()
        {
            handler_error = Some(error);
        }
    }
    read_result?;
    if let Some(error) = handler_error {
        return Err(error);
    }
    ensure!(
        outcome.cleanup == CloseCleanup::Joined,
        "file-search backend cleanup is unconfirmed"
    );
    // Domain operation failures were preserved in typed release/shutdown replies.
    // A clean transport close acknowledges drainage, not success of the search.
    server.finish().await
}
