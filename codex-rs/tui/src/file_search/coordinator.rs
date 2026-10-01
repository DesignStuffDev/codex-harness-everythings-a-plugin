//! One retained picker coordinator; obsolete work closes before new admission.
use std::future::Future;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchQuery;
use codex_file_search_runtime::FileSearchSession;

use super::FileSearchRequest;
use super::reporter::TuiSessionReporter;
use super::runtime::RuntimeInner;
use super::state::lock;

struct Lease {
    session: FileSearchSession,
    reporter: Arc<TuiSessionReporter>,
    generation: FileSearchRequest,
}
impl Lease {
    fn same(&self, request: &FileSearchRequest) -> bool {
        Arc::ptr_eq(&self.generation.manager, &request.manager)
            && Arc::ptr_eq(&self.generation.session, &request.session)
    }
    async fn close(self, runtime: &RuntimeInner) {
        let outcome = observe(runtime, self.session.close()).await;
        retain(runtime, &outcome);
    }
}

pub(super) fn start(inner: &Arc<RuntimeInner>) {
    let guard = CoordinatorGuard {
        inner: Arc::clone(inner),
        finished: false,
    };
    let owned = Arc::clone(inner);
    inner.tasks.spawn(async move {
        let mut guard = guard;
        run(owned).await;
        guard.finished = true;
    });
}

async fn run(inner: Arc<RuntimeInner>) {
    let mut changed = inner.signal.subscribe();
    let mut active: Option<Lease> = None;
    loop {
        changed.borrow_and_update();
        if inner.closing() {
            break;
        }
        let retry_delivery = super::reporter::retry_pending(&inner);
        let intent = inner.take_intent();
        let Some(intent) = intent else {
            if let Some(lease) = &active {
                // A dropped/retired manager invalidates a reusable lease even
                // without another nonempty query to trigger a replacement.
                let current = lease
                    .reporter
                    .state
                    .upgrade()
                    .is_some_and(|state| lock(&state).same_session(&lease.generation));
                if !current {
                    if let Some(lease) = active.take() {
                        lease.close(&inner).await;
                    }
                    continue;
                }
                tokio::select! {
                    outcome = lease.session.wait_closed() => {
                        retain(&inner, &outcome);
                        let still_current = lease.reporter.state.upgrade().is_some_and(|state| lock(&state).same_session(&lease.generation));
                        if still_current && !inner.closing() {
                            lease.reporter.fail(cause(&outcome));
                        }
                        if let Some(lease) = active.take() { lease.close(&inner).await; }
                    }
                    result = changed.changed() => { if result.is_err() { break; } }
                    _ = tokio::time::sleep(Duration::from_millis(20)), if retry_delivery => {}
                }
            } else {
                tokio::select! {
                    result = changed.changed() => { if result.is_err() { break; } }
                    _ = tokio::time::sleep(Duration::from_millis(20)), if retry_delivery => {}
                }
            }
            continue;
        };
        if active
            .as_ref()
            .is_some_and(|lease| !lease.same(&intent.request))
            && let Some(lease) = active.take()
        {
            lease.close(&inner).await;
        }
        if !inner.current_manager(&intent) || !intent.current() {
            continue;
        }
        if active.is_none() {
            let reporter = Arc::new(TuiSessionReporter {
                state: intent.state.clone(),
                runtime: Arc::downgrade(&inner),
                generation: intent.request.clone(),
            });
            // One owner retains the pending result and its cancellation receipt;
            // newer query intent replaces only the bounded presentation slot.
            if let Some(session) = super::startup::open(&inner, &intent, reporter.clone()).await {
                active = Some(Lease {
                    session,
                    reporter,
                    generation: intent.request.clone(),
                });
                // Resample after startup. Never replay A over newer B/C intent.
                inner.requeue(intent);
            }
            continue;
        }
        let Some(lease) = &active else {
            continue;
        };
        let Some(id) = NonZeroU64::new(intent.request.query_id) else {
            lease.reporter.fail(SearchError::new(
                SearchErrorKind::InvalidInput,
                "TUI search identity is zero",
            ));
            continue;
        };
        let updated = observe(
            &inner,
            lease.session.update_query(SearchQuery {
                id,
                text: intent.query,
            }),
        )
        .await;
        if let Err(error) = updated {
            let current = lease
                .reporter
                .state
                .upgrade()
                .is_some_and(|state| lock(&state).same_session(&lease.generation));
            if current && !inner.closing() {
                lease.reporter.fail(error);
            }
            if let Some(lease) = active.take() {
                lease.close(&inner).await;
            }
        }
    }
    if let Some(lease) = active {
        lease.close(&inner).await;
    }
}

/// Keep one accepted observer pinned while retrying presentation admission. A
/// filtered or capacity-blocked wake must not wait for the backend's next ack.
/// Signals only service delivery; newer query intent remains in its bounded slot.
pub(super) async fn observe<T>(runtime: &RuntimeInner, receipt: impl Future<Output = T>) -> T {
    let mut changed = runtime.signal.subscribe();
    tokio::pin!(receipt);
    loop {
        changed.borrow_and_update();
        let retry = !runtime.closing() && super::reporter::retry_pending(runtime);
        let active = lock(&runtime.state).active.upgrade();
        let channel = active.and_then(|state| {
            let state = lock(&state);
            (state.preparing || state.session.is_some()).then(|| state.tx.app_event_tx.clone())
        });
        let disconnected = async {
            match channel {
                Some(channel) => channel.closed().await,
                None => std::future::pending::<()>().await,
            }
        };
        tokio::select! {
            biased;
            outcome = &mut receipt => return outcome,
            _ = disconnected => { super::reporter::retry_pending(runtime); },
            _ = changed.changed() => {},
            _ = tokio::time::sleep(Duration::from_millis(20)), if retry => {},
        }
    }
}

pub(super) fn retain(runtime: &RuntimeInner, outcome: &SearchCloseOutcome) {
    if let Err(error) = &outcome.operation {
        runtime.failed(error.clone());
    }
    if let CloseCleanup::Unconfirmed(error) = &outcome.cleanup {
        runtime.unconfirmed(error.clone());
    }
}
fn cause(outcome: &SearchCloseOutcome) -> SearchError {
    outcome
        .operation
        .as_ref()
        .err()
        .cloned()
        .unwrap_or_else(|| match &outcome.cleanup {
            CloseCleanup::Unconfirmed(error) => error.clone(),
            CloseCleanup::Joined => SearchError::new(
                SearchErrorKind::ClosedLease,
                "file search provider closed the active session",
            ),
        })
}
struct CoordinatorGuard {
    inner: Arc<RuntimeInner>,
    finished: bool,
}
impl Drop for CoordinatorGuard {
    fn drop(&mut self) {
        if !self.finished {
            let error = SearchError::new(
                SearchErrorKind::TransportLost,
                "TUI search coordinator ended without joining; cleanup unconfirmed",
            );
            self.inner.coordinator_lost(error);
        }
    }
}
