//! Native catalog process service, reusing the native manager and file cache.
//!
//! The process serializes these calls. The host owns live credentials, routing,
//! managed HTTP policy and final owner-fenced publication; this service owns
//! native merge/discovery/cache policy and its state directory.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use anyhow::ensure;
use codex_component_host::DependencyClient;
use codex_component_host::DependencyRequestScope;
use codex_http_client::DEFAULT_HTTP_CLIENT_FACTORY;
use codex_model_catalog_api::CONTRACT_VERSION;
use codex_model_catalog_api::CatalogAuthState;
use codex_model_catalog_api::CatalogCache;
use codex_model_catalog_api::CatalogError;
use codex_model_catalog_api::CatalogOutcome;
use codex_model_catalog_api::CatalogSnapshot;
use codex_model_catalog_api::CatalogSource;
use codex_model_catalog_api::OpenCatalog;
use codex_model_catalog_api::RefreshCatalog;
use codex_model_catalog_api::RefreshEtag;
use codex_protocol::openai_models::ModelInfo;

use super::ModelsManager;
use super::OpenAiModelsManager;
use super::RefreshStrategy;
use super::component_endpoint::BrokerModelsEndpoint;

enum NativeManager {
    Remote(OpenAiModelsManager),
    Static(Vec<ModelInfo>),
}

/// One native component session. Methods require exclusive ownership so a scope
/// cannot be replaced while its cache write or dependency call is still running.
pub struct NativeCatalogService {
    manager: NativeManager,
    endpoint: Arc<BrokerModelsEndpoint>,
    auth: CatalogAuthState,
    revision: u64,
}

impl NativeCatalogService {
    pub async fn open(
        open: OpenCatalog,
        state_dir: &Path,
        client: DependencyClient,
        scope: DependencyRequestScope,
    ) -> Result<(Self, CatalogOutcome)> {
        ensure!(open.contract_version == CONTRACT_VERSION, "unsupported catalog contract version");
        ensure!(!open.client_version.is_empty(), "catalog client version is required");
        ensure!(open.auth.owner_generation == scope.owner_generation(), "catalog scope owner mismatch");
        let endpoint = Arc::new(BrokerModelsEndpoint::new(client, open.auth.clone(), scope));
        let manager = match open.source {
            CatalogSource::Static { models } | CatalogSource::ProviderStatic { models } => NativeManager::Static(models),
            source @ (CatalogSource::BundledMerge | CatalogSource::Authoritative) => {
                let dependency = Arc::clone(&endpoint) as Arc<dyn super::ModelsEndpointClient>;
                let mut manager = match open.cache {
                    CatalogCache::Disabled => OpenAiModelsManager::new_without_cache(dependency, None),
                    CatalogCache::ComponentState => OpenAiModelsManager::new(state_dir.join("catalog"), dependency, None),
                    CatalogCache::NativeDirectory { directory } => OpenAiModelsManager::new(directory, dependency, None),
                };
                // Cache compatibility belongs to the host requesting this operation,
                // not the build version of this independently packaged worker.
                manager.client_version = open.client_version.clone();
                manager.remote_models.get_mut().client_version = Some(open.client_version);
                if matches!(source, CatalogSource::Authoritative) {
                    manager = manager.with_provider_catalog();
                }
                manager.set_api_key_model_discovery_enabled(open.auth.api_key_discovery_enabled);
                NativeManager::Remote(manager)
            }
        };
        let mut service = Self { manager, endpoint, auth: open.auth, revision: 0 };
        let outcome = service.refresh_current(open.refresh.into()).await;
        Ok((service, outcome))
    }

    pub async fn refresh(&mut self, request: RefreshCatalog, scope: DependencyRequestScope) -> Result<CatalogOutcome> {
        self.begin(request.auth, scope)?;
        Ok(self.refresh_current(request.strategy.into()).await)
    }

    pub async fn refresh_etag(&mut self, request: RefreshEtag, scope: DependencyRequestScope) -> Result<CatalogOutcome> {
        self.begin(request.auth, scope)?;
        if !self.endpoint.owner_is_current().await {
            return Ok(self.outcome(Some(CatalogError::OwnerChanged)).await);
        }
        if let NativeManager::Remote(manager) = &self.manager {
            manager.refresh_if_new_etag(request.etag, DEFAULT_HTTP_CLIENT_FACTORY).await;
        }
        let failure = if self.endpoint.owner_is_current().await {
            self.endpoint.failure()
        } else {
            Some(CatalogError::OwnerChanged)
        };
        Ok(self.outcome(failure).await)
    }

    pub async fn snapshot(&mut self, auth: CatalogAuthState, scope: DependencyRequestScope) -> Result<CatalogOutcome> {
        self.begin(auth, scope)?;
        let failure = (!self.endpoint.owner_is_current().await).then_some(CatalogError::OwnerChanged);
        Ok(self.outcome(failure).await)
    }

    fn begin(&mut self, auth: CatalogAuthState, scope: DependencyRequestScope) -> Result<()> {
        ensure!(auth.owner_generation == scope.owner_generation(), "catalog scope owner mismatch");
        if let NativeManager::Remote(manager) = &self.manager {
            manager.set_api_key_model_discovery_enabled(auth.api_key_discovery_enabled);
        }
        self.endpoint.set_invocation(auth.clone(), scope);
        self.auth = auth;
        Ok(())
    }

    async fn refresh_current(&mut self, strategy: RefreshStrategy) -> CatalogOutcome {
        if !self.endpoint.owner_is_current().await {
            return self.outcome(Some(CatalogError::OwnerChanged)).await;
        }
        let mut failure = None;
        if let NativeManager::Remote(manager) = &self.manager
            && manager.refresh_available_models(strategy, &DEFAULT_HTTP_CLIENT_FACTORY).await.is_err()
        {
            failure = Some(self.endpoint.failure().unwrap_or(CatalogError::Unavailable));
        }
        if !self.endpoint.owner_is_current().await {
            failure = Some(CatalogError::OwnerChanged);
        }
        self.outcome(failure).await
    }

    async fn outcome(&mut self, error: Option<CatalogError>) -> CatalogOutcome {
        self.revision = self.revision.saturating_add(1);
        let (models, etag) = match &self.manager {
            NativeManager::Remote(manager) => {
                let models = manager.get_remote_models().await;
                let entry = manager.remote_models.read().await;
                let etag = (entry.identity == self.auth.identity).then(|| entry.etag.clone()).flatten();
                (models, etag)
            }
            NativeManager::Static(models) => (models.clone(), None),
        };
        let snapshot = CatalogSnapshot {
            contract_version: CONTRACT_VERSION,
            owner_generation: self.auth.owner_generation,
            identity: self.auth.identity.clone(),
            revision: self.revision,
            etag,
            // A stale-owner snapshot cannot serve as a fallback, even temporarily.
            models: if error == Some(CatalogError::OwnerChanged) { Vec::new() } else { models },
        };
        match error {
            Some(error) => CatalogOutcome::Failed { error, snapshot },
            None => CatalogOutcome::Ready { snapshot },
        }
    }
}

impl From<codex_model_catalog_api::RefreshStrategy> for RefreshStrategy {
    fn from(value: codex_model_catalog_api::RefreshStrategy) -> Self {
        match value {
            codex_model_catalog_api::RefreshStrategy::Online => Self::Online,
            codex_model_catalog_api::RefreshStrategy::Offline => Self::Offline,
            codex_model_catalog_api::RefreshStrategy::OnlineIfUncached => Self::OnlineIfUncached,
        }
    }
}

#[cfg(test)]
#[path = "component_cache_tests.rs"]
mod tests;
