use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use codex_component_host::DependencyOwner;
use codex_component_host::HostDependencyAuthority;
use codex_http_client::DestinationPolicy;
use codex_http_client::NetworkPolicyController;
use codex_http_client::OutboundProxyPolicy;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthKeyringBackendKind;
use codex_login::AuthRouteConfig;
use codex_login::CodexAuth;
use codex_login::ExternalAuth;
use codex_login::ExternalAuthFuture;
use codex_login::ExternalAuthRefreshContext;
use codex_model_catalog_api::CatalogError;
use codex_model_catalog_api::FetchModels;
use codex_model_catalog_api::FetchOutcome;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::manager::ModelsEndpointFuture;
use codex_models_manager::manager::ModelsEndpointResponse;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CoreResult;
use pretty_assertions::assert_eq;
use tokio::sync::Notify;
use tokio::sync::Semaphore;

use super::*;
use crate::models_endpoint::OpenAiModelsEndpoint;

struct FixedAuth(CodexAuth);

impl ExternalAuth for FixedAuth {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async { Ok(self.0.clone()) })
    }

    fn refresh(&self, _context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, CodexAuth> {
        self.resolve()
    }
}

fn chatgpt_auth(account: &str, signature: &str) -> CodexAuth {
    let claims = serde_json::json!({
        "email": "catalog-test@example.com",
        "https://api.openai.com/auth": { "chatgpt_user_id": "catalog-test-user" },
    });
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
    CodexAuth::from_external_chatgpt_tokens(&format!("e30.{payload}.{signature}"), account, Some("team"))
        .unwrap()
}

fn factory() -> HttpClientFactory {
    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault)
}

async fn request(authority: &NativeCatalogAuthority, factory: HttpClientFactory) -> CatalogEndpointContext {
    authority.snapshot(factory).await.unwrap().scope.request_context.unwrap()
        .downcast_ref::<CatalogEndpointContext>().unwrap().clone()
}

fn fetch_params() -> FetchModels {
    FetchModels { client_version: "1.2.3".to_owned() }
}

struct Endpoint {
    identity: Mutex<String>,
    response_identity: Mutex<Option<String>>,
    seen: Mutex<Vec<HttpClientFactory>>,
    delayed: AtomicBool,
    started: Notify,
    release: Semaphore,
    error: Mutex<Option<CodexErr>>,
}

impl fmt::Debug for Endpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TestCatalogEndpoint")
    }
}

impl Endpoint {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            identity: Mutex::new("initial-owner".to_owned()),
            response_identity: Mutex::new(None),
            seen: Mutex::new(Vec::new()),
            delayed: AtomicBool::new(false),
            started: Notify::new(),
            release: Semaphore::new(/*permits*/ 0),
            error: Mutex::new(None),
        })
    }
}

impl ModelsEndpointClient for Endpoint {
    fn identity(&self) -> Option<String> { Some(self.identity.lock().unwrap().clone()) }
    fn has_command_auth(&self) -> bool { false }
    fn supports_api_key_models(&self) -> bool { true }
    fn has_provider_api_key(&self) -> bool { true }
    fn uses_codex_backend(&self) -> ModelsEndpointFuture<'_, bool> { Box::pin(async { false }) }

    fn list_models<'a>(&'a self, _client_version: &'a str, factory: HttpClientFactory)
        -> ModelsEndpointFuture<'a, CoreResult<ModelsEndpointResponse>>
    {
        Box::pin(async move {
            self.seen.lock().unwrap().push(factory);
            let identity = self.response_identity.lock().unwrap().clone()
                .unwrap_or_else(|| self.identity.lock().unwrap().clone());
            self.started.notify_one();
            if self.delayed.load(Ordering::SeqCst) {
                self.release.acquire().await.unwrap().forget();
            }
            if let Some(error) = self.error.lock().unwrap().take() {
                return Err(error);
            }
            Ok(ModelsEndpointResponse { models: Vec::new(), etag: Some("test-etag".to_owned()), identity })
        })
    }
}

