use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use codex_http_client::HttpClientFactory;
use codex_model_catalog_api::CatalogAuthState;
use codex_model_catalog_api::CatalogError;
use codex_model_catalog_api::CatalogOutcome;
use codex_model_catalog_api::CatalogSnapshot;
use codex_model_catalog_api::CatalogSource;
use codex_protocol::error::Result as CoreResult;
use pretty_assertions::assert_eq;

use super::CatalogMirror;
use super::super::CatalogAuthority;
use super::super::CatalogRequestAuthority;
use crate::manager::ModelsManagerFuture;

#[derive(Debug)]
struct Authority {
    owner: AtomicU64,
    checks: AtomicUsize,
    revoke_on_check: usize,
}

impl Authority {
    fn new(revoke_on_check: usize) -> Self {
        Self { owner: AtomicU64::new(1), checks: AtomicUsize::new(0), revoke_on_check }
    }
}

impl CatalogAuthority for Authority {
    fn snapshot(&self, _: HttpClientFactory) -> ModelsManagerFuture<'_, CoreResult<CatalogRequestAuthority>> {
        Box::pin(async { unreachable!("mirror tests receive an already captured scope") })
    }
    fn is_current(&self, auth: &CatalogAuthState) -> bool {
        let check = self.checks.fetch_add(1, Ordering::SeqCst) + 1;
        if check == self.revoke_on_check { self.owner.store(2, Ordering::SeqCst); }
        auth.owner_generation == self.owner.load(Ordering::SeqCst)
    }
    fn set_api_key_model_discovery_enabled(&self, _: bool) {}
    fn revoke(&self) { self.owner.fetch_add(1, Ordering::SeqCst); }
}

fn auth() -> CatalogAuthState {
    CatalogAuthState { owner_generation: 1, identity: Some("opaque-test-owner".to_owned()),
        auth_mode: None, uses_codex_backend: true, has_command_auth: false,
        supports_api_key_models: true, has_provider_api_key: false, api_key_discovery_enabled: false }
}

fn snapshot(revision: u64) -> CatalogSnapshot {
    let model = crate::bundled_models_response().expect("bundled catalog").models.remove(0);
    CatalogSnapshot { contract_version: 1, owner_generation: 1, identity: auth().identity,
        revision, etag: Some("test-etag".to_owned()), models: vec![model] }
}

#[tokio::test]
async fn account_switch_invalidates_both_sync_and_async_reads() {
    let mirror = CatalogMirror::default();
    let authority = Authority::new(usize::MAX);
    let snapshot = snapshot(1);
    mirror.publish(&CatalogSource::Authoritative, auth(), CatalogOutcome::Ready { snapshot: snapshot.clone() }, &authority).await.unwrap();
    assert_eq!(mirror.try_models(&authority).unwrap(), snapshot.models);
    authority.owner.store(2, Ordering::SeqCst);
    assert_eq!(mirror.models(&authority).await, Vec::new());
    assert_eq!(mirror.try_models(&authority).unwrap(), Vec::new());
    assert!(mirror.check_health(&authority).await.is_err());
    assert!(!mirror.uses_codex_backend(&authority));
}

#[tokio::test]
async fn account_switch_during_publication_never_exposes_prior_owner() {
    let mirror = CatalogMirror::default();
    let authority = Authority::new(3);
    assert!(mirror.publish(&CatalogSource::BundledMerge, auth(),
        CatalogOutcome::Ready { snapshot: snapshot(1) }, &authority).await.is_err());
    assert_eq!(mirror.models(&authority).await, Vec::new());
    assert!(mirror.check_health(&authority).await.is_err());
}

#[tokio::test]
async fn native_fallback_requires_explicit_source_policy_and_healthy_transport() {
    for source in [CatalogSource::BundledMerge, CatalogSource::Authoritative] {
        let mirror = CatalogMirror::default();
        let authority = Authority::new(usize::MAX);
        let mut snapshot = snapshot(1);
        if matches!(source, CatalogSource::Authoritative) {
            snapshot.models.clear();
            snapshot.etag = None;
        }
        mirror.publish(&source, auth(), CatalogOutcome::Failed { error: CatalogError::Timeout, snapshot: snapshot.clone() }, &authority).await.unwrap();
        match source {
            CatalogSource::BundledMerge => {
                assert_eq!(mirror.models(&authority).await, snapshot.models);
                mirror.check_health(&authority).await.unwrap();
            }
            CatalogSource::Authoritative => {
                assert_eq!(mirror.models(&authority).await, Vec::new());
                mirror.check_health(&authority).await.unwrap();
            }
            CatalogSource::Static { .. } | CatalogSource::ProviderStatic { .. } => unreachable!(),
        }
        mirror.fail_transport().await;
        assert_eq!(mirror.models(&authority).await, Vec::new());
        assert!(mirror.check_health(&authority).await.is_err());
    }
}

#[tokio::test]
async fn replayed_revision_cannot_overwrite_a_newer_catalog() {
    let mirror = CatalogMirror::default();
    let authority = Authority::new(usize::MAX);
    let newest = snapshot(8);
    mirror.publish(&CatalogSource::Authoritative, auth(), CatalogOutcome::Ready { snapshot: newest.clone() }, &authority).await.unwrap();
    assert!(mirror.publish(&CatalogSource::Authoritative, auth(), CatalogOutcome::Ready { snapshot: snapshot(7) }, &authority).await.is_err());
    assert_eq!(mirror.models(&authority).await, newest.models);
}

#[tokio::test]
async fn provider_seed_is_replaceable_but_explicit_static_configuration_is_authoritative() {
    let authority = Authority::new(usize::MAX);
    let seed = snapshot(1).models;
    let mut replacement = snapshot(1);
    replacement.models[0].slug = "custom-component-model".to_owned();
    replacement.models[0].context_window = Some(32768);
    for source in [CatalogSource::Static { models: seed.clone() }, CatalogSource::ProviderStatic { models: seed.clone() }] {
        let mirror = CatalogMirror::default();
        let result = mirror.publish(&source, auth(), CatalogOutcome::Ready { snapshot: replacement.clone() }, &authority).await;
        match source {
            CatalogSource::Static { .. } => assert!(result.is_err()),
            CatalogSource::ProviderStatic { .. } => {
                result.unwrap();
                assert_eq!(mirror.models(&authority).await, replacement.models);
            }
            CatalogSource::BundledMerge | CatalogSource::Authoritative => unreachable!(),
        }
    }
}
