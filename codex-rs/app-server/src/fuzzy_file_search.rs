//! Presentation adaptation for backend-owned native search sessions.

use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_app_server_protocol::FuzzyFileSearchMatchType;
use codex_app_server_protocol::FuzzyFileSearchResult;
use codex_file_search as file_search;
use tokio_util::task::TaskTracker;

use crate::outgoing_message::ConnectionId;
use crate::outgoing_message::OutgoingMessageSender;

mod publisher;
pub(crate) use publisher::PublisherFailures;
use publisher::SearchObserver;
use publisher::SearchPublisher;

const MATCH_LIMIT: usize = 50;
const MAX_THREADS: usize = 12;

#[expect(clippy::expect_used)]
fn options() -> file_search::FileSearchOptions {
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
    owner: &file_search::FileSearchOwner,
    query: String,
    roots: Vec<String>,
    cancellation_flag: Arc<AtomicBool>,
) -> anyhow::Result<Vec<FuzzyFileSearchResult>> {
    if roots.is_empty() || query.is_empty() {
        return Ok(Vec::new());
    }
    let observer = Arc::new(SearchObserver::new(Arc::clone(&cancellation_flag)));
    let query_id = observer.set_query(query.clone())?;
    let session = owner
        .create(
            roots.into_iter().map(PathBuf::from).collect(),
            options(),
            observer.clone(),
            Some(cancellation_flag.clone()),
        )
        .await?;
    let mut changes = observer.subscribe();
    let result = async {
        session.update_query_tagged(&query, query_id)?;
        loop {
            if observer.is_complete() || cancellation_flag.load(Ordering::Acquire) {
                return Ok(observer.files());
            }
            if session.is_finished() {
                anyhow::bail!("file search ended before reporting query completion");
            }
            tokio::select! {
                changed = changes.changed() => { changed?; }
                _ = tokio::time::sleep(Duration::from_millis(20)) => {}
            }
        }
    }
    .await;
    observer.request_close();
    combine(result, session.close().await)
}

pub(crate) struct FuzzyFileSearchSession {
    session: file_search::ManagedFileSearchSession,
    publisher: SearchPublisher,
}

impl FuzzyFileSearchSession {
    pub(crate) fn update_query(&self, query: String) -> anyhow::Result<()> {
        // Publish expected identity before native work can invoke its reporter.
        let query_id = self.publisher.observer().set_query(query.clone())?;
        if let Err(error) = self.session.update_query_tagged(&query, query_id) {
            self.request_close();
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn request_close(&self) {
        self.publisher.request_close();
        self.session.request_close();
    }

    pub(crate) async fn close(self) -> anyhow::Result<()> {
        self.request_close();
        // Both owners retain accepted cleanup if this waiter disappears.
        let native = self.session.close().await;
        let publisher = self.publisher.close().await;
        combine(native, publisher)
    }
}

/// Constructed before native admission so a connection fence also stops startup.
pub(crate) struct PendingSearchSession {
    publisher: SearchPublisher,
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
    ) -> Self {
        Self {
            publisher: SearchPublisher::start(connection_id, session_id, outgoing, tasks, failures),
        }
    }

    pub(crate) fn observer(&self) -> Arc<SearchObserver> {
        Arc::clone(self.publisher.observer())
    }

    pub(crate) async fn start(
        self,
        owner: &file_search::FileSearchOwner,
        roots: Vec<String>,
    ) -> Result<FuzzyFileSearchSession, SearchStartFailure> {
        let observer = self.observer();
        let session = owner
            .create(
                roots.into_iter().map(PathBuf::from).collect(),
                options(),
                observer.clone(),
                Some(observer.cancellation()),
            )
            .await;
        match session {
            Ok(session) => Ok(FuzzyFileSearchSession { session, publisher: self.publisher }),
            Err(operation) => {
                // A failed constructor still owns a publisher. Requesting its
                // stop is insufficient for the joined start/stop acknowledgement.
                // Native startup rejection and cleanup failure are different:
                // intentional cancellation can have a successful close receipt.
                let native_cleanup = operation
                    .downcast_ref::<file_search::FileSearchStartError>()
                    .and_then(file_search::FileSearchStartError::cleanup_error)
                    .map(|error| anyhow::anyhow!("native startup cleanup failed: {error:#}"))
                    .map_or(Ok(()), Err);
                let cleanup = combine(native_cleanup, self.publisher.close().await);
                Err(SearchStartFailure { operation, cleanup })
            }
        }
    }
}

pub(crate) use publisher::SearchObserver as PendingSearchObserver;

fn combine<T>(result: anyhow::Result<T>, cleanup: anyhow::Result<()>) -> anyhow::Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(error.context(format!("search cleanup also failed: {cleanup:#}"))),
    }
}

fn collect_files(snapshot: &file_search::FileSearchSnapshot) -> Vec<FuzzyFileSearchResult> {
    if snapshot.query.is_empty() {
        return Vec::new();
    }
    let mut files = snapshot.matches.iter().map(|item| FuzzyFileSearchResult {
        root: item.root.to_string_lossy().to_string(),
        path: item.path.to_string_lossy().to_string(),
        match_type: match item.match_type {
            file_search::MatchType::File => FuzzyFileSearchMatchType::File,
            file_search::MatchType::Directory => FuzzyFileSearchMatchType::Directory,
        },
        file_name: item.path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        score: item.score,
        indices: item.indices.clone(),
    }).collect::<Vec<_>>();
    files.sort_by(file_search::cmp_by_score_desc_then_path_asc::<FuzzyFileSearchResult, _, _>(
        |file| file.score,
        |file| file.path.as_str(),
    ));
    files
}
