//! Atomic catalog publication and synchronous reads, guarded by live ownership.

use anyhow::Result;
use anyhow::ensure;
use codex_model_catalog_api::CONTRACT_VERSION;
use codex_model_catalog_api::CatalogAuthState;
use codex_model_catalog_api::CatalogError;
use codex_model_catalog_api::CatalogOutcome;
use codex_model_catalog_api::CatalogSnapshot;
use codex_model_catalog_api::CatalogSource;
use codex_protocol::error::Result as CoreResult;
use codex_protocol::openai_models::ModelInfo;
use tokio::sync::RwLock;
use tokio::sync::TryLockError;

use super::CatalogAuthority;
use super::unavailable;

struct Published {
    auth: CatalogAuthState,
    snapshot: CatalogSnapshot,
    failure: Option<CatalogError>,
}

#[derive(Default)]
pub(super) struct CatalogMirror {
    state: RwLock<Option<Published>>,
}

impl CatalogMirror {
    pub async fn publish(&self, source: &CatalogSource, auth: CatalogAuthState,
        outcome: CatalogOutcome, authority: &dyn CatalogAuthority) -> Result<()>
    {
        let (mut snapshot, failure) = match outcome {
            CatalogOutcome::Ready { snapshot } => (snapshot, None),
            CatalogOutcome::Failed { error, snapshot } => {
                // Endpoint failure follows native source policy. A broken
                // component/IPC path never reaches this domain envelope.
                if matches!(source, CatalogSource::Authoritative) {
                    ensure!(snapshot.models.is_empty() && snapshot.etag.is_none(),
                        "failed authoritative catalog retained stale data");
                }
                (snapshot, (error == CatalogError::OwnerChanged).then_some(error))
            }
        };
        ensure!(snapshot.contract_version == CONTRACT_VERSION, "incompatible catalog snapshot");
        ensure!(snapshot.owner_generation == auth.owner_generation && snapshot.identity == auth.identity,
            "catalog snapshot owner mismatch");
        ensure!(authority.is_current(&auth), "catalog owner changed before publication");
        let mut state = self.state.write().await;
        ensure!(authority.is_current(&auth), "catalog owner changed while awaiting publication");
        if let Some(previous) = state.as_ref() {
            ensure!(snapshot.revision > previous.snapshot.revision, "catalog revision did not advance");
        }
        if let CatalogSource::Static { models } = source
            && failure.is_none()
        {
            ensure!(snapshot.models == *models, "component changed an authoritative static catalog");
        }
        if failure.is_some() {
            snapshot.models.clear();
        }
        *state = Some(Published { auth, snapshot, failure });
        let published = state.as_mut().expect("snapshot was just published");
        if !authority.is_current(&published.auth) {
            published.snapshot.models.clear();
            published.failure = Some(CatalogError::OwnerChanged);
            anyhow::bail!("catalog owner changed during publication");
        }
        Ok(())
    }

    pub async fn fail_transport(&self) {
        if let Some(state) = self.state.write().await.as_mut() {
            state.snapshot.models.clear();
            state.failure = Some(CatalogError::Unavailable);
        }
    }

    pub async fn check_health(&self, authority: &dyn CatalogAuthority) -> CoreResult<()> {
        let state = self.state.read().await;
        let Some(state) = state.as_ref() else { return Err(unavailable()); };
        if state.failure.is_some() || !authority.is_current(&state.auth) {
            return Err(unavailable());
        }
        Ok(())
    }

    pub async fn models(&self, authority: &dyn CatalogAuthority) -> Vec<ModelInfo> {
        Self::read_models(self.state.read().await.as_ref(), authority)
    }

    pub fn try_models(&self, authority: &dyn CatalogAuthority) -> Result<Vec<ModelInfo>, TryLockError> {
        Ok(Self::read_models(self.state.try_read()?.as_ref(), authority))
    }

    pub fn uses_codex_backend(&self, authority: &dyn CatalogAuthority) -> bool {
        self.state.try_read().ok().and_then(|state| state.as_ref().map(|state|
            state.failure.is_none() && authority.is_current(&state.auth) && state.auth.uses_codex_backend))
            .unwrap_or(false)
    }

    fn read_models(state: Option<&Published>, authority: &dyn CatalogAuthority) -> Vec<ModelInfo> {
        let Some(state) = state else { return Vec::new(); };
        if state.failure.is_some() || !authority.is_current(&state.auth) { return Vec::new(); }
        let models = state.snapshot.models.clone();
        if authority.is_current(&state.auth) { models } else { Vec::new() }
    }
}

#[cfg(test)]
#[path = "component_mirror_tests.rs"]
mod tests;
