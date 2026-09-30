use std::sync::Arc;
use std::time::Duration;

use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_thread_store::AppendThreadItemsParams;
use codex_thread_store::ArchiveThreadParams;
use codex_thread_store::DeleteThreadParams;
use codex_thread_store::ForkBoundary;
use codex_thread_store::LiveThread;
use codex_thread_store::LoadThreadHistoryParams;
use codex_thread_store::PersistContext;
use codex_thread_store::PrepareForkParams;
use codex_thread_store::ReadThreadByRolloutPathParams;
use codex_thread_store::ReadThreadParams;
use codex_thread_store::ResumeThreadParams;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreError;
use pretty_assertions::assert_eq;

use crate::support::Fixture;
use crate::support::message;
use crate::support::turn;

#[tokio::test]
async fn installed_storage_preserves_host_tool_evidence_in_raw_native_rollouts() {
    use codex_protocol::models::ExecutedToolCall;
    use codex_protocol::models::FunctionCallOutputPayload;
    use codex_protocol::models::InternalChatMessageMetadataPassthrough;
    use codex_protocol::models::ResponseItem;

    let fixture = Fixture::install();
    let process = fixture.open().await;
    let native = fixture.local().await;
    let item = codex_rollout::RolloutItem::ResponseItem(
        ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some("host-evidence-call".to_owned()),
            name: Some("exec".to_owned()),
            namespace: None,
            output: FunctionCallOutputPayload::from_text("42".to_owned()),
            internal_chat_message_metadata_passthrough: Some(
                InternalChatMessageMetadataPassthrough {
                    turn_id: Some("host-evidence-turn".to_owned()),
                    cell_id: Some("host-evidence-cell".to_owned()),
                    executed_tool_calls: Some(vec![ExecutedToolCall::new(
                        "calculator".to_owned(),
                        serde_json::json!({"expression": "6 * 7"}),
                    )]),
                    tool_calls_complete: Some(true),
                    ..Default::default()
                },
            ),
        }
        .into(),
    );
    let mut comparisons = Vec::new();
    for mode in [ThreadHistoryMode::Legacy, ThreadHistoryMode::Paginated] {
        let mut raw_payloads = Vec::new();
        for store in [&native as &dyn ThreadStore, process.as_ref()] {
            let thread_id = ThreadId::new();
            store
                .create_thread(fixture.params(thread_id, mode))
                .await
                .unwrap();
            store
                .append_items(AppendThreadItemsParams {
                    thread_id,
                    items: vec![item.clone()],
                })
                .await
                .unwrap();
            store
                .persist_thread(thread_id, PersistContext::Standard)
                .await
                .unwrap();
            store.flush_thread(thread_id).await.unwrap();
            let path = store.local_rollout_path(thread_id).await.unwrap().unwrap();
            store.shutdown_thread(thread_id).await.unwrap();

            // Read durable JSON bytes directly. RolloutItem deserialization
            // intentionally distrusts these fields and would hide this regression.
            let raw_rollout = std::fs::read_to_string(path).unwrap();
            let payload = raw_rollout
                .lines()
                .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
                .find(|line| {
                    line["type"] == "response_item"
                        && line["payload"]["call_id"] == "host-evidence-call"
                })
                .expect("durable tool output must exist")["payload"]
                .clone();
            raw_payloads.push(payload);
        }
        comparisons.push((mode, raw_payloads));
    }
    process.shutdown_process().await.unwrap();

    for (mode, raw_payloads) in comparisons {
        let metadata = &raw_payloads[0]["internal_chat_message_metadata_passthrough"];
        assert_eq!(metadata["cell_id"], "host-evidence-cell");
        assert_eq!(metadata["tool_calls_complete"], true);
        assert_eq!(metadata["executed_tool_calls"][0]["name"], "calculator");
        assert_eq!(
            metadata["executed_tool_calls"][0]["arguments"],
            serde_json::json!({"expression": "6 * 7"})
        );
        assert_eq!(
            raw_payloads[1], raw_payloads[0],
            "selected process storage must preserve the native durable payload in {mode:?}"
        );
    }
}

