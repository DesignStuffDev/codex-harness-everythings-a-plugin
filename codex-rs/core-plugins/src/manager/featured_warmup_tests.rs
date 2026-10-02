use super::*;
use crate::FeaturedWarmupScope;
use crate::featured_warmup::FeaturedWarmupObservation;
use crate::featured_warmup::FeaturedWarmupOutcome;
use crate::featured_warmup::FeaturedWarmupOwnership;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn featured_warmup_close_preserves_same_home_replacement_and_foreground_cache() {
    let home = tempfile::tempdir().unwrap();
    write_file(
        &home.path().join(CONFIG_TOML_FILE),
        "[features]\nplugins = true\n",
    );
    let server = MockServer::start().await;
    let entered = Arc::new(tokio::sync::Notify::new());
    let request_entered = Arc::clone(&entered);
    let (release, released) = std::sync::mpsc::channel();
    let released = std::sync::Mutex::new(released);
    Mock::given(method("GET"))
        .and(path("/plugins/featured"))
        .and(query_param("platform", "codex"))
        .respond_with(move |_: &wiremock::Request| {
            request_entered.notify_one();
            // As in the existing recommended-cache fixture, wiremock owns its
            // server thread. Sender drop also releases this held response.
            let _ = released.lock().unwrap().recv();
            ResponseTemplate::new(200).set_body_json(vec!["from-closed-a"])
        })
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/plugins/featured"))
        .and(query_param("platform", "codex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(vec!["from-b"]))
        .expect(1)
        .mount(&server)
        .await;
    let mut config = load_config(home.path(), home.path()).await;
    config.chatgpt_base_url = server.uri();
    let a = Arc::new(test_plugins_manager(home.path().to_path_buf()));
    let b = Arc::new(test_plugins_manager(home.path().to_path_buf()));
    let key = featured_plugin_ids_cache_key(&config, None);
    let a_scope = FeaturedWarmupScope::new();
    a.start_featured_warmup_for_config(&config, &a_scope);
    tokio::time::timeout(Duration::from_secs(10), entered.notified())
        .await
        .unwrap();
    a_scope.begin_close();
    assert_eq!(
        a_scope
            .wait_until(Instant::now() + Duration::from_secs(10))
            .await,
        FeaturedWarmupObservation {
            ownership: FeaturedWarmupOwnership::Joined,
            outcome: Some(FeaturedWarmupOutcome::Cancelled)
        }
    );
    assert_eq!(a.cached_featured_plugin_ids(&key), None);
    release.send(()).unwrap();
    // Closed A must not regain admission or publish into either manager.
    a.start_featured_warmup_for_config(&config, &a_scope);
    let b_scope = FeaturedWarmupScope::new();
    b.start_featured_warmup_for_config(&config, &b_scope);
    let cached = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(ids) = b.cached_featured_plugin_ids(&key) {
                break ids;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(cached, vec!["from-b".to_string()]);
    assert_eq!(
        b_scope.wait().await,
        FeaturedWarmupObservation {
            ownership: FeaturedWarmupOwnership::Joined,
            outcome: Some(FeaturedWarmupOutcome::Succeeded)
        }
    );
    assert_eq!(a.cached_featured_plugin_ids(&key), None);
    assert_eq!(
        b.featured_plugin_ids_for_config(&config, None)
            .await
            .unwrap(),
        cached
    );
    server.verify().await;
}
