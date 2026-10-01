//! Presentation adaptation for the runtime-selected search provider.

use std::num::NonZero;
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use codex_app_server_protocol::FuzzyFileSearchMatchType;
use codex_app_server_protocol::FuzzyFileSearchResult;
use codex_file_search_api as file_search;
use codex_file_search_runtime::FileSearchScope;
use codex_file_search_runtime::FileSearchSession;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::file_search_services::SearchContext;
use crate::outgoing_message::ConnectionId;
use crate::outgoing_message::OutgoingMessageSender;

mod lifecycle;
mod publisher;
mod startup;
pub(crate) use lifecycle::PendingSearchUpdate;
pub(crate) use publisher::PublisherFailures;
use publisher::SearchObserver;
use publisher::SearchPublisher;

const MATCH_LIMIT: usize = 50;
const MAX_THREADS: usize = 12;

#[expect(clippy::expect_used)]
pub(crate) fn options() -> file_search::FileSearchOptions {
    let cores = std::thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(1);
    file_search::FileSearchOptions {
        limit: NonZero::new(MATCH_LIMIT).expect("search limit is nonzero"),
        threads: NonZero::new(cores.clamp(1, MAX_THREADS)).expect("search threads are nonzero"),
        compute_indices: true,
        ..Default::default()
    }
}

pub(crate) async fn run_fuzzy_file_search(
    scope: &FileSearchScope,
    context: &SearchContext,
    query: String,
    roots: Vec<String>,
    observer: Arc<SearchObserver>,
) -> anyhow::Result<Vec<FuzzyFileSearchResult>> {
    if roots.is_empty() || query.is_empty() {
        return Ok(Vec::new());
    }
    anyhow::ensure!(
        query.len() <= context.max_query_bytes.get(),
        "file search query exceeds the UTF-8 byte limit"
    );
    let query_id = observer.set_query(query.clone(), context.max_query_bytes)?;
    let (session, _startup) =
        match startup::open(scope, search_open(context, roots), observer.clone()).await? {
            startup::Started::Ready(session, startup) => (session, startup),
            startup::Started::Cancelled => return Ok(observer.files()),
        };
    let mut changes = observer.subscribe();
    let result = async {
        let update = session.update_query(file_search::SearchQuery {
            id: NonZeroU64::new(query_id)
                .ok_or_else(|| anyhow::anyhow!("search query identity is zero"))?,
            text: query,
        });
        tokio::pin!(update);
        loop {
            if observer.cancellation_requested() {
                return Ok(observer.files());
            }
            tokio::select! {
                result = &mut update => { result?; break; }
                _ = tokio::time::sleep(Duration::from_millis(20)) => {}
            }
        }
        loop {
            if let Some(error) = observer.error() {
                return Err(error.into());
            }
            if observer.is_complete() || observer.cancellation_requested() {
                return Ok(observer.files());
            }
            tokio::select! {
                biased;
                changed = changes.changed() => { changed?; }
                outcome = session.wait_closed() => {
                    close_result(outcome)?;
                    anyhow::bail!("file search ended before reporting query completion");
                }
                _ = tokio::time::sleep(Duration::from_millis(20)) => {}
            }
        }
    }
    .await;
    observer.request_close();
    combine(result, close_result(session.close().await))
}

pub(crate) struct FuzzyFileSearchSession {
    session: FileSearchSession,
    _startup: startup::Binding,
    publisher: SearchPublisher,
    tasks: TaskTracker,
    owners: TaskTracker,
    updating: Arc<AtomicBool>,
}

pub(crate) struct SearchCloseControl {
    observer: Arc<SearchObserver>,
    session: Option<FileSearchSession>,
}
impl SearchCloseControl {
    pub(crate) fn starting(observer: Arc<SearchObserver>) -> Self {
        Self {
            observer,
            session: None,
        }
    }
    pub(crate) fn request_close(&self) {
        self.observer.request_close();
        if let Some(session) = &self.session {
            session.request_close();
        }
    }
}

impl FuzzyFileSearchSession {
    pub(crate) fn close_control(&self) -> SearchCloseControl {
        SearchCloseControl {
            observer: self.publisher.observer().clone(),
            session: Some(self.session.clone()),
        }
    }
    /// Bounded admission under the connection fence; returned observation is
    /// awaited outside that fence. The retained task owns the actual update.
    pub(crate) fn prepare_update(
        &self,
        query: String,
        max_query_bytes: NonZero<usize>,
    ) -> anyhow::Result<PendingSearchUpdate> {
        lifecycle::prepare_update(self, query, max_query_bytes)
    }

    pub(crate) fn request_close(&self) {
        self.publisher.request_close();
        self.session.request_close();
    }

    pub(crate) async fn close(self) -> anyhow::Result<()> {
        self.request_close();
        let runtime = close_result(self.session.close().await);
        self.tasks.close();
        self.tasks.wait().await;
        let publisher = self.publisher.close().await;
        combine(runtime, publisher)
    }
}

