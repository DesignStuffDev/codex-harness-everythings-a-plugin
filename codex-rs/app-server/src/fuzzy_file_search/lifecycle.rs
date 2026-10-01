//! Retained update acknowledgements and cancellation for presentation owners.

use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_file_search_api::QueryAccepted;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchQuery;
use codex_file_search_api::SessionReporter;
use codex_file_search_runtime::FileSearchSession;
use futures::FutureExt;
use tokio::sync::oneshot;
use tokio_util::task::TaskTracker;

use super::FuzzyFileSearchSession;
use super::publisher::SearchObserver;

pub(crate) struct PendingSearchUpdate {
    response: oneshot::Receiver<Result<QueryAccepted, SearchError>>,
    observer: Arc<SearchObserver>,
}

impl PendingSearchUpdate {
    pub(crate) async fn accepted(self) -> anyhow::Result<QueryAccepted> {
        let accepted = self
            .response
            .await
            .map_err(|_| anyhow::anyhow!("file search query acknowledgement owner was lost"))??;
        if let Some(error) = self.observer.error() {
            return Err(error.into());
        }
        anyhow::ensure!(
            self.observer.is_current(accepted.id.get()),
            "file search query was released before acknowledgement"
        );
        Ok(accepted)
    }
}

pub(super) fn prepare_update(
    owner: &FuzzyFileSearchSession,
    query: String,
    max_query_bytes: NonZeroUsize,
) -> anyhow::Result<PendingSearchUpdate> {
    anyhow::ensure!(
        query.len() <= max_query_bytes.get(),
        "file search query exceeds the UTF-8 byte limit"
    );
    anyhow::ensure!(
        owner
            .updating
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok(),
        "file search query update already pending"
    );
    let guard = UpdatingGuard(Arc::clone(&owner.updating));
    let observer = Arc::clone(owner.publisher.observer());
    let id = observer.set_query(query.clone(), max_query_bytes)?;
    let id = NonZeroU64::new(id).ok_or_else(|| anyhow::anyhow!("search query identity is zero"))?;
    let (reply, response) = oneshot::channel();
    let session = owner.session.clone();
    let retained_observer = Arc::clone(&observer);
    owner.tasks.spawn(owner.owners.track_future(async move {
        let updating_guard = guard;
        let result = AssertUnwindSafe(session.update_query(SearchQuery { id, text: query }))
            .catch_unwind()
            .await
            .unwrap_or_else(|_| {
                Err(SearchError::new(
                    SearchErrorKind::SearchFailed,
                    "file search update owner panicked",
                ))
            });
        if let Err(error) = &result
            && !(error.kind() == SearchErrorKind::ClosedLease
                && retained_observer.cancellation_requested())
        {
            retained_observer.on_error(error);
            session.request_close();
        }
        // Release presentation admission before delivery. Abandoned observers
        // cannot release it early; any Unconfirmed backend work remains charged
        // separately to the facade reservation until provider recovery.
        drop(updating_guard);
        let _ = reply.send(result);
    }));
    Ok(PendingSearchUpdate { response, observer })
}

struct UpdatingGuard(Arc<AtomicBool>);
impl Drop for UpdatingGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(super) fn watch_session(
    tasks: &TaskTracker,
    owners: &TaskTracker,
    session: FileSearchSession,
    observer: Arc<SearchObserver>,
) {
    tasks.spawn(owners.track_future(async move {
        let requested = loop {
            if observer.cancellation_requested() {
                break true;
            }
            tokio::select! {
                outcome = session.wait_closed() => {
                    if !observer.cancellation_requested() {
                        let error = outcome.operation.err().unwrap_or_else(|| match outcome.cleanup {
                            codex_file_search_api::CloseCleanup::Unconfirmed(error) => error,
                            codex_file_search_api::CloseCleanup::Joined => SearchError::new(
                                SearchErrorKind::ClosedLease,
                                "file search provider closed the active session",
                            ),
                        });
                        observer.on_error(&error);
                    }
                    break false;
                }
                _ = tokio::time::sleep(Duration::from_millis(20)) => {}
            }
        };
        if requested {
            observer.request_close();
        }
        // A provider failure must reach the bounded publisher before its stop
        // fence. Explicit caller stop instead suppresses pending notifications.
        if let Err(error) = super::close_result(session.close().await) {
            observer.on_error(&SearchError::new(SearchErrorKind::SearchFailed, format!("{error:#}")));
        }
    }));
}
