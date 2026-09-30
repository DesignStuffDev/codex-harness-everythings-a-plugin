//! Native discovery, merge and cache policy without a Codex engine dependency.

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
use codex_component_host::ComponentServer;
use codex_component_host::ComponentServerRequest;
use codex_component_host::HostDependencyDescriptor;
use codex_model_catalog_api::ENDPOINT_SERVICE;
use codex_model_catalog_api::ENDPOINT_VERSION;
use codex_model_catalog_api::OPEN;
use codex_model_catalog_api::REFRESH;
use codex_model_catalog_api::REFRESH_ETAG;
use codex_model_catalog_api::SNAPSHOT;
use codex_models_manager::manager::component_service::NativeCatalogService;
use serde_json::Value;

/// Run the native catalog service. The dedicated transport reader remains live
/// during dependency calls; native manager mutations are deliberately serialized.
pub async fn run_stdio() -> Result<()> {
    let descriptor = HostDependencyDescriptor {
        name: ENDPOINT_SERVICE.to_owned(), version: ENDPOINT_VERSION,
        operations: vec!["fetch".to_owned(), "validate_owner".to_owned()],
    };
    let mut server = ComponentServer::stdio_with_broker(&[descriptor]).await?;
    let client = server.dependencies().context("native catalog needs endpoint dependency")?;
    let state_dir = server.initialization().state_dir.clone();
    let mut first = server.next().await?.context("catalog initialization missing")?;
    ensure!(first.method == OPEN && first.component.kind == "model_catalog" && !first.is_control,
        "first request must initialize model catalog");
    let scope = first.dependency_scope().cloned().context("catalog request needs host scope")?;
    let open = serde_json::from_value(std::mem::take(&mut first.params))?;
    let (mut service, outcome) = match NativeCatalogService::open(open, &state_dir, client, scope).await {
        Ok(opened) => opened,
        Err(error) => {
            first.respond(Err("native catalog initialization failed".to_owned())).await?;
            return Err(error);
        }
    };
    first.respond(Ok(serde_json::to_value(outcome)?)).await?;
    while let Some(mut request) = server.next().await? {
        let outcome = dispatch(&mut service, &mut request).await;
        request.respond(outcome.map_err(|_| "native catalog request failed".to_owned())).await?;
    }
    server.finish().await
}

async fn dispatch(service: &mut NativeCatalogService, request: &mut ComponentServerRequest) -> Result<Value> {
    ensure!(request.component.kind == "model_catalog" && !request.is_control, "invalid catalog request kind");
    let scope = request.dependency_scope().cloned().context("catalog request needs host scope")?;
    let params = std::mem::take(&mut request.params);
    let outcome = match request.method.as_str() {
        REFRESH => service.refresh(serde_json::from_value(params)?, scope).await?,
        REFRESH_ETAG => service.refresh_etag(serde_json::from_value(params)?, scope).await?,
        SNAPSHOT => service.snapshot(serde_json::from_value(params)?, scope).await?,
        _ => bail!("unknown catalog operation"),
    };
    Ok(serde_json::to_value(outcome)?)
}
