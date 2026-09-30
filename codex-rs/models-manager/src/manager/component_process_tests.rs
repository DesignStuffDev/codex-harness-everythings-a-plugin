//! Actual installed native worker tests; the source package is removed before use.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Result;
use codex_component_host::ComponentCatalog;
use codex_component_host::DependencyContext;
use codex_component_host::DependencyError;
use codex_component_host::DependencyErrorCode;
use codex_component_host::DependencyFuture;
use codex_component_host::DependencyOwner;
use codex_component_host::HostDependencyAuthority;
use codex_component_host::HostDependencyDescriptor;
use codex_component_host::HostDependencyRegistry;
use codex_component_host::HostDependencyService;
use codex_component_host::HostOperationScope;
use codex_http_client::DEFAULT_HTTP_CLIENT_FACTORY;
use codex_http_client::HttpClientFactory;
use codex_model_catalog_api::*;
use codex_protocol::auth::AuthMode;
use codex_protocol::error::Result as CoreResult;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelsResponse;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tokio::sync::watch;
use tokio::time::Instant;

use super::CatalogAuthority;
use super::CatalogRequestAuthority;
use super::ModelsManager;
use super::ModelsManagerFuture;
use super::ProcessModelsManager;
use crate::ModelsManagerConfig;
use crate::manager::RefreshStrategy as NativeRefresh;
use crate::manager::StaticModelsManager;

#[derive(Debug)]
struct Host {
    auth: Mutex<CatalogAuthState>,
    models: Vec<ModelInfo>,
    fetches: AtomicUsize,
    fail: AtomicBool,
    revoked: AtomicBool,
    changes: watch::Sender<u64>,
}

impl Host {
    fn new() -> Arc<Self> {
        let mut model = crate::bundled_models_response().unwrap().models.remove(0);
        model.slug = "component-native-model".to_owned();
        model.context_window = Some(32768);
        Arc::new(Self {
            auth: Mutex::new(CatalogAuthState { owner_generation: 1, identity: Some("catalog-test-owner".to_owned()),
                auth_mode: Some(AuthMode::ApiKey), uses_codex_backend: false, has_command_auth: false,
                supports_api_key_models: true, has_provider_api_key: true, api_key_discovery_enabled: false }),
            models: vec![model], fetches: AtomicUsize::new(0), fail: AtomicBool::new(false),
            revoked: AtomicBool::new(false), changes: watch::channel(0).0,
        })
    }

    fn registry(self: &Arc<Self>) -> Result<HostDependencyRegistry> {
        let mut registry = HostDependencyRegistry::new();
        registry.register_optional(HostDependencyDescriptor { name: ENDPOINT_SERVICE.to_owned(),
            version: ENDPOINT_VERSION, operations: vec!["fetch".to_owned(), "validate_owner".to_owned()] },
            self.clone(), self.clone())?;
        Ok(registry)
    }
}

impl CatalogAuthority for Host {
    fn snapshot(&self, factory: HttpClientFactory) -> ModelsManagerFuture<'_, CoreResult<CatalogRequestAuthority>> {
        Box::pin(async move {
            let auth = self.auth.lock().unwrap().clone();
            let scope = HostOperationScope::new(DependencyOwner { generation: auth.owner_generation,
                identity: auth.identity.clone() }, Instant::now() + Duration::from_secs(10))
                .with_request_context(factory);
            Ok(CatalogRequestAuthority { auth, scope })
        })
    }
    fn is_current(&self, auth: &CatalogAuthState) -> bool {
        !self.revoked.load(Ordering::SeqCst) && *self.auth.lock().unwrap() == *auth
    }
    fn set_api_key_model_discovery_enabled(&self, enabled: bool) {
        let mut auth = self.auth.lock().unwrap();
        if auth.api_key_discovery_enabled != enabled {
            auth.api_key_discovery_enabled = enabled;
            auth.owner_generation += 1;
            self.changes.send_modify(|generation| *generation += 1);
        }
    }
    fn revoke(&self) {
        self.revoked.store(true, Ordering::SeqCst);
        self.changes.send_modify(|generation| *generation += 1);
    }
}

impl HostDependencyAuthority for Host {
    fn check(&self, owner: &DependencyOwner) -> Result<(), DependencyError> {
        let auth = self.auth.lock().unwrap();
        if self.revoked.load(Ordering::SeqCst) || owner.generation != auth.owner_generation || owner.identity != auth.identity {
            Err(DependencyError::new(DependencyErrorCode::OwnerChanged))
        } else { Ok(()) }
    }
    fn revoked(&self, owner: DependencyOwner) -> DependencyFuture<'_, ()> {
        Box::pin(async move {
            let mut changes = self.changes.subscribe();
            while self.check(&owner).is_ok() {
                if changes.changed().await.is_err() { return; }
            }
        })
    }
}