#[tokio::test]
async fn installed_storage_preserves_native_history_and_resume_across_process_restart() {
    let fixture = Fixture::install();
    let store = fixture.open().await;
    let mut expected = Vec::new();
    for mode in [ThreadHistoryMode::Legacy, ThreadHistoryMode::Paginated] {
        let thread_id = ThreadId::new();
        let params = fixture.params(thread_id, mode);
        let live = LiveThread::create(store.clone(), params.clone())
            .await
            .unwrap();
        live.append_items(&turn(thread_id, "first-turn"))
            .await
            .unwrap();
        store
            .persist_thread(thread_id, PersistContext::Standard)
            .await
            .unwrap();
        store.flush_thread(thread_id).await.unwrap();
        let history = store
            .load_latest_model_context(LoadThreadHistoryParams {
                thread_id,
                include_archived: false,
            })
            .await
            .unwrap();
        assert!(!history.items.is_empty());
        let path = live.local_rollout_path().await.unwrap().unwrap();
        let read = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .unwrap();
        let by_path = store
            .read_thread_by_rollout_path(ReadThreadByRolloutPathParams {
                rollout_path: path,
                include_archived: false,
                include_history: false,
            })
            .await
            .unwrap();
        // Native ID reads prefer SQLite timestamps, while path reads use the
        // file metadata; compare each operation against the native implementation.
        let native = fixture.local().await;
        let native_read = native
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .unwrap();
        let native_path = ThreadStore::read_thread_by_rollout_path(
            &native,
            ReadThreadByRolloutPathParams {
                rollout_path: by_path.rollout_path.clone().unwrap(),
                include_archived: false,
                include_history: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            serde_json::to_value(read).unwrap(),
            serde_json::to_value(native_read).unwrap()
        );
        assert_eq!(
            serde_json::to_value(by_path).unwrap(),
            serde_json::to_value(native_path).unwrap()
        );
        expected.push((
            thread_id,
            mode,
            params.metadata,
            serde_json::to_value(history).unwrap(),
        ));
    }
    // Process shutdown, rather than explicit per-thread shutdown, must fence every writer.
    store.shutdown_process().await.unwrap();
    let native = fixture.local().await;
    for (thread_id, _, _, history) in &expected {
        let actual = native
            .load_latest_model_context(LoadThreadHistoryParams {
                thread_id: *thread_id,
                include_archived: false,
            })
            .await
            .unwrap();
        assert_eq!(serde_json::to_value(actual).unwrap(), *history);
    }
    let restarted = fixture.open().await;
    let reopened_ids = expected.iter().map(|entry| entry.0).collect::<Vec<_>>();
    for (thread_id, mode, metadata, _) in expected {
        let resumed = LiveThread::resume(
            restarted.clone(),
            mode,
            ResumeThreadParams {
                thread_id,
                rollout_path: None,
                history: Some(Arc::new(
                    restarted
                        .load_latest_model_context(LoadThreadHistoryParams {
                            thread_id,
                            include_archived: false,
                        })
                        .await
                        .unwrap()
                        .items,
                )),
                include_archived: false,
                metadata: metadata.clone(),
            },
        )
        .await
        .unwrap();
        resumed
            .append_items(&turn(thread_id, "second-turn"))
            .await
            .unwrap();
        restarted.shutdown_thread(thread_id).await.unwrap();
        // Reopen a previously tracked ID, then leave it live for process shutdown.
        restarted
            .resume_thread(ResumeThreadParams {
                thread_id,
                rollout_path: None,
                history: Some(Arc::new(
                    restarted
                        .load_latest_model_context(LoadThreadHistoryParams {
                            thread_id,
                            include_archived: false,
                        })
                        .await
                        .unwrap()
                        .items,
                )),
                include_archived: false,
                metadata,
            })
            .await
            .unwrap();
        restarted
            .append_items(AppendThreadItemsParams {
                thread_id,
                items: turn(thread_id, "third-turn"),
            })
            .await
            .unwrap();
    }
    restarted.shutdown_store().await.unwrap();
    for thread_id in reopened_ids {
        let history = native
            .load_latest_model_context(LoadThreadHistoryParams {
                thread_id,
                include_archived: false,
            })
            .await
            .unwrap();
        assert!(
            serde_json::to_string(&history)
                .unwrap()
                .contains("third-turn")
        );
    }
}

#[tokio::test]
async fn large_native_rollout_roundtrips_beyond_transport_frame_size() {
    let fixture = Fixture::install();
    let store = fixture.open().await;
    let thread_id = ThreadId::new();
    store
        .create_thread(fixture.params(thread_id, ThreadHistoryMode::Legacy))
        .await
        .unwrap();
    let text = "x".repeat(5 * 1024 * 1024);
    store
        .append_items(AppendThreadItemsParams {
            thread_id,
            items: vec![message(&text)],
        })
        .await
        .unwrap();
    store
        .persist_thread(thread_id, PersistContext::Standard)
        .await
        .unwrap();
    let history = store
        .load_history(LoadThreadHistoryParams {
            thread_id,
            include_archived: false,
        })
        .await
        .unwrap();
    assert!(serde_json::to_string(&history).unwrap().len() > 5 * 1024 * 1024);
    let context = store
        .load_latest_model_context(LoadThreadHistoryParams {
            thread_id,
            include_archived: false,
        })
        .await
        .unwrap();
    assert!(!context.items.is_empty());
    store.shutdown_process().await.unwrap();
    let native = fixture.local().await;
    let actual = native
        .load_history(LoadThreadHistoryParams {
            thread_id,
            include_archived: false,
        })
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(history).unwrap()
    );
}

