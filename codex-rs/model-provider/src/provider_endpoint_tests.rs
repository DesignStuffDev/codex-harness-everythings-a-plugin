//! Native provider capability ownership and actual endpoint/manager behavior.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use codex_http_client::HttpClientFactory;
use codex_http_client::NetworkPolicyController;
use codex_http_client::OutboundProxyPolicy;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_model_provider_info::GatewayOAuthConfig;
use codex_model_provider_info::GatewayOAuthDelivery;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::cache::ModelsCache;
use codex_models_manager::cache::ModelsCacheEntry;
use codex_models_manager::cache::ModelsCacheError;
use codex_models_manager::cache::ModelsCacheFuture;
use codex_models_manager::manager::RefreshStrategy;
use codex_models_manager::manager::SharedModelsManager;
use codex_protocol::openai_models::ModelsResponse;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;
use wiremock::matchers::query_param;

use super::ModelProvider;
use super::ModelProviderFuture;
use super::ProviderAccountResult;
use super::ProviderAccountState;
use super::create_model_provider;

fn catalog(slug: &str) -> ModelsResponse {
    let mut model = codex_models_manager::bundled_models_response()
        .unwrap()
        .models
        .remove(0);
    model.slug = slug.into();
    model.display_name = slug.into();
    model.used_fallback_model_metadata = false;
    ModelsResponse {
        models: vec![model],
    }
}

fn provider_info(server: &MockServer) -> ModelProviderInfo {
    ModelProviderInfo {
        model_catalog_url: Some(format!("{}/catalog", server.uri()).into()),
        // Keep identity stable beside the existing parallel residency test.
        http_headers: Some(HashMap::from([(
            codex_login::default_client::RESIDENCY_HEADER_NAME.into(),
            "us".into(),
        )])),
        ..ModelProviderInfo::create_openai_provider(Some(format!("{}/inference", server.uri())))
    }
}

fn client() -> HttpClientFactory {
    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault)
}

#[derive(Debug, Default)]
struct RecordingCache {
    calls: Mutex<Vec<&'static str>>,
    entry: Mutex<Option<ModelsCacheEntry>>,
}

impl ModelsCache for RecordingCache {
    fn load<'a>(
        &'a self,
        _version: &'a str,
    ) -> ModelsCacheFuture<'a, Result<Option<ModelsCacheEntry>, ModelsCacheError>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push("load");
            Ok(self.entry.lock().unwrap().clone())
        })
    }
    fn store<'a>(
        &'a self,
        entry: &'a ModelsCacheEntry,
    ) -> ModelsCacheFuture<'a, Result<(), ModelsCacheError>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push("store");
            *self.entry.lock().unwrap() = Some(entry.clone());
            Ok(())
        })
    }
    fn refresh_ttl<'a>(
        &'a self,
        _version: &'a str,
        _identity: &'a str,
        _etag: &'a str,
    ) -> ModelsCacheFuture<'a, Result<(), ModelsCacheError>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push("refresh_ttl");
            Ok(())
        })
    }
}

