use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use crate::error_code::internal_error;
use crate::error_code::invalid_request;
use crate::fuzzy_file_search::PendingSearchSession;
use crate::fuzzy_file_search::PublisherFailures;
use crate::fuzzy_file_search::run_fuzzy_file_search;
use crate::outgoing_message::ConnectionId;
use crate::outgoing_message::OutgoingMessageSender;
use codex_app_server_protocol::FuzzyFileSearchParams;
use codex_app_server_protocol::FuzzyFileSearchResponse;
use codex_app_server_protocol::FuzzyFileSearchSessionStartParams;
use codex_app_server_protocol::FuzzyFileSearchSessionStartResponse;
use codex_app_server_protocol::FuzzyFileSearchSessionStopParams;
use codex_app_server_protocol::FuzzyFileSearchSessionStopResponse;
use codex_app_server_protocol::FuzzyFileSearchSessionUpdateParams;
use codex_app_server_protocol::FuzzyFileSearchSessionUpdateResponse;
use codex_app_server_protocol::JSONRPCErrorError;
use futures::FutureExt;
use tokio::sync::oneshot;
use tokio::sync::watch;

mod connection;
use connection::OneShotGuard;
pub(crate) use connection::SearchConnectionState;
use connection::SessionEntry;
use connection::StartWaiter;

#[derive(Clone)]
pub(crate) struct SearchRequestProcessor {
    outgoing: Arc<OutgoingMessageSender>,
    state: Arc<Mutex<ProcessorState>>,
    failures: PublisherFailures,
}

#[derive(Default)]
struct ProcessorState {
    closed: bool,
    connections: HashMap<ConnectionId, Arc<SearchConnectionState>>,
}

impl SearchRequestProcessor {
    pub(crate) fn new(outgoing: Arc<OutgoingMessageSender>) -> Self {
        Self {
            outgoing,
            state: Arc::new(Mutex::new(ProcessorState::default())),
            failures: PublisherFailures::default(),
        }
    }

