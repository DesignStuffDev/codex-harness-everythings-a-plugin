//! Compare the installed process with the native backend at typed boundaries.

use codex_protocol::ThreadId;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::protocol::TokenUsage;
use codex_thread_store::ThreadMetadataPatch;
use codex_thread_store::ThreadStore;
use codex_thread_store_component::StorageResponse;
use pretty_assertions::assert_eq;

use crate::support::Fixture;

#[tokio::test]
async fn pending_metadata_preserves_budget_custom_variants_and_field_presence() {
    let fixture = Fixture::install();
    let process = fixture.open().await;
    let native = fixture.local().await;
    let patch = ThreadMetadataPatch {
        name: Some(None),
        agent_role: Some(Some("reviewer".to_owned())),
        // These variants intentionally have the same display spelling as a
        // built-in variant. Provider serde must not define internal identity.
        reasoning_effort: Some(Some(ReasoningEffort::Custom("high".to_owned()))),
        thread_source: Some(Some(ThreadSource::Feature("user".to_owned()))),
        token_usage: Some(TokenUsage {
            input_tokens: 11,
            cached_input_tokens: 3,
            cache_write_input_tokens: 2,
            output_tokens: 7,
            reasoning_output_tokens: 5,
            total_tokens: 18,
            codex_rollout_budget_units: Some(
                serde_json::from_str("123456789012345678901234567890.1234567890123456789").unwrap(),
            ),
        }),
        ..Default::default()
    };
    let thread_id = ThreadId::new();
    for store in [&native as &dyn ThreadStore, process.as_ref()] {
        store
            .stage_pending_thread_metadata(thread_id, patch.clone())
            .await
            .unwrap();
    }
    let expected = native
        .read_pending_thread_metadata(thread_id)
        .await
        .unwrap()
        .unwrap();
    let actual = process
        .read_pending_thread_metadata(thread_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(actual.token_usage, expected.token_usage);
    assert_eq!(actual.reasoning_effort, expected.reasoning_effort);
    assert_eq!(actual.thread_source, expected.thread_source);
    assert_eq!(
        serde_json::to_value(StorageResponse::ReadPendingThreadMetadata(Some(actual))).unwrap(),
        serde_json::to_value(StorageResponse::ReadPendingThreadMetadata(Some(expected))).unwrap()
    );
    for store in [&native as &dyn ThreadStore, process.as_ref()] {
        store
            .remove_pending_thread_metadata(thread_id)
            .await
            .unwrap();
        assert!(
            store
                .read_pending_thread_metadata(thread_id)
                .await
                .unwrap()
                .is_none()
        );
    }
    process.shutdown_process().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn native_non_utf8_queries_and_pending_paths_survive_the_process_boundary() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    use codex_protocol::protocol::ThreadHistoryMode;
    use codex_thread_store::AppendThreadItemsParams;
    use codex_thread_store::PersistContext;
    use codex_thread_store::ReadThreadByRolloutPathParams;

    let fixture = Fixture::install();
    let process = fixture.open().await;
    let native = fixture.local().await;
    let directory = fixture
        ._root
        .path()
        .join(OsString::from_vec(b"native-\xff".to_vec()));
    std::fs::create_dir(&directory).unwrap();
    let raw_path = directory.join("history.jsonl");
    let pending_id = ThreadId::new();
    let patch = ThreadMetadataPatch {
        cwd: Some(directory.clone()),
        ..Default::default()
    };
    for store in [&native as &dyn ThreadStore, process.as_ref()] {
        store
            .stage_pending_thread_metadata(pending_id, patch.clone())
            .await
            .unwrap();
        let pending = store
            .read_pending_thread_metadata(pending_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (pending.cwd, pending.rollout_path),
            (patch.cwd.clone(), patch.rollout_path.clone())
        );
        store
            .remove_pending_thread_metadata(pending_id)
            .await
            .unwrap();
    }

    // The native query accepts raw OS paths even though persisted cwd strings
    // have their own separate format restrictions. Copy an ordinary rollout to
    // a non-UTF-8 parent and compare the actual native and process read results.
    let id = ThreadId::new();
    native
        .create_thread(fixture.params(id, ThreadHistoryMode::Legacy))
        .await
        .unwrap();
    native
        .append_items(AppendThreadItemsParams {
            thread_id: id,
            items: vec![crate::support::message("raw path history")],
        })
        .await
        .unwrap();
    native
        .persist_thread(id, PersistContext::Standard)
        .await
        .unwrap();
    native.flush_thread(id).await.unwrap();
    let original = native.local_rollout_path(id).await.unwrap().unwrap();
    native.shutdown_thread(id).await.unwrap();
    std::fs::copy(original, &raw_path).unwrap();
    let params = ReadThreadByRolloutPathParams {
        rollout_path: raw_path.clone(),
        include_archived: false,
        include_history: true,
    };
    let expected = ThreadStore::read_thread_by_rollout_path(&native, params.clone())
        .await
        .unwrap();
    let actual = process.read_thread_by_rollout_path(params).await.unwrap();
    assert_eq!(actual.rollout_path, Some(raw_path));
    assert_eq!(
        serde_json::to_value(StorageResponse::ReadThreadByRolloutPath(actual)).unwrap(),
        serde_json::to_value(StorageResponse::ReadThreadByRolloutPath(expected)).unwrap()
    );
    let mut filter = crate::support::list_params();
    filter.cwd_filters = Some(vec![directory]);
    assert!(
        native
            .list_threads(filter.clone())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(process.list_threads(filter).await.unwrap().items.is_empty());
    process.shutdown_process().await.unwrap();
}