#[tokio::test]
async fn native_managers_retain_the_provider_endpoint_across_all_cache_modes() {
    let server = MockServer::start().await;
    let expected = catalog("owned-endpoint-remote");
    Mock::given(method("GET"))
        .and(path("/catalog"))
        .and(query_param(
            "client_version",
            codex_models_manager::client_version_to_whole(),
        ))
        .and(header("Authorization", "Bearer provider-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&expected))
        .expect(3)
        .mount(&server)
        .await;
    let provider = create_model_provider(
        provider_info(&server),
        Some(AuthManager::from_auth_for_testing(CodexAuth::from_api_key(
            "provider-key",
        ))),
    );
    let endpoint = provider.models_endpoint().unwrap();
    assert!(Arc::ptr_eq(&endpoint, &provider.models_endpoint().unwrap()));
    let baseline = Arc::strong_count(&endpoint);
    let home = tempfile::tempdir().unwrap();
    let cache = Arc::new(RecordingCache::default());
    let managers = [
        provider.models_manager(home.path().into(), /*config_model_catalog*/ None),
        provider.models_manager_without_cache(/*config_model_catalog*/ None),
        provider.models_manager_with_cache(/*config_model_catalog*/ None, cache.clone()),
    ];
    // Source review confirms these retained references are the managers' endpoint fields.
    assert_eq!(Arc::strong_count(&endpoint), baseline + managers.len());
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(!home.path().join("models_cache.json").exists());
    for manager in &managers {
        manager.set_api_key_model_discovery_enabled(/*enabled*/ true);
        assert_eq!(
            manager
                .raw_model_catalog(RefreshStrategy::Online, client())
                .await,
            expected
        );
    }
    assert!(home.path().join("models_cache.json").is_file());
    let cached = cache.entry.lock().unwrap().clone().unwrap();
    assert_eq!(cached.models, expected.models);
    assert_eq!(cached.identity, endpoint.identity());
    assert_eq!(*cache.calls.lock().unwrap(), vec!["store"]);
    drop(managers);
    assert_eq!(Arc::strong_count(&endpoint), baseline);
}

#[tokio::test]
async fn explicit_static_catalog_bypasses_endpoint_and_every_cache_mode() {
    let server = MockServer::start().await;
    let provider = create_model_provider(provider_info(&server), /*auth_manager*/ None);
    let endpoint = provider.models_endpoint().unwrap();
    let baseline = Arc::strong_count(&endpoint);
    let expected = catalog("authoritative-static-only");
    let home = tempfile::tempdir().unwrap();
    let cache = Arc::new(RecordingCache::default());
    let managers = [
        provider.models_manager(home.path().into(), Some(expected.clone())),
        provider.models_manager_without_cache(Some(expected.clone())),
        provider.models_manager_with_cache(Some(expected.clone()), cache.clone()),
    ];
    assert_eq!(Arc::strong_count(&endpoint), baseline);
    for manager in managers {
        manager.set_api_key_model_discovery_enabled(/*enabled*/ true);
        assert_eq!(
            manager
                .raw_model_catalog(RefreshStrategy::Online, client())
                .await,
            expected
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
    assert!(cache.calls.lock().unwrap().is_empty());
    assert!(cache.entry.lock().unwrap().is_none());
}

#[derive(Debug)]
struct FixedAuth {
    key: &'static str,
    resolves: Arc<AtomicUsize>,
}
impl codex_login::ExternalAuth for FixedAuth {
    fn resolve(&self) -> codex_login::ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async move {
            self.resolves.fetch_add(1, Ordering::SeqCst);
            Ok(CodexAuth::from_api_key(self.key))
        })
    }
    fn refresh(
        &self,
        _context: codex_login::ExternalAuthRefreshContext,
    ) -> codex_login::ExternalAuthFuture<'_, CodexAuth> {
        self.resolve()
    }
}

#[tokio::test]
async fn retained_capability_observes_live_auth_and_each_request_network_policy() {
    let server = MockServer::start().await;
    let expected = catalog("live-provider-owned");
    for (key, count) in [("initial-key", 2), ("replacement-key", 1)] {
        Mock::given(method("GET"))
            .and(path("/catalog"))
            .and(query_param("client_version", "capability-test"))
            .and(header("Authorization", format!("Bearer {key}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected))
            .expect(count)
            .mount(&server)
            .await;
    }
    let auth = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("initial-key"));
    let resolves = Arc::new(AtomicUsize::new(0));
    auth.set_external_auth(Arc::new(FixedAuth {
        key: "initial-key",
        resolves: resolves.clone(),
    }))
    .await
    .unwrap();
    let before_construction = resolves.load(Ordering::SeqCst);
    let provider = create_model_provider(provider_info(&server), Some(auth.clone()));
    let sibling = create_model_provider(
        provider_info(&server),
        Some(AuthManager::from_auth_for_testing(CodexAuth::from_api_key(
            "initial-key",
        ))),
    );
    assert!(Arc::ptr_eq(&provider.auth_manager().unwrap(), &auth));
    let endpoint = provider.models_endpoint().unwrap();
    let sibling_endpoint = sibling.models_endpoint().unwrap();
    assert!(!Arc::ptr_eq(&endpoint, &sibling_endpoint));
    let original_identity = endpoint.identity().unwrap();
    assert_eq!(resolves.load(Ordering::SeqCst), before_construction);
    let first = endpoint
        .list_models("capability-test", client())
        .await
        .unwrap();
    assert_eq!(first.models, expected.models);
    assert_eq!(first.identity, original_identity);

    let denied = NetworkPolicyController::default();
    assert!(
        endpoint
            .list_models(
                "capability-test",
                client().with_network_policy(denied.policy())
            )
            .await
            .is_err()
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    provider
        .auth_manager()
        .unwrap()
        .set_external_auth(Arc::new(FixedAuth {
            key: "replacement-key",
            resolves,
        }))
        .await
        .unwrap();
    let updated_identity = endpoint.identity().unwrap();
    assert_ne!(updated_identity, original_identity);
    let updated = endpoint
        .list_models("capability-test", client())
        .await
        .unwrap();
    assert_eq!(updated.models, expected.models);
    assert_eq!(updated.identity, updated_identity);
    assert_eq!(sibling_endpoint.identity(), Some(original_identity.clone()));
    let unaffected = sibling_endpoint
        .list_models("capability-test", client())
        .await
        .unwrap();
    assert_eq!(unaffected.models, expected.models);
    assert_eq!(unaffected.identity, original_identity);
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[derive(Debug)]
struct CustomStaticProvider(ModelProviderInfo);
impl ModelProvider for CustomStaticProvider {
    fn info(&self) -> &ModelProviderInfo {
        &self.0
    }
    fn auth_manager(&self) -> Option<Arc<AuthManager>> {
        None
    }
    fn auth(&self) -> ModelProviderFuture<'_, Option<CodexAuth>> {
        Box::pin(async { None })
    }
    fn account_state(&self) -> ProviderAccountResult {
        Ok(ProviderAccountState {
            account: None,
            requires_openai_auth: false,
        })
    }
    fn models_manager(
        &self,
        _home: PathBuf,
        catalog: Option<ModelsResponse>,
    ) -> SharedModelsManager {
        self.models_manager_without_cache(catalog)
    }
}

#[tokio::test]
async fn bedrock_and_custom_provider_keep_static_managers_without_endpoint_capability() {
    let bedrock = create_model_provider(
        ModelProviderInfo::create_amazon_bedrock_provider(/*aws*/ None),
        /*auth_manager*/ None,
    );
    let custom: Arc<dyn ModelProvider> =
        Arc::new(CustomStaticProvider(ModelProviderInfo::default()));
    let mut expected = catalog("static-without-endpoint");
    // An already Bedrock-compatible static fixture avoids testing its separate normalization.
    expected.models[0].additional_speed_tiers.clear();
    expected.models[0].service_tiers.clear();
    expected.models[0].default_service_tier = None;
    expected.models[0].web_search_tool_type =
        codex_protocol::openai_models::WebSearchToolType::Text;
    for provider in [bedrock, custom] {
        assert!(provider.models_endpoint().is_none());
        let manager = provider.models_manager_without_cache(Some(expected.clone()));
        assert_eq!(
            manager
                .raw_model_catalog(RefreshStrategy::Online, client())
                .await,
            expected
        );
    }
}

#[tokio::test]
async fn capability_and_native_manager_share_gateway_credentials_after_rotation() {
    let server = MockServer::start().await;
    let home = tempfile::tempdir().unwrap();
    let primary = AuthManager::from_auth_for_testing_with_home(
        CodexAuth::from_api_key("primary-key"),
        home.path().into(),
    );
    let mut info = provider_info(&server);
    info.gateway_oauth = Some(GatewayOAuthConfig {
        authorization_url: format!("{}/authorize", server.uri()),
        token_url: format!("{}/token", server.uri()),
        client_id: "client".into(),
        resource: None,
        scopes: vec![],
        redirect_port: None,
        delivery: GatewayOAuthDelivery::Header {
            name: "x-gateway-auth".into(),
            scheme: "Bearer".into(),
        },
    });
    let gateway = crate::test_support::seed_gateway_auth(
        &info,
        &primary,
        json!({"access_token":"first", "refresh_token":"refresh-fixture", "expires_at":i64::MAX}),
    );
    let provider = create_model_provider(info, Some(primary));
    assert!(Arc::ptr_eq(
        &gateway,
        &provider.gateway_auth_manager().unwrap().unwrap()
    ));
    let endpoint = provider.models_endpoint().unwrap();
    let expected = catalog("shared-gateway-model");
    for token in ["first", "second"] {
        Mock::given(method("GET"))
            .and(path("/catalog"))
            .and(header("Authorization", "Bearer primary-key"))
            .and(header("x-gateway-auth", format!("Bearer {token}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected))
            .expect(1)
            .mount(&server)
            .await;
    }
    assert_eq!(
        endpoint
            .list_models("capability-test", client())
            .await
            .unwrap()
            .models,
        expected.models
    );
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token":"second", "expires_in":3600})),
        )
        .expect(1)
        .mount(&server)
        .await;
    gateway.refresh_access_token("first").await.unwrap();
    let manager = provider.models_manager_without_cache(/*config_model_catalog*/ None);
    manager.set_api_key_model_discovery_enabled(/*enabled*/ true);
    assert_eq!(
        manager
            .raw_model_catalog(RefreshStrategy::Online, client())
            .await,
        expected
    );
}

#[path = "provider_endpoint_cache_revision_tests.rs"]
mod cache_revision_tests;
