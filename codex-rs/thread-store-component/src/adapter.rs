use crate::contract::CALL_METHOD;
use crate::contract::CopyAttachmentsParams;
use crate::contract::OPEN_METHOD;
use crate::contract::PendingMetadataParams;
use crate::contract::PersistParams;
use crate::contract::PrepareForkRequest;
use crate::contract::RELEASE_FORK_METHOD;
use crate::contract::ReleaseForkRequest;
use crate::contract::StorageCapabilities;
use crate::contract::StorageInitialization;
use crate::contract::THREAD_STORE_CONTRACT_VERSION;
use codex_component_host::ComponentBinding;
use codex_component_host::ComponentSession;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_thread_store::AddThreadAttachmentOutcome;
use codex_thread_store::AddThreadAttachmentParams;
use codex_thread_store::AppendThreadItemsParams;
use codex_thread_store::ArchiveThreadParams;
use codex_thread_store::ArchiveThreadsParams;
use codex_thread_store::CreateProjectParams;
use codex_thread_store::CreateThreadParams;
use codex_thread_store::CreateThreadSectionParams;
use codex_thread_store::CreatedProject;
use codex_thread_store::DeleteThreadParams;
use codex_thread_store::DeleteThreadSectionParams;
use codex_thread_store::DeleteThreadsParams;
use codex_thread_store::DeletedProject;
use codex_thread_store::ItemPage;
use codex_thread_store::ListItemsParams;
use codex_thread_store::ListProjectsParams;
use codex_thread_store::ListThreadAttachmentsParams;
use codex_thread_store::ListThreadSectionsParams;
use codex_thread_store::ListThreadsParams;
use codex_thread_store::ListTimelineParams;
use codex_thread_store::ListTurnsParams;
use codex_thread_store::LoadThreadHistoryParams;
use codex_thread_store::MoveProjectParams;
use codex_thread_store::MoveThreadToSectionParams;
use codex_thread_store::PersistContext;
use codex_thread_store::PrepareForkParams;
use codex_thread_store::PreparedFork;
use codex_thread_store::ProjectMoveOutcome;
use codex_thread_store::ReadThreadByRolloutPathParams;
use codex_thread_store::ReadThreadParams;
use codex_thread_store::RemoveThreadAttachmentOutcome;
use codex_thread_store::RemoveThreadAttachmentParams;
use codex_thread_store::RenameThreadSectionParams;
use codex_thread_store::ResumeMetadata;
use codex_thread_store::ResumeThreadParams;
use codex_thread_store::RevertThreadParams;
use codex_thread_store::RolloutMaintenance;
use codex_thread_store::SearchThreadOccurrencesParams;
use codex_thread_store::SearchThreadsParams;
use codex_thread_store::StoredModelContext;
use codex_thread_store::StoredProject;
use codex_thread_store::StoredProjectsPage;
use codex_thread_store::StoredThread;
use codex_thread_store::StoredThreadHistory;
use codex_thread_store::StoredThreadSection;
use codex_thread_store::StoredThreadSectionsPage;
use codex_thread_store::ThreadAttachmentPage;
use codex_thread_store::ThreadMetadataPatch;
use codex_thread_store::ThreadOccurrenceSearchPage;
use codex_thread_store::ThreadPage;
use codex_thread_store::ThreadSearchPage;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreFuture;
use codex_thread_store::ThreadStoreResult;
use codex_thread_store::TimelinePage;
use codex_thread_store::TurnPage;
use codex_thread_store::UpdateProjectParams;
use codex_thread_store::UpdateThreadMetadataParams;
use codex_thread_store::UpdatedProject;
use std::path::PathBuf;
use std::sync::Arc;

use crate::StorageReply;
use crate::StorageRequest;
use crate::StorageResponse;
use crate::error::transport_error;

/// A process-scoped native ThreadStore implementation. Clones share the same
/// process and writer ownership; an interrupted call is never replayed.
#[derive(Clone)]
pub struct ProcessThreadStore {
    pub(crate) session: ComponentSession,
    capabilities: StorageCapabilities,
}

