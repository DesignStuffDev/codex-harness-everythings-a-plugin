//! Own only featured startup warmup; foreground reads retain their existing API.

use super::PluginsConfigInput;
use super::PluginsManager;
use super::featured_plugin_ids_cache_key;
use super::remote_plugin_service_config;
use crate::FeaturedWarmupScope;
use crate::featured_warmup::FeaturedWarmupAdmission;
use crate::featured_warmup::FeaturedWarmupOutcome;
use std::sync::Arc;

impl PluginsManager {
    pub(super) fn start_featured_warmup_for_config(
        self: &Arc<Self>,
        config: &PluginsConfigInput,
        scope: &Arc<FeaturedWarmupScope>,
    ) {
        let config = config.clone();
        let manager = Arc::clone(self);
        let publication = Arc::clone(scope);
        let admission = scope.start(async move {
            let auth = manager.auth_manager.auth().await;
            let key = featured_plugin_ids_cache_key(&config, auth.as_ref());
            if manager.cached_featured_plugin_ids(&key).is_some() {
                return FeaturedWarmupOutcome::Succeeded;
            }
            let ids = match crate::remote_legacy::fetch_remote_featured_plugin_ids(
                &remote_plugin_service_config(&config),
                auth.as_ref(),
                manager.restriction_product,
            )
            .await
            {
                Ok(ids) => ids,
                Err(err) => {
                    tracing::warn!(error = %err, "failed to warm featured plugin ids cache");
                    return FeaturedWarmupOutcome::Failed;
                }
            };
            // Lock order is scope state, then manager cache. No await or inverse
            // path is permitted here. Cache-clear epochs remain separate work.
            match publication.publish_if_open(|| manager.write_featured_plugin_ids_cache(key, &ids))
            {
                Some(()) => FeaturedWarmupOutcome::Succeeded,
                None => FeaturedWarmupOutcome::Cancelled,
            }
        });
        match admission {
            FeaturedWarmupAdmission::Started | FeaturedWarmupAdmission::Closed => {}
            FeaturedWarmupAdmission::AlreadyStarted
            | FeaturedWarmupAdmission::CapacityExceeded
            | FeaturedWarmupAdmission::SpawnFailed => {
                tracing::warn!(?admission, "featured warmup admission rejected");
            }
        }
    }
}