/// Constructed before runtime admission so a connection fence stops startup.
pub(crate) struct PendingSearchSession {
    publisher: SearchPublisher,
    owners: TaskTracker,
}

/// Shared with queued admission so a known original failure survives owner loss
/// during the subsequent publisher cleanup wait. Clean retirement records none.
#[derive(Default)]
pub(crate) struct SearchStartCause(std::sync::Mutex<Option<String>>);

impl SearchStartCause {
    pub(crate) fn record(&self, error: &anyhow::Error) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_insert_with(|| format!("{error:#}"));
    }

    pub(crate) fn operation(&self) -> Result<(), String> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .map_or(Ok(()), Err)
    }
}

pub(crate) struct SearchStartFailure {
    pub(crate) operation: anyhow::Error,
    pub(crate) cleanup: anyhow::Result<()>,
}

impl PendingSearchSession {
    pub(crate) fn new(
        connection_id: ConnectionId,
        session_id: String,
        outgoing: Arc<OutgoingMessageSender>,
        tasks: &TaskTracker,
        failures: PublisherFailures,
        shutdown_requested: CancellationToken,
    ) -> Self {
        Self {
            publisher: SearchPublisher::start(
                connection_id,
                session_id,
                outgoing,
                tasks,
                failures,
                shutdown_requested,
            ),
            owners: tasks.clone(),
        }
    }

    pub(crate) fn observer(&self) -> Arc<SearchObserver> {
        Arc::clone(self.publisher.observer())
    }

    pub(crate) async fn close(self) -> anyhow::Result<()> {
        self.publisher.close().await
    }

    pub(crate) async fn start(
        self,
        scope: &FileSearchScope,
        context: &SearchContext,
        roots: Vec<String>,
        cause: &SearchStartCause,
    ) -> Result<FuzzyFileSearchSession, SearchStartFailure> {
        let observer = self.observer();
        if observer.cancellation_requested() {
            return Err(SearchStartFailure {
                operation: anyhow::anyhow!("file search start was released before admission"),
                cleanup: self.close().await,
            });
        }
        match startup::open(scope, search_open(context, roots), observer.clone()).await {
            Ok(startup::Started::Ready(session, startup)) => {
                let tasks = TaskTracker::new();
                lifecycle::watch_session(&tasks, &self.owners, session.clone(), observer);
                Ok(FuzzyFileSearchSession {
                    session,
                    _startup: startup,
                    publisher: self.publisher,
                    tasks,
                    owners: self.owners,
                    updating: Arc::new(AtomicBool::new(false)),
                })
            }
            Ok(startup::Started::Cancelled) => Err(SearchStartFailure {
                operation: anyhow::anyhow!("file search start was released before acknowledgement"),
                cleanup: self.publisher.close().await,
            }),
            Err(failure) => {
                let runtime_cleanup = match &failure.cleanup {
                    file_search::StartCleanup::NotAdmitted
                    | file_search::StartCleanup::Confirmed => Ok(()),
                    file_search::StartCleanup::Unconfirmed(error) => Err(anyhow::anyhow!(
                        "file search startup cleanup unconfirmed: {error}"
                    )),
                };
                let operation = failure.into();
                cause.record(&operation);
                let cleanup = combine(runtime_cleanup, self.publisher.close().await);
                Err(SearchStartFailure { operation, cleanup })
            }
        }
    }
}

pub(crate) use publisher::SearchObserver as PendingSearchObserver;

pub(crate) fn close_result(outcome: file_search::SearchCloseOutcome) -> anyhow::Result<()> {
    let cleanup = match outcome.cleanup {
        file_search::CloseCleanup::Joined => Ok(()),
        file_search::CloseCleanup::Unconfirmed(error) => {
            Err(anyhow::anyhow!("file search cleanup unconfirmed: {error}"))
        }
    };
    combine(outcome.operation.map_err(anyhow::Error::from), cleanup)
}

fn search_open(context: &SearchContext, roots: Vec<String>) -> file_search::SearchOpen {
    file_search::SearchOpen {
        roots: roots.into_iter().map(PathBuf::from).collect(),
        options: context.options.clone(),
        budget: context.budget,
    }
}

fn combine<T>(result: anyhow::Result<T>, cleanup: anyhow::Result<()>) -> anyhow::Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("search cleanup also failed: {cleanup:#}")))
        }
    }
}

fn collect_files(snapshot: &file_search::FileSearchSnapshot) -> Vec<FuzzyFileSearchResult> {
    if snapshot.query.is_empty() {
        return Vec::new();
    }
    let mut files = snapshot
        .matches
        .iter()
        .map(|item| FuzzyFileSearchResult {
            root: item.root.to_string_lossy().to_string(),
            path: item.path.to_string_lossy().to_string(),
            match_type: match item.match_type {
                file_search::MatchType::File => FuzzyFileSearchMatchType::File,
                file_search::MatchType::Directory => FuzzyFileSearchMatchType::Directory,
            },
            file_name: item
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            score: item.score,
            indices: item.indices.clone(),
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    files
}
