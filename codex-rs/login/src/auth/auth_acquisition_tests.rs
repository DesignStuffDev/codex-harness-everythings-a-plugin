use super::Reply;
use super::fixture;
use super::header_auth;
use crate::auth::AuthHeaders;
use crate::auth::AuthManager;
use crate::auth::CodexAuth;
use crate::outbound_proxy::AuthRouteConfig;
use codex_http_client::DestinationPolicy;
use codex_http_client::HttpClientFactory;
use codex_http_client::NetworkPolicyController;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

async fn acquire(
    manager: &AuthManager,
    with_factory: bool,
) -> Option<(CodexAuth, Option<HttpClientFactory>)> {
    if with_factory {
        manager
            .auth_with_http_client_factory()
            .await
            .map(|(auth, factory)| (auth, Some(factory)))
    } else {
        manager.auth().await.map(|auth| (auth, None))
    }
}

fn other_account_auth() -> CodexAuth {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        "authorization",
        http::HeaderValue::from_static("Bearer newer"),
    );
    headers.insert(
        "chatgpt-account-id",
        http::HeaderValue::from_static("account-b"),
    );
    CodexAuth::Headers(AuthHeaders::new(headers))
}

#[tokio::test]
async fn held_source_failures_cannot_return_policy_rejected_cached_credentials() {
    for with_factory in [false, true] {
        for revoke_while_held in [false, true] {
            for permanent in [false, true] {
                let expected = header_auth("Bearer retained");
                let reply = if permanent {
                    Reply::Permanent
                } else {
                    Reply::Transient
                };
                let (manager, external) = fixture(expected.clone(), reply).await;
                if !revoke_while_held {
                    manager
                        .set_forced_chatgpt_workspace_id(Some(vec!["account-b".into()]))
                        .unwrap();
                }
                let changes = manager.auth_change_state_receiver();
                let task_manager = manager.clone();
                let task = tokio::spawn(async move { acquire(&task_manager, with_factory).await });
                tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
                    .await
                    .unwrap();
                if revoke_while_held {
                    manager
                        .set_forced_chatgpt_workspace_id(Some(vec!["account-b".into()]))
                        .unwrap();
                }
                external.release.add_permits(/*n*/ 1);
                let result = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
                    .await
                    .unwrap()
                    .unwrap();
                assert!(
                    result.is_none(),
                    "with_factory={with_factory}, revoke_while_held={revoke_while_held}, permanent={permanent}"
                );
                assert_eq!(manager.auth_cached(), Some(expected));
                assert!(!changes.has_changed().unwrap());
                assert_eq!(external.calls.load(Ordering::SeqCst), 2);
            }
        }
    }
}

#[tokio::test]
async fn held_source_failures_preserve_valid_current_credentials() {
    for with_factory in [false, true] {
        for permanent in [false, true] {
            let expected = header_auth("Bearer retained");
            let reply = if permanent {
                Reply::Permanent
            } else {
                Reply::Transient
            };
            let (manager, external) = fixture(expected.clone(), reply).await;
            manager
                .set_forced_chatgpt_workspace_id(Some(vec!["account-a".into()]))
                .unwrap();
            let task_manager = manager.clone();
            let task = tokio::spawn(async move { acquire(&task_manager, with_factory).await });
            tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
                .await
                .unwrap();
            external.release.add_permits(/*n*/ 1);
            let (auth, factory) = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(auth, expected);
            assert_eq!(factory.is_some(), with_factory);
            assert_eq!(manager.auth_cached(), Some(expected));
        }
    }
}

#[tokio::test]
async fn held_failure_acquires_newer_allowed_cache_and_its_current_factory() {
    for with_factory in [false, true] {
        let (mut manager, external) = fixture(header_auth("Bearer old"), Reply::Transient).await;
        let controller = NetworkPolicyController::default();
        let policy = controller.policy();
        let base = manager
            .http_client_factory()
            .with_network_policy(policy.clone());
        Arc::get_mut(&mut manager).unwrap().auth_route_config =
            AuthRouteConfig::from_http_client_factory(base);
        assert!(controller.publish(policy.revision(), DestinationPolicy::Unrestricted));
        let old_factory = manager.http_client_factory();
        let task_manager = manager.clone();
        let task = tokio::spawn(async move { acquire(&task_manager, with_factory).await });
        tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
            .await
            .unwrap();
        let expected = other_account_auth();
        manager.set_cached_auth(Some(expected.clone()));
        manager
            .set_forced_chatgpt_workspace_id(Some(vec!["account-b".into()]))
            .unwrap();
        assert!(controller.publish(policy.revision(), DestinationPolicy::Unrestricted));
        external.release.add_permits(/*n*/ 1);
        let (auth, factory) = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(auth, expected);
        assert_eq!(manager.auth_cached(), Some(expected));
        let destination = "https://example.com/".parse().unwrap();
        assert!(old_factory.network_policy().acquire(&destination).is_err());
        if let Some(factory) = factory {
            assert_eq!(
                factory.network_policy(),
                manager.http_client_factory().network_policy()
            );
            assert!(factory.network_policy().acquire(&destination).is_ok());
        } else {
            assert!(!with_factory);
        }
    }
}

#[tokio::test]
async fn native_cached_acquisition_denies_restrictions_without_mutating_cache() {
    for with_factory in [false, true] {
        let expected = header_auth("Bearer retained");
        let manager = AuthManager::from_optional_auth_for_testing(Some(expected.clone()));
        manager
            .set_forced_chatgpt_workspace_id(Some(vec!["account-b".into()]))
            .unwrap();
        assert!(acquire(&manager, with_factory).await.is_none());
        assert_eq!(manager.auth_cached(), Some(expected));
    }
}