#[tokio::test]
async fn native_account_identity_preserves_token_rotation_and_fences_coalesced_owner_changes() {
    let manager = AuthManager::from_auth_for_testing(chatgpt_auth("account-a", "first-dummy-token"));
    let mut provider = ModelProviderInfo::create_openai_provider(Some("https://catalog.invalid/v1".to_owned()));
    // Avoid unrelated residency-header mutation in other native endpoint tests.
    provider.http_headers = Some(std::collections::HashMap::from([
        (codex_login::default_client::RESIDENCY_HEADER_NAME.to_owned(), "us".to_owned()),
    ]));
    let endpoint = Arc::new(OpenAiModelsEndpoint::new(provider, Some(manager.clone()), /*gateway_auth_manager*/ None));
    let authority = NativeCatalogAuthority::new(Some(endpoint), Some(manager.clone()), Duration::from_secs(5));
    let initial = request(&authority, factory()).await;
    let owner = DependencyOwner { generation: initial.auth.owner_generation, identity: initial.auth.identity.clone() };
    let revocation = HostDependencyAuthority::revoked(&authority, owner);
    tokio::pin!(revocation);
    manager.set_external_auth(Arc::new(FixedAuth(chatgpt_auth("account-a", "rotated-dummy-token")))).await.unwrap();
    let rotated = request(&authority, factory()).await;
    assert_eq!(initial.auth, rotated.auth);
    assert!(tokio::time::timeout(Duration::from_millis(20), &mut revocation).await.is_err());
    manager.set_external_auth(Arc::new(FixedAuth(chatgpt_auth("account-b", "other-dummy-token")))).await.unwrap();
    manager.set_external_auth(Arc::new(FixedAuth(chatgpt_auth("account-a", "returned-dummy-token")))).await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), &mut revocation).await.expect("account change revokes the scope");
    assert!(!authority.is_current(&initial.auth));
    let returned = request(&authority, factory()).await;
    assert_eq!(initial.auth.identity, returned.auth.identity);
    assert!(returned.auth.owner_generation > initial.auth.owner_generation);
}

#[tokio::test]
async fn accepted_requests_keep_their_own_http_factories() {
    let endpoint = Endpoint::new();
    let authority = NativeCatalogAuthority::new(Some(endpoint.clone()), /*auth_manager*/ None, Duration::from_secs(5));
    authority.set_api_key_model_discovery_enabled(true);
    let first_factory = factory();
    let second_factory = HttpClientFactory::new(OutboundProxyPolicy::RespectSystemProxy);
    let first = request(&authority, first_factory.clone()).await;
    let second = request(&authority, second_factory.clone()).await;
    let (first_result, second_result) = tokio::join!(
        authority.fetch(&first, fetch_params()), authority.fetch(&second, fetch_params()),
    );
    let expected = FetchOutcome::Fetched { models: Vec::new(), etag: Some("test-etag".to_owned()), identity: "initial-owner".to_owned() };
    assert_eq!((first_result.unwrap(), second_result.unwrap()), (expected.clone(), expected));
    assert_eq!(*endpoint.seen.lock().unwrap(), vec![first_factory, second_factory]);
}

#[tokio::test]
async fn discovery_policy_blocks_the_host_grant_and_fences_unsampled_policy_cycles() {
    let endpoint = Endpoint::new();
    let authority = NativeCatalogAuthority::new(Some(endpoint.clone()), /*auth_manager*/ None, Duration::from_secs(5));
    let disabled = request(&authority, factory()).await;
    assert_eq!(authority.fetch(&disabled, fetch_params()).await.unwrap(),
        FetchOutcome::Failed { error: CatalogError::InvalidRequest });
    assert!(endpoint.seen.lock().unwrap().is_empty());
    authority.set_api_key_model_discovery_enabled(true);
    let enabled = request(&authority, factory()).await;
    authority.set_api_key_model_discovery_enabled(false);
    authority.set_api_key_model_discovery_enabled(true);
    assert!(!authority.is_current(&enabled.auth));
}