#[tokio::test]
async fn failed_preparation_discard_does_not_materialize_a_thread() {
    let fixture = Fixture::install();
    let store = fixture.open().await;
    for mode in [ThreadHistoryMode::Legacy, ThreadHistoryMode::Paginated] {
        let id = ThreadId::new();
        store.create_thread(fixture.params(id, mode)).await.unwrap();
        let path = store.local_rollout_path(id).await.unwrap().unwrap();
        store
            .persist_thread(id, PersistContext::ThreadPreparation)
            .await
            .unwrap();
        store.discard_thread(id).await.unwrap();
        assert!(
            !path.exists(),
            "discarded preparation must remain disposable"
        );
        let params = LoadThreadHistoryParams {
            thread_id: id,
            include_archived: false,
        };
        let error = store.load_history(params.clone()).await.unwrap_err();
        let native_error = fixture
            .local()
            .await
            .load_history(params)
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), native_error.to_string());
    }
    store.shutdown_process().await.unwrap();
}

#[tokio::test]
async fn fork_owner_cancellation_releases_source_while_delete_is_waiting() {
    let fixture = Fixture::install();
    let store = fixture.open().await;
    let source_id = ThreadId::new();
    store
        .create_thread(fixture.params(source_id, ThreadHistoryMode::Paginated))
        .await
        .unwrap();
    store
        .append_items(AppendThreadItemsParams {
            thread_id: source_id,
            items: turn(source_id, "fork-source"),
        })
        .await
        .unwrap();
    store
        .persist_thread(source_id, PersistContext::Standard)
        .await
        .unwrap();
    store.shutdown_thread(source_id).await.unwrap();
    let (ready, received) = tokio::sync::oneshot::channel();
    let fork_store = Arc::clone(&store);
    let owner = tokio::spawn(async move {
        let _prepared = fork_store
            .prepare_fork(PrepareForkParams {
                thread_id: source_id,
                boundary: ForkBoundary::Latest,
            })
            .await
            .unwrap();
        ready.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    received.await.unwrap();
    let delete_store = Arc::clone(&store);
    let mut deleting = tokio::spawn(async move {
        delete_store
            .delete_thread(DeleteThreadParams {
                thread_id: source_id,
            })
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut deleting)
            .await
            .is_err()
    );
    owner.abort();
    assert!(owner.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(10), deleting)
        .await
        .expect("lease release must bypass blocked delete")
        .unwrap()
        .unwrap();
    store.shutdown_process().await.unwrap();
}

#[tokio::test]
async fn fork_reference_survives_restart_and_preserves_delete_conflicts() {
    let fixture = Fixture::install();
    let store = fixture.open().await;
    let source_id = ThreadId::new();
    store
        .create_thread(fixture.params(source_id, ThreadHistoryMode::Paginated))
        .await
        .unwrap();
    store
        .append_items(AppendThreadItemsParams {
            thread_id: source_id,
            items: turn(source_id, "inherited-turn"),
        })
        .await
        .unwrap();
    store
        .persist_thread(source_id, PersistContext::Standard)
        .await
        .unwrap();
    let prepared = store
        .prepare_fork(PrepareForkParams {
            thread_id: source_id,
            boundary: ForkBoundary::Latest,
        })
        .await
        .unwrap();
    let child_id = ThreadId::new();
    let mut child = fixture.params(child_id, ThreadHistoryMode::Paginated);
    child.forked_from_id = Some(source_id);
    child.history_base = prepared.history_base;
    store.create_thread(child).await.unwrap();
    store
        .persist_thread(child_id, PersistContext::Standard)
        .await
        .unwrap();
    drop(prepared);
    store.shutdown_process().await.unwrap();
    let reopened = fixture.open().await;
    let child_history = reopened
        .load_latest_model_context(LoadThreadHistoryParams {
            thread_id: child_id,
            include_archived: false,
        })
        .await
        .unwrap();
    assert!(
        serde_json::to_string(&child_history)
            .unwrap()
            .contains("inherited-turn")
    );
    let params = DeleteThreadParams {
        thread_id: source_id,
    };
    let conflict = reopened.delete_thread(params.clone()).await.unwrap_err();
    let native_conflict = fixture
        .local()
        .await
        .delete_thread(params)
        .await
        .unwrap_err();
    assert!(matches!(conflict, ThreadStoreError::InvalidRequest { .. }));
    assert_eq!(conflict.to_string(), native_conflict.to_string());
    reopened
        .archive_thread(ArchiveThreadParams {
            thread_id: child_id,
        })
        .await
        .unwrap();
    let restored = reopened
        .unarchive_thread(ArchiveThreadParams {
            thread_id: child_id,
        })
        .await
        .unwrap();
    assert_eq!(restored.thread_id, child_id);
    reopened.shutdown_process().await.unwrap();
}

#[tokio::test]
async fn failed_fork_calls_release_cleanup_capacity_and_keep_native_error_types() {
    let fixture = Fixture::install();
    let store = fixture.open().await;
    let missing = ThreadId::new();
    let native_error = fixture
        .local()
        .await
        .prepare_fork(PrepareForkParams {
            thread_id: missing,
            boundary: ForkBoundary::Latest,
        })
        .await
        .unwrap_err()
        .to_string();
    tokio::time::timeout(Duration::from_secs(10), async {
        // More failures than the transport cleanup lane capacity: a failed native
        // prepare must release its paired cleanup guard as well as its error.
        for _ in 0..64 {
            let error = store
                .prepare_fork(PrepareForkParams {
                    thread_id: missing,
                    boundary: ForkBoundary::Latest,
                })
                .await
                .unwrap_err();
            assert!(matches!(error, ThreadStoreError::InvalidRequest { .. }));
            assert_eq!(error.to_string(), native_error);
        }
    })
    .await
    .expect("failed fork requests must not exhaust cleanup admission");
    store.shutdown_process().await.unwrap();
}

#[tokio::test]
async fn optional_sqlite_features_match_native_capabilities_and_errors() {
    let mut fixture = Fixture::install();
    fixture.initialization.state_db_enabled = false;
    let store = fixture.open().await;
    assert!(!store.supports_projects());
    assert!(!store.supports_thread_sections());
    assert!(!store.supports_thread_attachments());
    assert!(!store.supports_paginated_history_lists());
    assert!(matches!(
        store
            .create_thread_section(codex_thread_store::CreateThreadSectionParams {
                name: "SQLite required".to_owned(),
                appearance: None,
            })
            .await,
        Err(ThreadStoreError::Unsupported {
            operation: "threadSection/create"
        })
    ));
    let thread_id = ThreadId::new();
    store
        .create_thread(fixture.params(thread_id, ThreadHistoryMode::Legacy))
        .await
        .unwrap();
    store
        .append_items(AppendThreadItemsParams {
            thread_id,
            items: vec![message("native without SQLite")],
        })
        .await
        .unwrap();
    store.flush_thread(thread_id).await.unwrap();
    assert!(
        store
            .load_history(LoadThreadHistoryParams {
                thread_id,
                include_archived: false
            })
            .await
            .is_ok()
    );
    store.shutdown_process().await.unwrap();
}