impl HostDependencyService for Host {
    fn call(&self, context: DependencyContext, method: String, params: Value) -> DependencyFuture<'_, Result<Value, DependencyError>> {
        Box::pin(async move {
            context.check_authority()?;
            assert!(context.request_context::<HttpClientFactory>().is_some());
            let value = match method.as_str() {
                "validate_owner" => {
                    let request: ValidateOwner = serde_json::from_value(params)
                        .map_err(|_| DependencyError::new(DependencyErrorCode::InvalidRequest))?;
                    serde_json::to_value(OwnerValidation { matches: request.identity.as_deref() == context.owner_identity() })
                }
                "fetch" => {
                    let _: FetchModels = serde_json::from_value(params)
                        .map_err(|_| DependencyError::new(DependencyErrorCode::InvalidRequest))?;
                    self.fetches.fetch_add(1, Ordering::SeqCst);
                    serde_json::to_value(if self.fail.load(Ordering::SeqCst) {
                        FetchOutcome::Failed { error: CatalogError::Timeout }
                    } else {
                        FetchOutcome::Fetched { models: self.models.clone(), etag: Some("catalog-etag".to_owned()),
                            identity: context.owner_identity().unwrap().to_owned() }
                    })
                }
                _ => return Err(DependencyError::new(DependencyErrorCode::InvalidRequest)),
            };
            value.map_err(|_| DependencyError::new(DependencyErrorCode::ServiceFailure))
        })
    }
}

fn install(home: &std::path::Path) -> Result<codex_component_host::ComponentBinding> {
    let package = tempfile::tempdir()?;
    let name = format!("catalog{}", std::env::consts::EXE_SUFFIX);
    std::fs::copy(codex_utils_cargo_bin::cargo_bin("codex-model-catalog-native-plugin")?, package.path().join(&name))?;
    std::fs::write(package.path().join("codex-component.json"), serde_json::to_vec(&json!({
        "api_version":1,"id":"test.native-catalog","version":"1.0.0","entrypoint":name,
        "components":[{"kind":"model_catalog","name":"default","contract_version":1}]
    }))?)?;
    codex_component_host::install(home, package.path())?;
    codex_component_host::select(home, "model_catalog", "default", Some("test.native-catalog"))?;
    package.close()?;
    Ok(ComponentCatalog::load(home)?.selected("model_catalog", "default").unwrap())
}

async fn connect(home: &std::path::Path, host: Arc<Host>, refresh: RefreshStrategy) -> Result<ProcessModelsManager> {
    let open = OpenCatalog { contract_version: CONTRACT_VERSION, client_version: "9.8.7".to_owned(),
        source: CatalogSource::Authoritative, cache: CatalogCache::NativeDirectory { directory: home.join("cache") },
        auth: host.auth.lock().unwrap().clone(), refresh };
    ProcessModelsManager::connect(install(home)?, open, host.clone(), host.registry()?, None, DEFAULT_HTTP_CLIENT_FACTORY).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_native_catalog_preserves_discovery_opt_in_and_real_model_metadata() -> Result<()> {
    let home = tempfile::tempdir()?;
    let host = Host::new();
    let manager = connect(home.path(), host.clone(), RefreshStrategy::Online).await?;
    assert_eq!(host.fetches.load(Ordering::SeqCst), 0);
    assert_eq!(manager.get_remote_models().await, Vec::new());
    manager.set_api_key_model_discovery_enabled(true);
    let catalog = manager.raw_model_catalog(NativeRefresh::Online, DEFAULT_HTTP_CLIENT_FACTORY).await;
    assert_eq!(catalog.models, host.models);
    assert_eq!(host.fetches.load(Ordering::SeqCst), 1);
    let baseline = StaticModelsManager::new(None, ModelsResponse { models: host.models.clone() });
    let config = ModelsManagerConfig { model_context_window: Some(16000), ..Default::default() };
    assert_eq!(manager.get_model_info("component-native-model", &config).await,
        baseline.get_model_info("component-native-model", &config).await);
    manager.check_health().await?;
    manager.close().await?;
    assert!(manager.check_health().await.is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_worker_restart_reuses_host_versioned_cache_without_fetching() -> Result<()> {
    let home = tempfile::tempdir()?;
    let host = Host::new();
    host.set_api_key_model_discovery_enabled(true);
    let manager = connect(home.path(), host.clone(), RefreshStrategy::Online).await?;
    assert_eq!(manager.get_remote_models().await, host.models);
    manager.close().await?;
    let restarted_host = Host::new();
    restarted_host.set_api_key_model_discovery_enabled(true);
    let restarted = connect(home.path(), restarted_host.clone(), RefreshStrategy::Offline).await?;
    assert_eq!(restarted.get_remote_models().await, restarted_host.models);
    assert_eq!(restarted_host.fetches.load(Ordering::SeqCst), 0);
    let cache: Value = serde_json::from_slice(&std::fs::read(home.path().join("cache/models_cache.json"))?)?;
    assert_eq!(cache["client_version"], json!("9.8.7"));
    restarted.close().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_authoritative_failure_invalidates_cache_and_later_refresh_recovers() -> Result<()> {
    let home = tempfile::tempdir()?;
    let host = Host::new();
    host.set_api_key_model_discovery_enabled(true);
    let manager = connect(home.path(), host.clone(), RefreshStrategy::Online).await?;
    host.fail.store(true, Ordering::SeqCst);
    assert_eq!(manager.raw_model_catalog(NativeRefresh::Online, DEFAULT_HTTP_CLIENT_FACTORY).await.models, Vec::new());
    manager.check_health().await?;
    host.fail.store(false, Ordering::SeqCst);
    assert_eq!(manager.raw_model_catalog(NativeRefresh::Offline, DEFAULT_HTTP_CLIENT_FACTORY).await.models, Vec::new());
    assert_eq!(manager.raw_model_catalog(NativeRefresh::Online, DEFAULT_HTTP_CLIENT_FACTORY).await.models, host.models);
    manager.close().await?;
    Ok(())
}