#[tokio::test]
async fn owner_change_during_fetch_rejects_the_old_response() {
    let endpoint = Endpoint::new();
    endpoint.delayed.store(true, Ordering::SeqCst);
    let authority = NativeCatalogAuthority::new(Some(endpoint.clone()), /*auth_manager*/ None, Duration::from_secs(5));
    authority.set_api_key_model_discovery_enabled(true);
    let accepted = request(&authority, factory()).await;
    let (result, ()) = tokio::join!(authority.fetch(&accepted, fetch_params()), async {
        endpoint.started.notified().await;
        *endpoint.identity.lock().unwrap() = "replacement-owner".to_owned();
        endpoint.release.add_permits(1);
    });
    assert_eq!(result.unwrap_err(), DependencyError::new(DependencyErrorCode::OwnerChanged));
}

#[tokio::test]
async fn request_credentials_must_match_the_accepted_identity_and_errors_are_sanitized() {
    let endpoint = Endpoint::new();
    let authority = NativeCatalogAuthority::new(Some(endpoint.clone()), /*auth_manager*/ None, Duration::from_secs(5));
    authority.set_api_key_model_discovery_enabled(true);
    let accepted = request(&authority, factory()).await;
    *endpoint.response_identity.lock().unwrap() = Some("other-request-owner".to_owned());
    assert_eq!(authority.fetch(&accepted, fetch_params()).await.unwrap_err(),
        DependencyError::new(DependencyErrorCode::OwnerChanged));
    *endpoint.error.lock().unwrap() = Some(CodexErr::InvalidRequest("dummy-secret-provider-body".to_owned()));
    let outcome = authority.fetch(&accepted, fetch_params()).await.unwrap();
    assert_eq!(serde_json::to_value(outcome).unwrap(), serde_json::json!({
        "status": "failed", "error": "invalid_request",
    }));
}

#[tokio::test]
async fn revocation_wakes_waiters_and_static_mode_has_no_http_endpoint() {
    let authority = NativeCatalogAuthority::new(/*endpoint*/ None, /*auth_manager*/ None, Duration::from_secs(5));
    let accepted = request(&authority, factory()).await;
    assert_eq!(authority.fetch(&accepted, fetch_params()).await.unwrap_err(),
        DependencyError::new(DependencyErrorCode::ServiceUnavailable));
    let owner = DependencyOwner { generation: accepted.auth.owner_generation, identity: accepted.auth.identity.clone() };
    tokio::time::timeout(Duration::from_secs(1), async {
        tokio::join!(HostDependencyAuthority::revoked(&authority, owner), async {
            tokio::task::yield_now().await;
            authority.revoke();
        });
    }).await.expect("explicit close revokes the scope");
    assert!(!authority.is_current(&accepted.auth));
    assert!(authority.snapshot(factory()).await.is_err());
}

#[tokio::test]
async fn application_policy_publication_invalidates_a_catalog_snapshot() {
    let home = tempfile::tempdir().unwrap();
    let controller = NetworkPolicyController::default();
    let policy = controller.policy();
    assert!(controller.publish(policy.revision(), DestinationPolicy::Unrestricted));
    let manager = Arc::new(AuthManager::new(
        home.path().to_owned(), /*enable_codex_api_key_env*/ false,
        AuthCredentialsStoreMode::Ephemeral, /*forced_chatgpt_workspace_id*/ None,
        /*chatgpt_base_url*/ None, AuthKeyringBackendKind::Direct,
        AuthRouteConfig::from_http_client_factory(factory()).with_application_network_policy(policy.clone()),
    ).await);
    let authority = NativeCatalogAuthority::new(/*endpoint*/ None, Some(manager), Duration::from_secs(5));
    let accepted = request(&authority, factory()).await;
    assert!(controller.publish(policy.revision(), DestinationPolicy::Restricted { allowed_hosts: Default::default() }));
    assert!(!authority.is_current(&accepted.auth));
}

#[tokio::test]
async fn native_workspace_restriction_changes_invalidate_cached_catalogs() {
    let manager = AuthManager::from_auth_for_testing(chatgpt_auth("account-a", "policy-dummy-token"));
    let authority = NativeCatalogAuthority::new(/*endpoint*/ None, Some(manager.clone()), Duration::from_secs(5));
    let accepted = request(&authority, factory()).await;
    manager.set_forced_chatgpt_workspace_id(Some(vec!["account-a".to_owned()]));
    assert!(!authority.is_current(&accepted.auth));
}
