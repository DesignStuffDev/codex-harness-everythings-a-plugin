//! Regression coverage for the asynchronous ownership seam used by native workers.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use chrono::Utc;
use codex_http_client::DEFAULT_HTTP_CLIENT_FACTORY;
use codex_http_client::HttpClientFactory;
use codex_protocol::error::Result as CoreResult;
use codex_protocol::openai_models::ModelInfo;
use pretty_assertions::assert_eq;

use crate::cache::ModelsCache;
use crate::cache::ModelsCacheEntry;
use crate::cache::ModelsCacheError;
use crate::cache::ModelsCacheFuture;
use crate::manager::ModelsEndpointClient;
use crate::manager::ModelsEndpointFuture;
use crate::manager::ModelsEndpointResponse;
use crate::manager::ModelsManager;
use crate::manager::OpenAiModelsManager;
use crate::manager::RefreshStrategy;

#[derive(Debug)]
struct Endpoint {
    owner_current: AtomicBool,
    models: Vec<ModelInfo>,
}

impl Endpoint {
    fn new() -> Arc<Self> {
        Arc::new(Self { owner_current: AtomicBool::new(true),
            models: vec![crate::bundled_models_response().unwrap().models.remove(0)] })
    }
}

impl ModelsEndpointClient for Endpoint {
    // The worker's local snapshot remains unchanged while live host ownership changes.
    fn identity(&self) -> Option<String> { Some("captured-owner".to_owned()) }
    fn has_command_auth(&self) -> bool { true }
    fn uses_codex_backend(&self) -> ModelsEndpointFuture<'_, bool> { Box::pin(async { true }) }
    fn validate_identity<'a>(&'a self, identity: Option<&'a str>) -> ModelsEndpointFuture<'a, bool> {
        Box::pin(async move { self.owner_current.load(Ordering::SeqCst) && identity == Some("captured-owner") })
    }
    fn list_models<'a>(&'a self, _: &'a str, _: HttpClientFactory) -> ModelsEndpointFuture<'a, CoreResult<ModelsEndpointResponse>> {
        Box::pin(async move { Ok(ModelsEndpointResponse { models: self.models.clone(),
            etag: Some("etag".to_owned()), identity: "captured-owner".to_owned() }) })
    }
}

#[derive(Clone, Copy, Debug)]
enum RevokeAt { Never, Load, Store }

#[derive(Debug)]
struct Cache {
    endpoint: Arc<Endpoint>,
    entry: Mutex<Option<ModelsCacheEntry>>,
    revoke_at: RevokeAt,
    renewals: AtomicUsize,
}

impl Cache {
    fn new(endpoint: Arc<Endpoint>, revoke_at: RevokeAt) -> Arc<Self> {
        Arc::new(Self { endpoint, entry: Mutex::new(None), revoke_at, renewals: AtomicUsize::new(0) })
    }
}

impl ModelsCache for Cache {
    fn load<'a>(&'a self, _: &'a str) -> ModelsCacheFuture<'a, Result<Option<ModelsCacheEntry>, ModelsCacheError>> {
        Box::pin(async move {
            if matches!(self.revoke_at, RevokeAt::Load) { self.endpoint.owner_current.store(false, Ordering::SeqCst); }
            Ok(self.entry.lock().unwrap().clone())
        })
    }
    fn store<'a>(&'a self, entry: &'a ModelsCacheEntry) -> ModelsCacheFuture<'a, Result<(), ModelsCacheError>> {
        Box::pin(async move {
            *self.entry.lock().unwrap() = Some(entry.clone());
            if matches!(self.revoke_at, RevokeAt::Store) { self.endpoint.owner_current.store(false, Ordering::SeqCst); }
            Ok(())
        })
    }
    fn refresh_ttl<'a>(&'a self, _: &'a str, _: &'a str, _: &'a str) -> ModelsCacheFuture<'a, Result<(), ModelsCacheError>> {
        Box::pin(async move { self.renewals.fetch_add(1, Ordering::SeqCst); Ok(()) })
    }
}

#[tokio::test]
async fn host_owner_change_during_cache_write_prevents_native_publication() {
    let endpoint = Endpoint::new();
    let cache = Cache::new(Arc::clone(&endpoint), RevokeAt::Store);
    let manager = OpenAiModelsManager::new_with_cache(cache, endpoint, None).with_provider_catalog();
    let catalog = manager.raw_model_catalog(RefreshStrategy::Online, DEFAULT_HTTP_CLIENT_FACTORY).await;
    assert_eq!(catalog.models, Vec::new());
}

#[tokio::test]
async fn host_owner_change_during_cache_read_prevents_native_publication() {
    let endpoint = Endpoint::new();
    let cache = Cache::new(Arc::clone(&endpoint), RevokeAt::Load);
    *cache.entry.lock().unwrap() = Some(ModelsCacheEntry { fetched_at: Utc::now(),
        etag: Some("etag".to_owned()), client_version: Some(crate::client_version_to_whole()),
        identity: endpoint.identity(), models: endpoint.models.clone() });
    let manager = OpenAiModelsManager::new_with_cache(cache, endpoint, None).with_provider_catalog();
    let catalog = manager.raw_model_catalog(RefreshStrategy::Offline, DEFAULT_HTTP_CLIENT_FACTORY).await;
    assert_eq!(catalog.models, Vec::new());
}

#[tokio::test]
async fn matching_worker_etag_cannot_renew_cache_after_host_owner_change() {
    let endpoint = Endpoint::new();
    let cache = Cache::new(Arc::clone(&endpoint), RevokeAt::Never);
    let manager = OpenAiModelsManager::new_with_cache(cache.clone(), endpoint.clone(), None).with_provider_catalog();
    manager.raw_model_catalog(RefreshStrategy::Online, DEFAULT_HTTP_CLIENT_FACTORY).await;
    endpoint.owner_current.store(false, Ordering::SeqCst);
    manager.refresh_if_new_etag("etag".to_owned(), DEFAULT_HTTP_CLIENT_FACTORY).await;
    assert_eq!(cache.renewals.load(Ordering::SeqCst), 0);
}