    fn register(
        &self,
        connection_id: ConnectionId,
        connection: &Arc<SearchConnectionState>,
    ) -> Result<(), JSONRPCErrorError> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed
            || connection
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .closed
        {
            return Err(invalid_request("search connection is closed"));
        }
        state
            .connections
            .entry(connection_id)
            .or_insert_with(|| Arc::clone(connection));
        Ok(())
    }

    pub(crate) async fn connection_closed(
        &self,
        connection_id: ConnectionId,
        connection: &SearchConnectionState,
    ) {
        if let Err(error) = connection.shutdown().await {
            self.failures
                .record(&format!("connection {connection_id:?}: {error:#}"));
        }
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .connections
            .remove(&connection_id);
    }

    pub(crate) async fn shutdown(&self) -> anyhow::Result<()> {
        self.request_shutdown();
        let connections = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closed = true;
            let connections = state.connections.values().cloned().collect::<Vec<_>>();
            for connection in &connections {
                connection.request_shutdown();
            }
            connections
        };
        for connection in connections {
            if let Err(error) = connection.shutdown().await {
                self.failures.record(&format!("{error:#}"));
            }
        }
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .connections
            .clear();
        self.failures.result()
    }

    pub(crate) fn request_shutdown(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.closed = true;
        for connection in state.connections.values() {
            connection.request_shutdown();
        }
    }

    pub(crate) async fn fuzzy_file_search(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchParams,
    ) -> Result<FuzzyFileSearchResponse, JSONRPCErrorError> {
        self.register(connection_id, &connection)?;
        let cancellation = Arc::new(AtomicBool::new(false));
        let id = {
            let mut state = connection
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closed {
                return Err(invalid_request("search connection is closed"));
            }
            // Cancellation remains available at capacity. It does not release
            // the predecessor's slot until that search has joined its cleanup.
            if let Some(token) = &params.cancellation_token
                && let Some(previous) = state
                    .tokens
                    .get(token)
                    .and_then(|id| state.one_shots.get(id))
            {
                previous.store(true, Ordering::Release);
            }
            if params.query.is_empty() || params.roots.is_empty() {
                return Ok(FuzzyFileSearchResponse { files: Vec::new() });
            }
            let id = state.admit().map_err(search_error)?;
            if let Some(token) = &params.cancellation_token {
                state.tokens.insert(token.clone(), id);
            }
            state.one_shots.insert(id, Arc::clone(&cancellation));
            id
        };
        let _guard = OneShotGuard {
            connection: Arc::clone(&connection),
            id,
            token: params.cancellation_token,
            cancellation: Arc::clone(&cancellation),
        };
        let files =
            run_fuzzy_file_search(&connection.native, params.query, params.roots, cancellation)
                .await
                .map_err(search_error)?;
        Ok(FuzzyFileSearchResponse { files })
    }

    pub(crate) async fn fuzzy_file_search_session_start_response(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchSessionStartParams,
    ) -> Result<FuzzyFileSearchSessionStartResponse, JSONRPCErrorError> {
        self.register(connection_id, &connection)?;
        let FuzzyFileSearchSessionStartParams { session_id, roots } = params;
        if session_id.is_empty() {
            return Err(invalid_request("sessionId must not be empty"));
        }
        if roots.is_empty() {
            return Err(invalid_request("at least one search directory is required"));
        }
        let previous = connection
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sessions
            .remove(&session_id);
        if let Some(previous) = previous {
            previous.close().await.map_err(search_error)?;
        }
        let (reply, response) = oneshot::channel();
        let id = {
            let mut state = connection
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let id = state.admit().map_err(search_error)?;
            let pending = PendingSearchSession::new(
                connection_id,
                session_id.clone(),
                Arc::clone(&self.outgoing),
                &connection.publishers,
                connection.failures.clone(),
            );
            let (finished, complete) = watch::channel(None);
            state.sessions.insert(
                session_id.clone(),
                SessionEntry::Starting {
                    id,
                    observer: pending.observer(),
                    complete,
                },
            );
            let owned = Arc::clone(&connection);
            let owned_id = session_id.clone();
            // Register while holding the admission lock. Shutdown cannot miss a
            // startup task whose native constructor has not been polled yet.
            connection.startups.spawn(async move {
                let operation = async {
                    let session = match pending.start(&owned.native, roots).await {
                        Ok(session) => session,
                        Err(failure) => {
                            finished.send_replace(Some(
                                failure
                                    .cleanup
                                    .as_ref()
                                    .copied()
                                    .map_err(|error| format!("{error:#}")),
                            ));
                            return Err(match failure.cleanup {
                                Ok(()) => failure.operation,
                                Err(cleanup) => failure
                                    .operation
                                    .context(format!("search cleanup also failed: {cleanup:#}")),
                            });
                        }
                    };
                    let session = {
                        let mut state = owned
                            .state
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        if !state.closed
                            && state
                                .sessions
                                .get(&owned_id)
                                .is_some_and(|entry| entry.id() == id)
                        {
                            state
                                .sessions
                                .insert(owned_id.clone(), SessionEntry::Ready { id, session });
                            None
                        } else {
                            Some(session)
                        }
                    };
                    if let Some(session) = session {
                        if let Err(error) = session.close().await {
                            owned.failures.record(&format!("{error:#}"));
                            finished.send_replace(Some(Err(format!("{error:#}"))));
                            return Err(error);
                        }
                        anyhow::bail!("file search start was released before acknowledgement");
                    }
                    Ok(())
                };
                let result = AssertUnwindSafe(operation)
                    .catch_unwind()
                    .await
                    .unwrap_or_else(|_| {
                        owned.failures.record("file search startup task panicked");
                        finished.send_replace(Some(Err(
                            "file search startup task panicked; cleanup was not confirmed"
                                .to_owned(),
                        )));
                        Err(anyhow::anyhow!("file search startup task panicked"))
                    });
                if result.is_err() {
                    let mut state = owned
                        .state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if state
                        .sessions
                        .get(&owned_id)
                        .is_some_and(|entry| entry.id() == id)
                    {
                        state.sessions.remove(&owned_id);
                    }
                }
                if finished.borrow().is_none() {
                    finished.send_replace(Some(Ok(())));
                }
                let _ = reply.send(result);
            });
            id
        };
        let mut waiter = StartWaiter {
            connection,
            session_id,
            id,
            armed: true,
        };
        response
            .await
            .map_err(|_| internal_error("file search startup response lost"))?
            .map_err(search_error)?;
        waiter.armed = false;
        Ok(FuzzyFileSearchSessionStartResponse {})
    }

    pub(crate) async fn fuzzy_file_search_session_update_response(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchSessionUpdateParams,
    ) -> Result<FuzzyFileSearchSessionUpdateResponse, JSONRPCErrorError> {
        self.register(connection_id, &connection)?;
        let state = connection
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(SessionEntry::Ready { session, .. }) = state.sessions.get(&params.session_id) {
            session.update_query(params.query).map_err(search_error)?;
            Ok(FuzzyFileSearchSessionUpdateResponse {})
        } else {
            Err(invalid_request(format!(
                "fuzzy file search session not found: {}",
                params.session_id
            )))
        }
    }

    pub(crate) async fn fuzzy_file_search_session_stop(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchSessionStopParams,
    ) -> Result<FuzzyFileSearchSessionStopResponse, JSONRPCErrorError> {
        self.register(connection_id, &connection)?;
        let session = connection
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sessions
            .remove(&params.session_id);
        if let Some(session) = session {
            session.close().await.map_err(search_error)?;
        }
        Ok(FuzzyFileSearchSessionStopResponse {})
    }
}

fn search_error(error: anyhow::Error) -> JSONRPCErrorError {
    internal_error(format!("fuzzy file search failed: {error:#}"))
}

#[cfg(test)]
#[path = "search/search_tests.rs"]
mod tests;
