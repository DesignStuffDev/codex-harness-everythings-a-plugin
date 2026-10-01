//! Bounded retained calls and public lease ownership.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use codex_file_search_api::QueryAccepted;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchFuture;
use codex_file_search_api::SearchPoll;
use codex_file_search_api::SearchQuery;
use tokio::sync::oneshot;

use crate::FILE_SEARCH_CONTRACT_VERSION;
use crate::POLL_METHOD;
use crate::PollRequest;
use crate::UPDATE_METHOD;
use crate::UpdateRequest;
use crate::UpdateResponse;
use crate::WirePoll;
use crate::WireU64;
use crate::validate_wire_poll;

use super::decode::decode;
use super::decode::encode;
use super::decode::malformed;
use super::decode::transport_error;
use super::state::Lease;
use super::state::OperationGuard;
use super::state::OperationKind;
use super::state::closed;
use super::state::lock;
use super::state::lost;
use super::state::observe;

pub(super) struct ProcessSession {
    pub inner: Arc<Lease>,
}

impl Drop for ProcessSession {
    fn drop(&mut self) {
        self.inner.request_close();
    }
}

impl SearchBackendSession for ProcessSession {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        Box::pin(update(Arc::clone(&self.inner), query))
    }

    fn next_snapshot(&self, after_revision: u64, wait: Duration) -> SearchFuture<'_, SearchPoll> {
        Box::pin(poll(Arc::clone(&self.inner), after_revision, wait))
    }

    fn request_close(&self) {
        self.inner.request_close();
    }

    fn close(&self) -> SearchCloseFuture<'_> {
        self.inner.request_close();
        Box::pin(observe(self.inner.completed.subscribe()))
    }
}

fn busy() -> SearchError {
    SearchError::new(
        SearchErrorKind::ResourceExhausted,
        "file-search lease already owns an operation in this lane",
    )
}

fn stale() -> SearchError {
    SearchError::new(
        SearchErrorKind::StaleEpoch,
        "file-search reply belongs to an obsolete query or revision",
    )
}

async fn update(lease: Arc<Lease>, query: SearchQuery) -> Result<QueryAccepted, SearchError> {
    let limits = lease.limits.get().ok_or_else(closed)?;
    let max_query =
        NonZeroUsize::new(limits.max_query_utf8_bytes as usize).ok_or_else(malformed)?;
    query.validate_after(0, max_query)?;
    let provider = lease.provider.upgrade().ok_or_else(closed)?;
    let identity = lease.identity.get().ok_or_else(closed)?.clone();
    let params = encode(&UpdateRequest {
        contract_version: FILE_SEARCH_CONTRACT_VERSION,
        identity: identity.clone(),
        query_epoch: WireU64(query.id.get()),
        query: query.text.clone(),
    })?;
    {
        let mut state = lock(&lease.state);
        if state.closing {
            return Err(closed());
        }
        if state.update_active {
            return Err(busy());
        }
        query.validate_after(state.query_id, max_query)?;
        state.update_active = true;
        // Reserve identity before transport admission: a lost response cannot
        // authorize reuse, and polls can race ahead of an update acknowledgement.
        state.query_id = query.id.get();
        state.query = query.text;
    }
    let (sender, receiver) = oneshot::channel();
    let mut guard = OperationGuard {
        lease: Arc::clone(&lease),
        kind: OperationKind::Update,
        finished: false,
    };
    tokio::spawn(async move {
        let result = match provider.transport.call(UPDATE_METHOD, params).await {
            Ok(value) => {
                match decode::<_, UpdateResponse, SearchError>(value, UPDATE_METHOD, &identity) {
                    Ok(Ok(response)) if response.accepted_query_epoch.0 == query.id.get() => {
                        Ok(QueryAccepted { id: query.id })
                    }
                    Ok(Ok(_)) => {
                        let error = malformed();
                        provider.fail(error.clone());
                        Err(error)
                    }
                    Ok(Err(error)) => Err(error),
                    Err(error) => {
                        provider.fail(error.clone());
                        Err(error)
                    }
                }
            }
            Err(error) => {
                let error = transport_error(&error);
                provider.fail(error.clone());
                Err(error)
            }
        };
        if let Err(error) = &result
            && !(error.kind() == SearchErrorKind::ClosedLease && lock(&lease.state).closing)
        {
            lease.fail(error.clone());
        }
        guard.finished = true;
        drop(guard);
        let _ = sender.send(result);
    });
    receiver.await.unwrap_or_else(|_| Err(lost()))
}

