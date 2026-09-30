//! Process-backed model manager with a host-owned, owner-fenced synchronous mirror.

use std::fmt;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use codex_component_host::ComponentBinding;
use codex_component_host::ComponentSession;
use codex_component_host::HostDependencyRegistry;
use codex_component_host::HostOperationScope;
use codex_http_client::HttpClientFactory;
use codex_login::AuthManager;
use codex_model_catalog_api::CatalogAuthState;
use codex_model_catalog_api::CatalogOutcome;
use codex_model_catalog_api::CatalogSource;
use codex_model_catalog_api::OPEN;
use codex_model_catalog_api::OpenCatalog;
use codex_model_catalog_api::REFRESH;
use codex_model_catalog_api::REFRESH_ETAG;
use codex_model_catalog_api::RefreshCatalog;
use codex_model_catalog_api::RefreshEtag;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CoreResult;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelsResponse;
use serde_json::Value;
use tokio::sync::Mutex;
use tokio::sync::TryLockError;

use super::ModelsManager;
use super::ModelsManagerFuture;
use super::RefreshStrategy;
use super::construct_model_info;
use super::construct_model_info_from_candidates;
use super::default_model_from_available;
use super::requested_model_is_available;
use crate::ModelsManagerConfig;
use crate::collaboration_mode_presets::builtin_collaboration_mode_presets;

#[path = "component_mirror.rs"]
mod mirror;
use mirror::CatalogMirror;

/// Host-only dependencies retained with an accepted operation. The scope contains
/// its per-call HTTP factory and policy snapshot; none of that is serialized.
pub struct CatalogRequestAuthority {
    pub auth: CatalogAuthState,
    pub scope: HostOperationScope,
}

/// Live provider authority, independent of the selected catalog implementation.
/// Snapshots must resolve lazy command auth before capturing identity. Every
/// publication and read checks current authority, including policy changes.
pub trait CatalogAuthority: fmt::Debug + Send + Sync {
    fn snapshot(&self, http_client_factory: HttpClientFactory)
        -> ModelsManagerFuture<'_, CoreResult<CatalogRequestAuthority>>;
    fn is_current(&self, auth: &CatalogAuthState) -> bool;
    fn set_api_key_model_discovery_enabled(&self, enabled: bool);
    /// Retire this manager's provider scope and invalidate all previously published snapshots.
    fn revoke(&self);
}

struct Inner {
    session: ComponentSession,
    authority: Arc<dyn CatalogAuthority>,
    source: CatalogSource,
    mirror: CatalogMirror,
    operations: Mutex<()>,
}

/// Implements the existing native manager API while making process failures
/// visible through `check_health`. Engine callers must check health before inference.
pub struct ProcessModelsManager {
    inner: Arc<Inner>,
    auth_manager: Option<Arc<AuthManager>>,
}

impl fmt::Debug for ProcessModelsManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("ProcessModelsManager").finish_non_exhaustive()
    }
}

impl ProcessModelsManager {
    pub async fn connect(
        binding: ComponentBinding,
        mut open: OpenCatalog,
        authority: Arc<dyn CatalogAuthority>,
        registry: HostDependencyRegistry,
        auth_manager: Option<Arc<AuthManager>>,
        http_client_factory: HttpClientFactory,
    ) -> Result<Self> {
        authority.set_api_key_model_discovery_enabled(open.auth.api_key_discovery_enabled);
        let request = authority.snapshot(http_client_factory).await?;
        open.auth = request.auth.clone();
        let session = binding.connect_with_broker(registry).await?;
        let value = session.call_scoped(OPEN, serde_json::to_value(&open)?, request.scope).await?;
        let outcome: CatalogOutcome = serde_json::from_value(value).context("decode selected catalog initialization")?;
        let mirror = CatalogMirror::default();
        mirror.publish(&open.source, request.auth, outcome, authority.as_ref()).await?;
        mirror.check_health(authority.as_ref()).await?;
        Ok(Self { inner: Arc::new(Inner { session, authority, source: open.source,
            mirror, operations: Mutex::new(()) }), auth_manager })
    }

    /// Explicit process shutdown after the owning model-services scope stops accepting work.
    pub async fn close(&self) -> Result<()> {
        self.inner.authority.revoke();
        let _guard = self.inner.operations.lock().await;
        self.inner.session.close().await
    }

    async fn refresh(&self, operation: RefreshOperation, http_client_factory: HttpClientFactory) {
        let inner = Arc::clone(&self.inner);
        // Native discovery/cache operations retain ownership once accepted. A
        // canceled UI/turn waiter must not abandon a cache write halfway through.
        let task = tokio::spawn(async move {
            let _guard = inner.operations.lock().await;
            let result = inner.perform(operation, http_client_factory).await;
            if result.is_err() {
                inner.mirror.fail_transport().await;
            }
        });
        if task.await.is_err() {
            self.inner.mirror.fail_transport().await;
        }
    }
}

enum RefreshOperation {
    Catalog(RefreshStrategy),
    Etag(String),
}