impl ProcessThreadStore {
    pub async fn connect(
        binding: ComponentBinding,
        initialization: StorageInitialization,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            binding.spec.kind == "thread_store",
            "component is not a thread_store"
        );
        anyhow::ensure!(
            binding.spec.contract_version == THREAD_STORE_CONTRACT_VERSION,
            "unsupported thread-store component contract"
        );
        let params = serde_json::to_value(&initialization)?;
        let session = binding.connect().await?;
        let capabilities = async {
            let result = session.call(OPEN_METHOD, params).await?;
            let capabilities: StorageCapabilities = serde_json::from_value(result)?;
            anyhow::ensure!(
                capabilities.contract_version == THREAD_STORE_CONTRACT_VERSION,
                "unsupported thread-store service contract"
            );
            anyhow::ensure!(
                capabilities.shared_local_sqlite == initialization.paths,
                "storage component does not preserve the configured host state paths"
            );
            Ok::<_, anyhow::Error>(capabilities)
        }
        .await;
        let capabilities = match capabilities {
            Ok(capabilities) => capabilities,
            Err(error) => {
                // Initialization can have accepted stateful work before failing
                // validation. Observe cleanup instead of merely dropping the
                // handle; cancellation still triggers SessionInner's drain.
                return match session.close().await {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(error.context(format!(
                        "storage initialization cleanup failed; completion may be unknown: {cleanup:#}"
                    ))),
                };
            }
        };
        Ok(Self {
            session,
            capabilities,
        })
    }

    pub fn capabilities(&self) -> &StorageCapabilities {
        &self.capabilities
    }

    /// Drain accepted operations and close every live writer before process exit.
    /// All clones become closed; callers must stop admitting new work first.
    pub async fn shutdown_process(&self) -> anyhow::Result<()> {
        self.session.close().await
    }

    pub(crate) async fn request(
        &self,
        request: StorageRequest,
    ) -> ThreadStoreResult<StorageResponse> {
        let params = serde_json::to_value(request).map_err(transport_error)?;
        let value = self
            .session
            .call(CALL_METHOD, params)
            .await
            .map_err(transport_error)?;
        decode_reply(value)
    }
}

pub(crate) fn decode_reply(value: serde_json::Value) -> ThreadStoreResult<StorageResponse> {
    match serde_json::from_value(value).map_err(transport_error)? {
        StorageReply::Ok(response) => Ok(*response),
        StorageReply::Error(error) => Err(error.into_native()),
    }
}

macro_rules! forward {
    ($name:ident, $variant:ident, $input:ty, $output:ty) => {
        fn $name(&self, params: $input) -> ThreadStoreFuture<'_, $output> {
            Box::pin(async move {
                match self.request(StorageRequest::$variant(params)).await? {
                    StorageResponse::$variant(value) => Ok(value),
                    _ => Err(transport_error(
                        "storage response operation does not match request",
                    )),
                }
            })
        }
    };
}