async fn poll(
    lease: Arc<Lease>,
    after_revision: u64,
    wait: Duration,
) -> Result<SearchPoll, SearchError> {
    let limits = lease.limits.get().ok_or_else(closed)?.clone();
    if wait > Duration::from_millis(u64::from(limits.max_poll_wait_ms))
        || !wait.subsec_nanos().is_multiple_of(1_000_000)
    {
        return Err(SearchError::new(
            SearchErrorKind::InvalidInput,
            "file-search poll wait must be whole milliseconds within the negotiated limit",
        ));
    }
    let provider = lease.provider.upgrade().ok_or_else(closed)?;
    let identity = lease.identity.get().ok_or_else(closed)?.clone();
    let params = encode(&PollRequest {
        contract_version: FILE_SEARCH_CONTRACT_VERSION,
        identity: identity.clone(),
        after_revision: WireU64(after_revision),
        wait_ms: wait.as_millis() as u32,
    })?;
    {
        let mut state = lock(&lease.state);
        if state.closing {
            return Err(closed());
        }
        if state.poll_active {
            return Err(busy());
        }
        if after_revision > state.revision {
            return Err(stale());
        }
        state.poll_active = true;
    }
    let (sender, receiver) = oneshot::channel();
    let mut guard = OperationGuard {
        lease: Arc::clone(&lease),
        kind: OperationKind::Poll,
        finished: false,
    };
    tokio::spawn(async move {
        let result = match provider.transport.call(POLL_METHOD, params).await {
            Ok(value) => match decode::<_, WirePoll, SearchError>(value, POLL_METHOD, &identity) {
                Ok(Ok(wire)) => {
                    let validated = validate_wire_poll(&wire, &lease.request.options, &limits)
                        .and_then(|()| wire.into_native(&lease.request.roots))
                        .and_then(|poll| {
                            poll.validate_after(after_revision)?;
                            Ok(poll)
                        });
                    match validated {
                        Ok(poll) => {
                            let result = fence_poll(&lease, poll);
                            if let Err(error) = &result
                                && error.kind() == SearchErrorKind::TransportLost
                            {
                                provider.fail(error.clone());
                            }
                            result
                        }
                        Err(_) => {
                            let error = malformed();
                            provider.fail(error.clone());
                            Err(error)
                        }
                    }
                }
                Ok(Err(error)) => Err(error),
                Err(error) => {
                    provider.fail(error.clone());
                    Err(error)
                }
            },
            Err(error) => {
                let error = transport_error(&error);
                provider.fail(error.clone());
                Err(error)
            }
        };
        if let Err(error) = &result
            && error.kind() != SearchErrorKind::StaleEpoch
            && !(error.kind() == SearchErrorKind::ClosedLease && lock(&lease.state).closing)
        {
            lease.fail(error.clone());
        }
        guard.finished = true;
        drop(guard);
        let _ = sender.send(result);
    });
    receiver.await.unwrap_or_else(|_| Err(lost()))
}

fn fence_poll(lease: &Lease, poll: SearchPoll) -> Result<SearchPoll, SearchError> {
    let mut state = lock(&lease.state);
    if state.closing {
        return Err(closed());
    }
    match &poll {
        SearchPoll::Changed(frame) => {
            if frame.query_id > state.query_id
                || (frame.query_id == state.query_id && frame.query != state.query)
            {
                return Err(malformed());
            }
            if frame.query_id < state.query_id || frame.revision < state.revision {
                return Err(stale());
            }
            state.revision = frame.revision;
        }
        SearchPoll::Unchanged { revision } => {
            if *revision != state.revision {
                return Err(stale());
            }
        }
    }
    Ok(poll)
}
