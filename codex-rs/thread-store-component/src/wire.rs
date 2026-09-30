//! Complete typed requests and replies for ThreadStore contract v2.
//!
//! External variant tags avoid serde's buffered tagged-enum representation,
//! which cannot preserve arbitrary-precision JSON in nested native values.
//! Rollout and provider serde is never a substitute for these trusted mirrors.

use crate::StorageError;
use crate::contract::CopyAttachmentsParams;
use crate::contract::PendingMetadataParams;
use crate::contract::PersistParams;
use crate::contract::PrepareForkRequest;
use crate::contract::PreparedForkResponse;
use codex_protocol::ThreadId;
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
use codex_thread_store::TimelinePage;
use codex_thread_store::TurnPage;
use codex_thread_store::UpdateProjectParams;
use codex_thread_store::UpdateThreadMetadataParams;
use codex_thread_store::UpdatedProject;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageRequest {
    CreateThread(#[serde(with = "crate::remote::create_thread_params")] CreateThreadParams),
    StagePendingThreadMetadata(PendingMetadataParams),
    ReadPendingThreadMetadata(ThreadId),
    RemovePendingThreadMetadata(ThreadId),
    ResumeThread(#[serde(with = "crate::remote::resume_thread_params")] ResumeThreadParams),
    AppendItems(
        #[serde(with = "crate::remote::append_thread_items_params")] AppendThreadItemsParams,
    ),
    RecordThreadMetadata(
        #[serde(with = "crate::remote::update_thread_metadata_params")] UpdateThreadMetadataParams,
    ),
    PersistThread(PersistParams),
    FlushThread(ThreadId),
    ShutdownThread(ThreadId),
    DiscardThread(ThreadId),
    LoadHistory(LoadThreadHistoryParams),
    LoadLatestModelContext(LoadThreadHistoryParams),
    PrepareFork(PrepareForkRequest),
    RevertThread(RevertThreadParams),
    ReadThread(ReadThreadParams),
    ReadThreadByRolloutPath(
        #[serde(with = "crate::remote::read_thread_by_rollout_path_params")]
        ReadThreadByRolloutPathParams,
    ),
    ListThreads(#[serde(with = "crate::remote::list_threads_params")] ListThreadsParams),
    ListThreadSections(ListThreadSectionsParams),
    CreateThreadSection(CreateThreadSectionParams),
    RenameThreadSection(RenameThreadSectionParams),
    DeleteThreadSection(DeleteThreadSectionParams),
    CopyThreadAttachments(CopyAttachmentsParams),
    AddThreadAttachment(AddThreadAttachmentParams),
    ListThreadAttachments(ListThreadAttachmentsParams),
    RemoveThreadAttachment(RemoveThreadAttachmentParams),
    ListProjects(ListProjectsParams),
    ReadProject(String),
    CreateProject(CreateProjectParams),
    UpdateProject(UpdateProjectParams),
    MoveProject(MoveProjectParams),
    DeleteProject(String),
    SearchThreads(SearchThreadsParams),
    SearchThreadOccurrences(SearchThreadOccurrencesParams),
    ListTurns(ListTurnsParams),
    ListItems(ListItemsParams),
    ListTimeline(ListTimelineParams),
    UpdateThreadMetadata(
        #[serde(with = "crate::remote::update_thread_metadata_params")] UpdateThreadMetadataParams,
    ),
    MoveThreadToSection(MoveThreadToSectionParams),
    ArchiveThread(ArchiveThreadParams),
    ArchiveThreads(ArchiveThreadsParams),
    UnarchiveThread(ArchiveThreadParams),
    DeleteThread(DeleteThreadParams),
    DeleteThreads(DeleteThreadsParams),
    ReadResumeMetadata(ThreadId),
    LocalRolloutPath(ThreadId),
    RunRolloutMaintenance(RolloutMaintenance),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageResponse {
    CreateThread(()),
    StagePendingThreadMetadata(()),
    ReadPendingThreadMetadata(
        #[serde(with = "crate::remote::thread_metadata_patch::option")] Option<ThreadMetadataPatch>,
    ),
    RemovePendingThreadMetadata(()),
    ResumeThread(()),
    AppendItems(()),
    RecordThreadMetadata(()),
    PersistThread(()),
    FlushThread(()),
    ShutdownThread(()),
    DiscardThread(()),
    LoadHistory(#[serde(with = "crate::remote::stored_thread_history")] StoredThreadHistory),
    LoadLatestModelContext(
        #[serde(with = "crate::remote::stored_model_context")] StoredModelContext,
    ),
    PrepareFork(PreparedForkResponse),
    RevertThread(()),
    ReadThread(#[serde(with = "crate::remote::stored_thread")] StoredThread),
    ReadThreadByRolloutPath(#[serde(with = "crate::remote::stored_thread")] StoredThread),
    ListThreads(#[serde(with = "crate::remote::thread_page")] ThreadPage),
    ListThreadSections(StoredThreadSectionsPage),
    CreateThreadSection(StoredThreadSection),
    RenameThreadSection(Option<StoredThreadSection>),
    DeleteThreadSection(bool),
    CopyThreadAttachments(()),
    AddThreadAttachment(AddThreadAttachmentOutcome),
    ListThreadAttachments(ThreadAttachmentPage),
    RemoveThreadAttachment(RemoveThreadAttachmentOutcome),
    ListProjects(StoredProjectsPage),
    ReadProject(Option<StoredProject>),
    CreateProject(CreatedProject),
    UpdateProject(Option<UpdatedProject>),
    MoveProject(Option<ProjectMoveOutcome>),
    DeleteProject(Option<DeletedProject>),
    SearchThreads(#[serde(with = "crate::remote::thread_search_page")] ThreadSearchPage),
    SearchThreadOccurrences(ThreadOccurrenceSearchPage),
    ListTurns(TurnPage),
    ListItems(ItemPage),
    ListTimeline(#[serde(with = "crate::remote::timeline")] TimelinePage),
    UpdateThreadMetadata(
        #[serde(with = "crate::remote::stored_thread::option")] Option<StoredThread>,
    ),
    MoveThreadToSection(()),
    ArchiveThread(()),
    ArchiveThreads(Vec<ThreadId>),
    UnarchiveThread(#[serde(with = "crate::remote::stored_thread")] StoredThread),
    DeleteThread(()),
    DeleteThreads(()),
    ReadResumeMetadata(Option<ResumeMetadata>),
    LocalRolloutPath(
        #[serde(with = "codex_component_state_codec::native_path::option")] Option<PathBuf>,
    ),
    RunRolloutMaintenance(()),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageReply {
    Ok(Box<StorageResponse>),
    Error(StorageError),
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