impl ThreadStore for ProcessThreadStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn begin_shutdown_store(&self) {
        self.session.begin_close();
    }
    fn shutdown_store(&self) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move { self.session.close().await.map_err(transport_error) })
    }
    fn default_history_mode(&self) -> ThreadHistoryMode {
        self.capabilities.default_history_mode
    }
    fn supports_thread_sections(&self) -> bool {
        self.capabilities.thread_sections
    }
    fn supports_thread_attachments(&self) -> bool {
        self.capabilities.thread_attachments
    }
    fn supports_projects(&self) -> bool {
        self.capabilities.projects
    }
    fn supports_paginated_history_lists(&self) -> bool {
        self.capabilities.paginated_history_lists
    }
    fn supports_rollout_maintenance(&self) -> bool {
        self.capabilities.rollout_maintenance
    }
    fn supports_manual_rollout_migration(&self) -> bool {
        self.capabilities.manual_rollout_migration
            == Some(crate::MANUAL_ROLLOUT_MIGRATION_CONTRACT_VERSION)
    }
    fn start_rollout_migration(
        &self,
        options: codex_thread_store::RolloutMigrationOptions,
    ) -> ThreadStoreFuture<'_, Box<dyn codex_thread_store::RolloutMigrationRun>> {
        Box::pin(crate::migration_adapter::start(self, options))
    }
    fn supports_rollout_path_reads(&self) -> bool {
        self.capabilities.rollout_path_reads
    }
    fn create_thread(&self, params: CreateThreadParams) -> ThreadStoreFuture<'_, ()> {
        Box::pin(crate::acquisition::acquire(
            &self.session,
            StorageRequest::CreateThread(params),
            crate::acquisition::AcquisitionKind::Create,
        ))
    }
    forward!(
        read_pending_thread_metadata,
        ReadPendingThreadMetadata,
        ThreadId,
        Option<ThreadMetadataPatch>
    );
    forward!(
        remove_pending_thread_metadata,
        RemovePendingThreadMetadata,
        ThreadId,
        ()
    );
    fn resume_thread(&self, params: ResumeThreadParams) -> ThreadStoreFuture<'_, ()> {
        Box::pin(crate::acquisition::acquire(
            &self.session,
            StorageRequest::ResumeThread(params),
            crate::acquisition::AcquisitionKind::Resume,
        ))
    }
    forward!(append_items, AppendItems, AppendThreadItemsParams, ());
    forward!(
        record_thread_metadata,
        RecordThreadMetadata,
        UpdateThreadMetadataParams,
        ()
    );
    forward!(flush_thread, FlushThread, ThreadId, ());
    forward!(shutdown_thread, ShutdownThread, ThreadId, ());
    forward!(discard_thread, DiscardThread, ThreadId, ());
    forward!(
        load_history,
        LoadHistory,
        LoadThreadHistoryParams,
        StoredThreadHistory
    );
    forward!(
        load_latest_model_context,
        LoadLatestModelContext,
        LoadThreadHistoryParams,
        StoredModelContext
    );
    forward!(revert_thread, RevertThread, RevertThreadParams, ());
    forward!(read_thread, ReadThread, ReadThreadParams, StoredThread);
    forward!(
        read_thread_by_rollout_path,
        ReadThreadByRolloutPath,
        ReadThreadByRolloutPathParams,
        StoredThread
    );
    forward!(list_threads, ListThreads, ListThreadsParams, ThreadPage);
    forward!(
        list_thread_sections,
        ListThreadSections,
        ListThreadSectionsParams,
        StoredThreadSectionsPage
    );
    forward!(
        create_thread_section,
        CreateThreadSection,
        CreateThreadSectionParams,
        StoredThreadSection
    );
    forward!(
        rename_thread_section,
        RenameThreadSection,
        RenameThreadSectionParams,
        Option<StoredThreadSection>
    );
    forward!(
        delete_thread_section,
        DeleteThreadSection,
        DeleteThreadSectionParams,
        bool
    );
    forward!(
        add_thread_attachment,
        AddThreadAttachment,
        AddThreadAttachmentParams,
        AddThreadAttachmentOutcome
    );
    forward!(
        list_thread_attachments,
        ListThreadAttachments,
        ListThreadAttachmentsParams,
        ThreadAttachmentPage
    );
    forward!(
        remove_thread_attachment,
        RemoveThreadAttachment,
        RemoveThreadAttachmentParams,
        RemoveThreadAttachmentOutcome
    );
    forward!(
        list_projects,
        ListProjects,
        ListProjectsParams,
        StoredProjectsPage
    );
    forward!(read_project, ReadProject, String, Option<StoredProject>);
    forward!(
        create_project,
        CreateProject,
        CreateProjectParams,
        CreatedProject
    );
    forward!(
        update_project,
        UpdateProject,
        UpdateProjectParams,
        Option<UpdatedProject>
    );
    forward!(
        move_project,
        MoveProject,
        MoveProjectParams,
        Option<ProjectMoveOutcome>
    );
    forward!(
        delete_project,
        DeleteProject,
        String,
        Option<DeletedProject>
    );
    forward!(
        search_threads,
        SearchThreads,
        SearchThreadsParams,
        ThreadSearchPage
    );
    forward!(
        search_thread_occurrences,
        SearchThreadOccurrences,
        SearchThreadOccurrencesParams,
        ThreadOccurrenceSearchPage
    );
    forward!(list_turns, ListTurns, ListTurnsParams, TurnPage);
    forward!(list_items, ListItems, ListItemsParams, ItemPage);
    forward!(
        list_timeline,
        ListTimeline,
        ListTimelineParams,
        TimelinePage
    );
    forward!(
        update_thread_metadata,
        UpdateThreadMetadata,
        UpdateThreadMetadataParams,
        Option<StoredThread>
    );
    forward!(
        move_thread_to_section,
        MoveThreadToSection,
        MoveThreadToSectionParams,
        ()
    );
    forward!(archive_thread, ArchiveThread, ArchiveThreadParams, ());
    forward!(
        archive_threads,
        ArchiveThreads,
        ArchiveThreadsParams,
        Vec<ThreadId>
    );
    forward!(
        unarchive_thread,
        UnarchiveThread,
        ArchiveThreadParams,
        StoredThread
    );
    forward!(delete_thread, DeleteThread, DeleteThreadParams, ());
    forward!(delete_threads, DeleteThreads, DeleteThreadsParams, ());
    forward!(
        read_resume_metadata,
        ReadResumeMetadata,
        ThreadId,
        Option<ResumeMetadata>
    );
    forward!(
        local_rollout_path,
        LocalRolloutPath,
        ThreadId,
        Option<PathBuf>
    );
    forward!(
        run_rollout_maintenance,
        RunRolloutMaintenance,
        RolloutMaintenance,
        ()
    );

    fn stage_pending_thread_metadata(
        &self,
        thread_id: ThreadId,
        patch: ThreadMetadataPatch,
    ) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            match self
                .request(StorageRequest::StagePendingThreadMetadata(
                    PendingMetadataParams { thread_id, patch },
                ))
                .await?
            {
                StorageResponse::StagePendingThreadMetadata(()) => Ok(()),
                _ => Err(transport_error(
                    "storage response operation does not match request",
                )),
            }
        })
    }

    fn persist_thread(
        &self,
        thread_id: ThreadId,
        context: PersistContext,
    ) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            match self
                .request(StorageRequest::PersistThread(PersistParams {
                    thread_id,
                    context,
                }))
                .await?
            {
                StorageResponse::PersistThread(()) => Ok(()),
                _ => Err(transport_error(
                    "storage response operation does not match request",
                )),
            }
        })
    }

    fn copy_thread_attachments(
        &self,
        source_thread_id: ThreadId,
        destination_thread_id: ThreadId,
    ) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            match self
                .request(StorageRequest::CopyThreadAttachments(
                    CopyAttachmentsParams {
                        source_thread_id,
                        destination_thread_id,
                    },
                ))
                .await?
            {
                StorageResponse::CopyThreadAttachments(()) => Ok(()),
                _ => Err(transport_error(
                    "storage response operation does not match request",
                )),
            }
        })
    }

    fn prepare_fork(&self, params: PrepareForkParams) -> ThreadStoreFuture<'_, PreparedFork> {
        Box::pin(async move {
            let lease_id = uuid::Uuid::new_v4().to_string();
            let request = StorageRequest::PrepareFork(PrepareForkRequest {
                lease_id: lease_id.clone(),
                params,
            });
            let (value, reservation) = self
                .session
                .call_with_cleanup(
                    CALL_METHOD,
                    serde_json::to_value(request).map_err(transport_error)?,
                    RELEASE_FORK_METHOD,
                    serde_json::to_value(ReleaseForkRequest { lease_id })
                        .map_err(transport_error)?,
                )
                .await
                .map_err(transport_error)?;
            match decode_reply(value)? {
                StorageResponse::PrepareFork(snapshot) => Ok(PreparedFork::new(
                    snapshot.source_thread_id,
                    snapshot.history_base,
                    Arc::new(snapshot.model_context.items),
                    reservation,
                )),
                _ => Err(transport_error(
                    "storage response operation does not match request",
                )),
            }
        })
    }
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