impl Inner {
    async fn perform(&self, operation: RefreshOperation, http_client_factory: HttpClientFactory) -> Result<()> {
        let request = self.authority.snapshot(http_client_factory).await?;
        let (method, params): (&str, Value) = match operation {
            RefreshOperation::Catalog(strategy) => (REFRESH, serde_json::to_value(RefreshCatalog {
                auth: request.auth.clone(), strategy: strategy.into(),
            })?),
            RefreshOperation::Etag(etag) => (REFRESH_ETAG, serde_json::to_value(RefreshEtag {
                auth: request.auth.clone(), etag,
            })?),
        };
        let response = self.session.call_scoped(method, params, request.scope).await?;
        let outcome = serde_json::from_value(response).context("decode selected catalog response")?;
        self.mirror.publish(&self.source, request.auth, outcome, self.authority.as_ref()).await
    }
}

impl ModelsManager for ProcessModelsManager {
    fn check_health(&self) -> ModelsManagerFuture<'_, CoreResult<()>> {
        Box::pin(async { self.inner.mirror.check_health(self.inner.authority.as_ref()).await })
    }

    fn set_api_key_model_discovery_enabled(&self, enabled: bool) {
        self.inner.authority.set_api_key_model_discovery_enabled(enabled);
    }

    fn raw_model_catalog(&self, strategy: RefreshStrategy, http_client_factory: HttpClientFactory)
        -> ModelsManagerFuture<'_, ModelsResponse>
    {
        Box::pin(async move {
            self.refresh(RefreshOperation::Catalog(strategy), http_client_factory).await;
            ModelsResponse { models: self.get_remote_models().await }
        })
    }

    fn refresh_after_auth_change(&self, http_client_factory: HttpClientFactory) -> ModelsManagerFuture<'_, ()> {
        Box::pin(self.refresh(RefreshOperation::Catalog(RefreshStrategy::OnlineIfUncached), http_client_factory))
    }

    fn get_remote_models(&self) -> ModelsManagerFuture<'_, Vec<ModelInfo>> {
        Box::pin(self.inner.mirror.models(self.inner.authority.as_ref()))
    }

    fn try_get_remote_models(&self) -> Result<Vec<ModelInfo>, TryLockError> {
        self.inner.mirror.try_models(self.inner.authority.as_ref())
    }

    fn auth_manager(&self) -> Option<&AuthManager> { self.auth_manager.as_deref() }

    fn catalog_uses_codex_backend(&self) -> bool {
        self.inner.mirror.uses_codex_backend(self.inner.authority.as_ref())
    }

    fn list_collaboration_modes(&self) -> Vec<CollaborationModeMask> {
        builtin_collaboration_mode_presets()
    }

    fn get_default_model<'a>(&'a self, model: &'a Option<String>, allow_provider_model_fallback: bool,
        strategy: RefreshStrategy, http_client_factory: HttpClientFactory) -> ModelsManagerFuture<'a, String>
    {
        Box::pin(async move {
            if !matches!(self.inner.source, CatalogSource::Static { .. } | CatalogSource::ProviderStatic { .. }) {
                if let Some(model) = model { return model.clone(); }
                return default_model_from_available(self.list_models(strategy, http_client_factory).await);
            }
            let available = self.list_models(strategy, http_client_factory).await;
            if allow_provider_model_fallback {
                if requested_model_is_available(model.as_deref(), &available)
                    && let Some(model) = model { return model.clone(); }
                return default_model_from_available(available);
            }
            model.clone().unwrap_or_else(|| default_model_from_available(available))
        })
    }

    fn get_model_info<'a>(&'a self, model: &'a str, config: &'a ModelsManagerConfig)
        -> ModelsManagerFuture<'a, ModelInfo>
    {
        Box::pin(async move {
            let models = self.get_remote_models().await;
            match &self.inner.source {
                CatalogSource::Authoritative => construct_model_info(model,
                    models.into_iter().find(|candidate| candidate.slug == model), config),
                CatalogSource::BundledMerge | CatalogSource::Static { .. } | CatalogSource::ProviderStatic { .. } =>
                    construct_model_info_from_candidates(model, &models, config),
            }
        })
    }

    fn refresh_if_new_etag(&self, etag: String, http_client_factory: HttpClientFactory) -> ModelsManagerFuture<'_, ()> {
        Box::pin(self.refresh(RefreshOperation::Etag(etag), http_client_factory))
    }
}

impl From<RefreshStrategy> for codex_model_catalog_api::RefreshStrategy {
    fn from(value: RefreshStrategy) -> Self {
        match value {
            RefreshStrategy::Online => Self::Online,
            RefreshStrategy::Offline => Self::Offline,
            RefreshStrategy::OnlineIfUncached => Self::OnlineIfUncached,
        }
    }
}

fn unavailable() -> CodexErr {
    CodexErr::Fatal("selected model catalog is unavailable; retry catalog refresh before inference".to_owned())
}

#[cfg(test)]
#[path = "component_process_tests.rs"]
mod process_tests;
