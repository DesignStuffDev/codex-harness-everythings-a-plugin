use crate::StorageRequest;
use crate::StorageResponse;
use crate::service::StorageService;
use codex_thread_store::ThreadStoreResult;

impl StorageService {
    pub(crate) async fn execute(
        &self,
        request: StorageRequest,
    ) -> ThreadStoreResult<StorageResponse> {
        match request {
            StorageRequest::CreateThread(params) => {
                let thread_id = params.thread_id;
                self.store.create_thread(params).await?;
                self.open_threads.lock().await.insert(thread_id);
                Ok(StorageResponse::CreateThread(()))
            }
            StorageRequest::StagePendingThreadMetadata(params) => self
                .store
                .stage_pending_thread_metadata(params.thread_id, params.patch)
                .await
                .map(StorageResponse::StagePendingThreadMetadata),
            StorageRequest::ReadPendingThreadMetadata(params) => self
                .store
                .read_pending_thread_metadata(params)
                .await
                .map(StorageResponse::ReadPendingThreadMetadata),
            StorageRequest::RemovePendingThreadMetadata(params) => self
                .store
                .remove_pending_thread_metadata(params)
                .await
                .map(StorageResponse::RemovePendingThreadMetadata),
            StorageRequest::ResumeThread(params) => {
                let thread_id = params.thread_id;
                self.store.resume_thread(params).await?;
                self.open_threads.lock().await.insert(thread_id);
                Ok(StorageResponse::ResumeThread(()))
            }
            StorageRequest::AppendItems(params) => self
                .store
                .append_items(params)
                .await
                .map(StorageResponse::AppendItems),
            StorageRequest::RecordThreadMetadata(params) => self
                .store
                .record_thread_metadata(params)
                .await
                .map(StorageResponse::RecordThreadMetadata),
            StorageRequest::PersistThread(params) => self
                .store
                .persist_thread(params.thread_id, params.context)
                .await
                .map(StorageResponse::PersistThread),
            StorageRequest::FlushThread(params) => self
                .store
                .flush_thread(params)
                .await
                .map(StorageResponse::FlushThread),
            StorageRequest::ShutdownThread(params) => {
                self.store.shutdown_thread(params).await?;
                Ok(StorageResponse::ShutdownThread(()))
            }
            StorageRequest::DiscardThread(params) => {
                self.store.discard_thread(params).await?;
                Ok(StorageResponse::DiscardThread(()))
            }
            StorageRequest::LoadHistory(params) => self
                .store
                .load_history(params)
                .await
                .map(StorageResponse::LoadHistory),
            StorageRequest::LoadLatestModelContext(params) => self
                .store
                .load_latest_model_context(params)
                .await
                .map(StorageResponse::LoadLatestModelContext),
            StorageRequest::PrepareFork(params) => self
                .prepare_fork(params)
                .await
                .map(StorageResponse::PrepareFork),
            StorageRequest::RevertThread(params) => self
                .store
                .revert_thread(params)
                .await
                .map(StorageResponse::RevertThread),
            StorageRequest::ReadThread(params) => self
                .store
                .read_thread(params)
                .await
                .map(StorageResponse::ReadThread),
            StorageRequest::ReadThreadByRolloutPath(params) => self
                .store
                .read_thread_by_rollout_path(params)
                .await
                .map(StorageResponse::ReadThreadByRolloutPath),
            StorageRequest::ListThreads(params) => self
                .store
                .list_threads(params)
                .await
                .map(StorageResponse::ListThreads),
            StorageRequest::ListThreadSections(params) => self
                .store
                .list_thread_sections(params)
                .await
                .map(StorageResponse::ListThreadSections),
            StorageRequest::CreateThreadSection(params) => self
                .store
                .create_thread_section(params)
                .await
                .map(StorageResponse::CreateThreadSection),
            StorageRequest::RenameThreadSection(params) => self
                .store
                .rename_thread_section(params)
                .await
                .map(StorageResponse::RenameThreadSection),
            StorageRequest::DeleteThreadSection(params) => self
                .store
                .delete_thread_section(params)
                .await
                .map(StorageResponse::DeleteThreadSection),
            StorageRequest::CopyThreadAttachments(params) => self
                .store
                .copy_thread_attachments(params.source_thread_id, params.destination_thread_id)
                .await
                .map(StorageResponse::CopyThreadAttachments),
            StorageRequest::AddThreadAttachment(params) => self
                .store
                .add_thread_attachment(params)
                .await
                .map(StorageResponse::AddThreadAttachment),
            StorageRequest::ListThreadAttachments(params) => self
                .store
                .list_thread_attachments(params)
                .await
                .map(StorageResponse::ListThreadAttachments),
            StorageRequest::RemoveThreadAttachment(params) => self
                .store
                .remove_thread_attachment(params)
                .await
                .map(StorageResponse::RemoveThreadAttachment),
            StorageRequest::ListProjects(params) => self
                .store
                .list_projects(params)
                .await
                .map(StorageResponse::ListProjects),
            StorageRequest::ReadProject(params) => self
                .store
                .read_project(params)
                .await
                .map(StorageResponse::ReadProject),
            StorageRequest::CreateProject(params) => self
                .store
                .create_project(params)
                .await
                .map(StorageResponse::CreateProject),
            StorageRequest::UpdateProject(params) => self
                .store
                .update_project(params)
                .await
                .map(StorageResponse::UpdateProject),
            StorageRequest::MoveProject(params) => self
                .store
                .move_project(params)
                .await
                .map(StorageResponse::MoveProject),
            StorageRequest::DeleteProject(params) => self
                .store
                .delete_project(params)
                .await
                .map(StorageResponse::DeleteProject),
            StorageRequest::SearchThreads(params) => self
                .store
                .search_threads(params)
                .await
                .map(StorageResponse::SearchThreads),
            StorageRequest::SearchThreadOccurrences(params) => self
                .store
                .search_thread_occurrences(params)
                .await
                .map(StorageResponse::SearchThreadOccurrences),
            StorageRequest::ListTurns(params) => self
                .store
                .list_turns(params)
                .await
                .map(StorageResponse::ListTurns),
            StorageRequest::ListItems(params) => self
                .store
                .list_items(params)
                .await
                .map(StorageResponse::ListItems),
            StorageRequest::ListTimeline(params) => self
                .store
                .list_timeline(params)
                .await
                .map(StorageResponse::ListTimeline),
            StorageRequest::UpdateThreadMetadata(params) => self
                .store
                .update_thread_metadata(params)
                .await
                .map(StorageResponse::UpdateThreadMetadata),
            StorageRequest::MoveThreadToSection(params) => self
                .store
                .move_thread_to_section(params)
                .await
                .map(StorageResponse::MoveThreadToSection),
            StorageRequest::ArchiveThread(params) => self
                .store
                .archive_thread(params)
                .await
                .map(StorageResponse::ArchiveThread),
            StorageRequest::ArchiveThreads(params) => self
                .store
                .archive_threads(params)
                .await
                .map(StorageResponse::ArchiveThreads),
            StorageRequest::UnarchiveThread(params) => self
                .store
                .unarchive_thread(params)
                .await
                .map(StorageResponse::UnarchiveThread),
            StorageRequest::DeleteThread(params) => self
                .store
                .delete_thread(params)
                .await
                .map(StorageResponse::DeleteThread),
            StorageRequest::DeleteThreads(params) => self
                .store
                .delete_threads(params)
                .await
                .map(StorageResponse::DeleteThreads),
            StorageRequest::ReadResumeMetadata(params) => self
                .store
                .read_resume_metadata(params)
                .await
                .map(StorageResponse::ReadResumeMetadata),
            StorageRequest::LocalRolloutPath(params) => self
                .store
                .local_rollout_path(params)
                .await
                .map(StorageResponse::LocalRolloutPath),
            StorageRequest::RunRolloutMaintenance(params) => self
                .store
                .run_rollout_maintenance(params)
                .await
                .map(StorageResponse::RunRolloutMaintenance),
        }
    }
}
