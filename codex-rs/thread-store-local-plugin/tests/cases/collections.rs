use std::collections::BTreeMap;

use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_state::ThreadSectionAppearance;
use codex_thread_store::*;
use pretty_assertions::assert_eq;

use crate::support::Fixture;
use crate::support::list_params;
use crate::support::turn;

#[tokio::test]
async fn installed_store_preserves_projects_sections_attachments_and_literal_clears() {
    let fixture = Fixture::install();
    let store = fixture.open().await;
    let first = ThreadId::new();
    let second = ThreadId::new();
    for thread_id in [first, second] {
        let live = LiveThread::create(
            store.clone(),
            fixture.params(thread_id, ThreadHistoryMode::Paginated),
        )
        .await
        .unwrap();
        live.append_items(&turn(thread_id, "collections-turn"))
            .await
            .unwrap();
        live.persist(PersistContext::Standard).await.unwrap();
        store.shutdown_thread(thread_id).await.unwrap();
    }
    let section = store
        .create_thread_section(CreateThreadSectionParams {
            name: "Research".to_owned(),
            appearance: Some(ThreadSectionAppearance {
                icon: Some("folder".to_owned()),
                color: Some("purple".to_owned()),
            }),
        })
        .await
        .unwrap();
    let unchanged = store
        .rename_thread_section(RenameThreadSectionParams {
            section_id: section.id.clone(),
            name: section.name.clone(),
            appearance: None,
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unchanged, section);
    let cleared = store
        .rename_thread_section(RenameThreadSectionParams {
            section_id: section.id.clone(),
            name: "Planning".to_owned(),
            appearance: Some(None),
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cleared.appearance, None);
    assert!(
        store
            .list_thread_sections(ListThreadSectionsParams {
                cursor: None,
                limit: 20
            })
            .await
            .unwrap()
            .sections
            .contains(&cleared)
    );
    store
        .move_thread_to_section(MoveThreadToSectionParams {
            thread_id: first,
            section: Some(section.id.clone()),
            before_thread_id: None,
        })
        .await
        .unwrap();
    let mut filter = list_params();
    filter.section = Some(None);
    let unsectioned = store
        .list_threads(filter)
        .await
        .unwrap()
        .items
        .into_iter()
        .map(|thread| thread.thread_id)
        .collect::<Vec<_>>();
    assert_eq!(unsectioned, vec![second]);
    let all = store.list_threads(list_params()).await.unwrap();
    assert_eq!(all.items.len(), 2);

    let creation = CreateProjectParams {
        name: "Persistence".to_owned(),
        roots: Vec::new(),
        metadata: BTreeMap::new(),
        thread_ids: vec![first.to_string()],
        idempotency_key: "storage-contract-project".to_owned(),
    };
    let created = store.create_project(creation.clone()).await.unwrap();
    let repeated = store.create_project(creation).await.unwrap();
    assert_eq!(repeated.project, created.project);
    assert!(!repeated.created);
    let id = created.project.id.clone();
    assert_eq!(
        store.read_project(id.clone()).await.unwrap(),
        Some(created.project.clone())
    );
    let updated = store
        .update_project(UpdateProjectParams {
            project_id: id.clone(),
            name: Some("Persistent projects".to_owned()),
            roots: None,
            metadata: Some(BTreeMap::from([("scope".to_owned(), "test".to_owned())])),
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.project.name, "Persistent projects");
    store
        .move_project(MoveProjectParams {
            project_id: id.clone(),
            before_project_id: None,
        })
        .await
        .unwrap();
    assert_eq!(
        store
            .list_projects(ListProjectsParams {
                cursor: None,
                limit: 10,
                sort_key: ProjectSortKey::Position,
                sort_direction: SortDirection::Asc
            })
            .await
            .unwrap()
            .projects,
        vec![updated.project]
    );
    let mut filter = list_params();
    filter.project_id = Some(None);
    assert_eq!(
        store
            .list_threads(filter)
            .await
            .unwrap()
            .items
            .into_iter()
            .map(|thread| thread.thread_id)
            .collect::<Vec<_>>(),
        vec![second]
    );

    let attachment = AddThreadAttachmentParams {
        thread_id: first,
        attachment_type: "test".to_owned(),
        identity_key: "document".to_owned(),
        payload: serde_json::from_str(r#"{"name":"retained","nested":[1,2,3],"precise":123456789012345678901234567890.123456789}"#).unwrap(),
    };
    let added = store
        .add_thread_attachment(attachment.clone())
        .await
        .unwrap();
    let AddThreadAttachmentOutcome::Created(expected) = added else {
        panic!("new attachment")
    };
    assert_eq!(
        store.add_thread_attachment(attachment).await.unwrap(),
        AddThreadAttachmentOutcome::Existing(expected.clone())
    );
    store.copy_thread_attachments(first, second).await.unwrap();
    assert_eq!(
        store
            .list_thread_attachments(ListThreadAttachmentsParams {
                thread_id: first,
                cursor: None,
                limit: 10
            })
            .await
            .unwrap()
            .attachments,
        vec![expected.clone()]
    );
    let copied = store
        .list_thread_attachments(ListThreadAttachmentsParams {
            thread_id: second,
            cursor: None,
            limit: 10,
        })
        .await
        .unwrap();
    assert_eq!(copied.attachments.len(), 1);
    assert_eq!(copied.attachments[0].payload, expected.payload);
    assert_eq!(
        store
            .remove_thread_attachment(RemoveThreadAttachmentParams {
                thread_id: first,
                attachment_type: "test".to_owned(),
                identity_key: "document".to_owned()
            })
            .await
            .unwrap(),
        RemoveThreadAttachmentOutcome::Removed(expected)
    );

    // Compare actual persisted collections through the unchanged native implementation.
    let projects_before = store
        .list_projects(ListProjectsParams {
            cursor: None,
            limit: 10,
            sort_key: ProjectSortKey::Position,
            sort_direction: SortDirection::Asc,
        })
        .await
        .unwrap();
    store.shutdown_process().await.unwrap();
    let native = fixture.local().await;
    assert_eq!(
        native
            .list_projects(ListProjectsParams {
                cursor: None,
                limit: 10,
                sort_key: ProjectSortKey::Position,
                sort_direction: SortDirection::Asc
            })
            .await
            .unwrap(),
        projects_before
    );
    let store = fixture.open().await;
    assert!(store.delete_project(id).await.unwrap().is_some());
    assert!(
        store
            .delete_thread_section(DeleteThreadSectionParams {
                section_id: section.id
            })
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .archive_threads(ArchiveThreadsParams {
                thread_ids: vec![first, second],
                writer_lock_thread_ids: Vec::new()
            })
            .await
            .unwrap(),
        vec![first, second]
    );
    store
        .delete_threads(DeleteThreadsParams {
            thread_ids: vec![first, second],
        })
        .await
        .unwrap();
    assert!(
        store
            .list_threads(list_params())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    store.shutdown_process().await.unwrap();
}

#[tokio::test]
async fn metadata_and_paginated_reads_preserve_native_results_and_revert() {
    let fixture = Fixture::install();
    let store = fixture.open().await;
    let thread_id = ThreadId::new();
    let patch = ThreadMetadataPatch {
        model: Some("reserved-model".to_owned()),
        ..Default::default()
    };
    store
        .stage_pending_thread_metadata(thread_id, patch.clone())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(store.read_pending_thread_metadata(thread_id).await.unwrap()).unwrap(),
        serde_json::to_value(Some(patch)).unwrap()
    );
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
    let live = LiveThread::create(
        store.clone(),
        fixture.params(thread_id, ThreadHistoryMode::Paginated),
    )
    .await
    .unwrap();
    live.append_items(&turn(thread_id, "turn-one"))
        .await
        .unwrap();
    live.append_items(&turn(thread_id, "turn-two"))
        .await
        .unwrap();
    store
        .record_thread_metadata(UpdateThreadMetadataParams {
            thread_id,
            patch: ThreadMetadataPatch {
                title: Some("Kept title".to_owned()),
                ..Default::default()
            },
            include_archived: false,
        })
        .await
        .unwrap();
    store
        .update_thread_metadata(UpdateThreadMetadataParams {
            thread_id,
            patch: ThreadMetadataPatch {
                model: Some("runtime-model".to_owned()),
                ..Default::default()
            },
            include_archived: false,
        })
        .await
        .unwrap();
    store.flush_thread(thread_id).await.unwrap();
    let metadata = store
        .read_resume_metadata(thread_id)
        .await
        .unwrap()
        .unwrap();
    assert!(metadata.title_seen);
    let turns = ListTurnsParams {
        thread_id,
        include_archived: false,
        cursor: None,
        page_size: 10,
        sort_direction: SortDirection::Asc,
        items_view: StoredTurnItemsView::Summary,
    };
    let items = ListItemsParams {
        thread_id,
        turn_id: None,
        include_archived: false,
        position: None,
        page_size: 10,
        sort_direction: SortDirection::Asc,
        sort_key: ItemSortKey::CreatedAtOrdinal,
        after_updated_at_ordinal: None,
    };
    let timeline = ListTimelineParams {
        thread_id,
        cursor: None,
        page_size: 10,
    };
    let search = SearchThreadsParams {
        page_size: 10,
        cursor: None,
        sort_key: ThreadSortKey::CreatedAt,
        sort_direction: SortDirection::Desc,
        allowed_sources: Vec::new(),
        archived: false,
        search_term: "searchable".to_owned(),
    };
    let occurrences = SearchThreadOccurrencesParams {
        thread_id,
        search_term: "searchable".to_owned(),
        cursor: None,
        page_size: 10,
    };
    let expected_turns = store.list_turns(turns.clone()).await.unwrap();
    assert_eq!(
        expected_turns
            .turns
            .iter()
            .map(|turn| turn.turn_id.as_str())
            .collect::<Vec<_>>(),
        vec!["turn-one", "turn-two"]
    );
    let expected_items = store.list_items(items.clone()).await.unwrap();
    let expected_timeline = store.list_timeline(timeline.clone()).await.unwrap();
    let expected_search = store.search_threads(search.clone()).await.unwrap();
    assert_eq!(expected_search.items.len(), 1);
    let expected_occurrences = store
        .search_thread_occurrences(occurrences.clone())
        .await
        .unwrap();
    assert_eq!(expected_occurrences.items.len(), 2);
    store.shutdown_process().await.unwrap();
    let native = fixture.local().await;
    assert_eq!(
        serde_json::to_value(native.list_turns(turns.clone()).await.unwrap()).unwrap(),
        serde_json::to_value(expected_turns).unwrap()
    );
    assert_eq!(
        serde_json::to_value(native.list_items(items).await.unwrap()).unwrap(),
        serde_json::to_value(expected_items).unwrap()
    );
    assert_eq!(
        native.list_timeline(timeline).await.unwrap(),
        expected_timeline
    );
    assert_eq!(
        serde_json::to_value(native.search_threads(search).await.unwrap()).unwrap(),
        serde_json::to_value(expected_search).unwrap()
    );
    assert_eq!(
        serde_json::to_value(native.search_thread_occurrences(occurrences).await.unwrap()).unwrap(),
        serde_json::to_value(expected_occurrences).unwrap()
    );
    let store = fixture.open().await;
    store
        .revert_thread(RevertThreadParams {
            thread_id,
            before_turn_id: "turn-two".to_owned(),
            multi_agent_version: None,
        })
        .await
        .unwrap();
    assert_eq!(
        store
            .list_turns(turns)
            .await
            .unwrap()
            .turns
            .iter()
            .map(|turn| turn.turn_id.as_str())
            .collect::<Vec<_>>(),
        vec!["turn-one"]
    );
    store
        .run_rollout_maintenance(RolloutMaintenance::Compress)
        .await
        .unwrap();
    store.shutdown_process().await.unwrap();
}
