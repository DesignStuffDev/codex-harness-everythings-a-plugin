//! Real retained endpoint coverage for stale native credential-cache publication.
//!
//! The HTTP factory is deliberately unbound to account A. This exercises the
//! cache overwrite race, not revocation of an account-bound caller transport.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_login::ExternalAuth;
use codex_login::ExternalAuthFuture;
use codex_login::ExternalAuthRefreshContext;
use pretty_assertions::assert_eq;
use tokio::sync::Notify;
use tokio::sync::Semaphore;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;
use wiremock::matchers::query_param;

use super::FixedAuth;
use super::catalog;
use super::client;
use super::create_model_provider;
use super::provider_info;

#[derive(Debug)]
struct HeldCatalogAuth {
    calls: AtomicUsize,
    entered: Notify,
    release: Semaphore,
}

impl ExternalAuth for HeldCatalogAuth {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async move {
            let call = self.calls.fetch_add(/*val*/ 1, Ordering::SeqCst);
            if call == 0 {
                return Ok(CodexAuth::from_api_key("catalog-owner-a"));
            }
            assert_eq!(call, 1, "only one old endpoint acquisition is expected");
            self.entered.notify_one();
            self.release.acquire().await.unwrap().forget();
            Ok(CodexAuth::from_api_key("catalog-stale-a2"))
        })
    }

    fn refresh(&self, _context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, CodexAuth> {
        self.resolve()
    }
}

#[derive(Clone, Copy)]
enum Replacement {
    NewCredential,
    CredentialAba,
}

async fn retained_endpoint_rejects_stale_cache_load(replacement: Replacement) {
    let (version, slug, final_key, replacement_resolves) = match replacement {
        Replacement::NewCredential => (
            "cache-revision-replacement",
            "catalog-from-new-owner-b",
            "catalog-owner-b",
            1,
        ),
        Replacement::CredentialAba => (
            "cache-revision-aba",
            "catalog-after-owner-a-b-a",
            "catalog-owner-a",
            2,
        ),
    };
    let server = MockServer::start().await;
    let expected = catalog(slug);
    Mock::given(method("GET"))
        .and(path("/catalog"))
        .and(query_param("client_version", version))
        .and(header("Authorization", format!("Bearer {final_key}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&expected))
        .expect(1)
        .mount(&server)
        .await;

    let manager = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("catalog-owner-a"));
    let held = Arc::new(HeldCatalogAuth {
        calls: AtomicUsize::new(/*v*/ 0),
        entered: Notify::new(),
        release: Semaphore::new(/*permits*/ 0),
    });
    manager.set_external_auth(held.clone()).await.unwrap();
    assert_eq!(held.calls.load(Ordering::SeqCst), 1);
    let provider = create_model_provider(provider_info(&server), Some(manager.clone()));
    assert!(Arc::ptr_eq(&manager, &provider.auth_manager().unwrap()));
    let endpoint = provider.models_endpoint().unwrap();
    let initial_identity = endpoint.identity().unwrap();
    assert!(Arc::ptr_eq(&endpoint, &provider.models_endpoint().unwrap()));
    assert_eq!(held.calls.load(Ordering::SeqCst), 1);
    let resolves = Arc::new(AtomicUsize::new(/*v*/ 0));

    // Joining both branches retains and cancels them together on timeout/panic;
    // there is no spawned endpoint task that can outlive a failed assertion.
    let (result, final_identity) = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
        tokio::join!(endpoint.list_models(version, client()), async {
            held.entered.notified().await;
            assert_eq!(held.calls.load(Ordering::SeqCst), 2);
            assert_eq!(
                manager.auth_cached().unwrap().get_token().unwrap(),
                "catalog-owner-a"
            );
            assert_eq!(server.received_requests().await.unwrap().len(), 0);
            provider
                .auth_manager()
                .unwrap()
                .set_external_auth(Arc::new(FixedAuth {
                    key: "catalog-owner-b",
                    resolves: resolves.clone(),
                }))
                .await
                .unwrap();
            let changed_identity = endpoint.identity().unwrap();
            assert_ne!(changed_identity, initial_identity);
            if let Replacement::CredentialAba = replacement {
                provider
                    .auth_manager()
                    .unwrap()
                    .set_external_auth(Arc::new(FixedAuth {
                        key: "catalog-owner-a",
                        resolves: resolves.clone(),
                    }))
                    .await
                    .unwrap();
            }
            let current_identity = endpoint.identity().unwrap();
            match replacement {
                Replacement::NewCredential => assert_eq!(current_identity, changed_identity),
                Replacement::CredentialAba => assert_eq!(current_identity, initial_identity),
            }
            assert_eq!(
                manager.auth_cached().unwrap().get_token().unwrap(),
                final_key
            );
            assert_eq!(server.received_requests().await.unwrap().len(), 0);
            held.release.add_permits(/*n*/ 1);
            current_identity
        })
    })
    .await
    .expect("retained endpoint acquisition and real HTTP request must finish");

    // Check the native cache before unwrapping HTTP: the baseline restores A2,
    // and this assertion identifies the actual stale-publication defect.
    assert_eq!(
        manager.auth_cached().unwrap().get_token().unwrap(),
        final_key
    );
    assert_eq!(held.calls.load(Ordering::SeqCst), 2);
    assert_eq!(resolves.load(Ordering::SeqCst), replacement_resolves);
    assert!(Arc::ptr_eq(&manager, &provider.auth_manager().unwrap()));
    assert!(Arc::ptr_eq(&endpoint, &provider.models_endpoint().unwrap()));
    assert_eq!(endpoint.identity(), Some(final_identity.clone()));
    let response = result.expect("the retained endpoint must use the current cached credential");
    assert_eq!(response.models, expected.models);
    assert_eq!(response.identity, final_identity);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers["authorization"].to_str().unwrap(),
        format!("Bearer {final_key}")
    );
    server.verify().await;
}

#[tokio::test]
async fn retained_endpoint_uses_new_credentials_after_old_external_load_finishes() {
    retained_endpoint_rejects_stale_cache_load(Replacement::NewCredential).await;
}

#[tokio::test]
async fn retained_endpoint_rejects_stale_external_load_after_credential_aba() {
    retained_endpoint_rejects_stale_cache_load(Replacement::CredentialAba).await;
}
