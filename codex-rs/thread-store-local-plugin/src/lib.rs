//! Standalone native LocalThreadStore service, independent of codex-core.

mod native;

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_component_host::ComponentServer;
use codex_thread_store_component::OPEN_METHOD;
use codex_thread_store_component::StorageInitialization;
use codex_thread_store_component::StorageService;
use tokio::task::JoinSet;

/// Serve one persistent component session over standard input/output.
/// The embedding executable supplies its Tokio runtime; only this storage
/// subsystem is initialized, and accepted work is drained before return.
pub async fn run_stdio() -> Result<()> {
    let mut server = ComponentServer::stdio().await?;
    let mut first = server
        .next()
        .await?
        .context("storage initialization missing")?;
    ensure!(
        first.method == OPEN_METHOD && !first.is_control,
        "first storage request must initialize the store"
    );
    ensure!(
        first.component.kind == "thread_store",
        "component kind must be thread_store"
    );
    let initialization: StorageInitialization =
        serde_json::from_value(std::mem::take(&mut first.params))?;
    let native::OpenedStore {
        store,
        capabilities,
        owner,
        state_db,
    } = match native::open(initialization).await {
        Ok(opened) => opened,
        Err(error) => {
            if let Err(reply) = first
                .respond(Err(format!("initialize native thread store: {error:#}")))
                .await
            {
                return Err(
                    error.context(format!("storage initialization reply failed: {reply:#}"))
                );
            }
            return Err(error);
        }
    };
    let service = Arc::new(StorageService::new(store));
    let mut handlers = JoinSet::new();
    let read_result = async {
        // Once initialization schedules maintenance, every exit path owns and
        // observes its cleanup, including a failed capability acknowledgement.
        first
            .respond(Ok(serde_json::to_value(capabilities)?))
            .await?;
        loop {
            tokio::select! {
                result = server.next() => match result {
                    Ok(Some(mut request)) => {
                        let service = Arc::clone(&service);
                        handlers.spawn(async move {
                            let result = if request.component.kind == "thread_store" {
                                service.handle(&request.method, std::mem::take(&mut request.params)).await
                            } else {
                                Err("component kind must be thread_store".to_owned())
                            };
                            request.respond(result).await
                        });
                    }
                    Ok(None) => break Ok(()),
                    Err(error) => break Err(error),
                },
                result = handlers.join_next(), if !handlers.is_empty() => {
                    if let Some(result) = result
                        && let Err(error) = result.context("storage handler panicked").and_then(|result| result)
                    {
                        break Err(error);
                    }
                }
            }
        }
    }.await;

    // Stop maintenance admission first without denying previously accepted
    // writer operations. Release leases before waiting for deletion handlers.
    owner.begin_shutdown();
    service.release_all_forks().await;
    service.begin_shutdown_migrations().await;
    let mut failures = Vec::new();
    if let Err(error) = read_result {
        failures.push(format!("storage request stream failed: {error:#}"));
    }
    while let Some(result) = handlers.join_next().await {
        if let Err(error) = result
            .context("storage handler panicked")
            .and_then(|result| result)
        {
            failures.push(format!("storage handler failed: {error:#}"));
        }
    }
    if let Err(error) = service.shutdown_migrations().await {
        failures.push(format!("storage migration cleanup failed: {error}"));
    }
    if let Err(error) = service.shutdown_writers().await {
        failures.push(format!("storage writer durability fence failed: {error}"));
    }
    // Maintenance can still own SQLite/rollout mutations after the last RPC.
    // A successful protocol shutdown is sent only after these workers settle.
    if let Err(error) = owner.shutdown().await {
        failures.push(format!("storage maintenance shutdown failed: {error}"));
    }
    // Close only the state runtime created inside this worker. The builtin
    // LocalThreadStore must never close the host's shared auxiliary state DB.
    if let Some(state_db) = state_db {
        state_db.close().await;
    }
    ensure!(failures.is_empty(), "{}", failures.join("; "));
    server.finish().await
}
