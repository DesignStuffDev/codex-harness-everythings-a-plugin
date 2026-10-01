use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use crate::error_code::internal_error;
use crate::error_code::invalid_request;
use crate::file_search_services::SearchContext;
use crate::fuzzy_file_search::PendingSearchObserver;
use crate::fuzzy_file_search::PendingSearchSession;
use crate::fuzzy_file_search::PublisherFailures;
use crate::fuzzy_file_search::SearchStartCause;
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
mod ingress;
use connection::OneShotGuard;
use connection::OneShotWaiter;
pub(crate) use connection::SearchConnectionState;
use connection::SessionEntry;
use connection::StartWaiter;
use ingress::QueuedSearchStart;
pub(crate) use ingress::SearchAdmission;

#[derive(Clone)]
pub(crate) struct SearchRequestProcessor {
    outgoing: Arc<OutgoingMessageSender>,
    context: SearchContext,
    state: Arc<Mutex<ProcessorState>>,
    failures: PublisherFailures,
}

#[derive(Default)]
struct ProcessorState {
    closed: bool,
    connections: HashMap<ConnectionId, Arc<SearchConnectionState>>,
}

impl SearchRequestProcessor {
    pub(crate) fn new(outgoing: Arc<OutgoingMessageSender>, context: SearchContext) -> Self {
        Self {
            outgoing,
            context,
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
        let mut connection_state = connection
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed || connection_state.closed {
            return Err(invalid_request("search connection is closed"));
        }
        if connection_state
            .connection_id
            .is_some_and(|id| id != connection_id)
            || state
                .connections
                .get(&connection_id)
                .is_some_and(|existing| !Arc::ptr_eq(existing, connection))
        {
            return Err(invalid_request(
                "file search connection identity does not match",
            ));
        }
        if let Some(scope) = connection.scope.get() {
            if !self.context.factory.owns_scope(scope) {
                return Err(invalid_request(
                    "file search connection belongs to another runtime",
                ));
            }
        } else {
            let scope = self
                .context
                .factory
                .new_scope(self.context.scope_limits)
                .map_err(|error| search_error(error.into()))?;
            // Both processor and connection admission fences are held; a foreign
            // processor cannot race another initialization or shutdown here.
            connection
                .scope
                .set(scope)
                .map_err(|_| internal_error("file search scope initialization raced"))?;
        }
        connection_state.connection_id = Some(connection_id);
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
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state
            .connections
            .get(&connection_id)
            .is_some_and(|existing| std::ptr::eq(existing.as_ref(), connection))
        {
            state.connections.remove(&connection_id);
        }
    }

    pub(crate) async fn shutdown(&self) -> anyhow::Result<()> {
        self.request_shutdown();
        let connections = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closed = true;

            state.connections.values().cloned().collect::<Vec<_>>()
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
        let connections = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closed = true;
            state.connections.values().cloned().collect::<Vec<_>>()
        };
        for connection in connections {
            connection.request_shutdown();
        }
    }

    pub(crate) async fn fuzzy_file_search(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchParams,
    ) -> Result<FuzzyFileSearchResponse, JSONRPCErrorError> {
        if let Some(token) = &params.cancellation_token {
            validate_id(token, "cancellationToken")?;
        }
        self.register(connection_id, &connection)?;
        let cancellation = Arc::new(PendingSearchObserver::new(
            Arc::new(AtomicBool::new(false)),
            self.context.shutdown_requested.clone(),
        ));
        let _waiter = OneShotWaiter(Arc::clone(&cancellation));
        let (reply, response) = oneshot::channel();
        let scope = connection
            .scope
            .get()
            .cloned()
            .ok_or_else(|| internal_error("file search scope missing after registration"))?;
        let (admitted, previous) = {
            let mut state = connection
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closed {
                return Err(invalid_request("search connection is closed"));
            }
            // Cancellation remains available at capacity. The predecessor keeps
            // its slot until the retained owner observes its cleanup receipt.
            let previous = params
                .cancellation_token
                .as_ref()
                .and_then(|token| state.tokens.get(token))
                .and_then(|id| state.one_shots.get(id))
                .cloned();
            let admitted: Result<bool, JSONRPCErrorError> = (|| {
                if params.query.is_empty() || params.roots.is_empty() {
                    return Ok(false);
                }
                if params.query.len() > self.context.max_query_bytes.get() {
                    return Err(invalid_request(
                        "file search query exceeds the UTF-8 byte limit",
                    ));
                }
                let id = state.admit().map_err(search_error)?;
                if let Some(token) = &params.cancellation_token {
                    state.tokens.insert(token.clone(), id);
                }
                state.one_shots.insert(id, Arc::clone(&cancellation));
                let guard = OneShotGuard {
                    connection: Arc::clone(&connection),
                    id,
                    token: params.cancellation_token,
                    cancellation: Arc::clone(&cancellation),
                };
                let context = self.context.clone();
                let failures = connection.failures.clone();
                connection.startups.spawn(async move {
                    let request_guard = guard;
                    let result = AssertUnwindSafe(run_fuzzy_file_search(
                        &scope,
                        &context,
                        params.query,
                        params.roots,
                        cancellation,
                    ))
                    .catch_unwind()
                    .await
                    .unwrap_or_else(|_| {
                        failures.record(
                            "file search request owner panicked; cleanup was not confirmed",
                        );
                        Err(anyhow::anyhow!(
                            "file search request owner panicked; cleanup was not confirmed"
                        ))
                    });
                    drop(request_guard);
                    let _ = reply.send(result);
                });
                Ok(true)
            })();
            (admitted, previous)
        };
        // Cancellation remains synchronous even if replacement admission fails,
        // but no backend hook executes while the connection fence is held.
        if let Some(previous) = previous {
            previous.request_close();
        }
        if !admitted? {
            return Ok(FuzzyFileSearchResponse { files: Vec::new() });
        }
        let files = response
            .await
            .map_err(|_| internal_error("file search request owner lost"))?
            .map_err(search_error)?;
        Ok(FuzzyFileSearchResponse { files })
    }

    #[cfg(test)]
    pub(crate) async fn fuzzy_file_search_session_start_response(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchSessionStartParams,
    ) -> Result<FuzzyFileSearchSessionStartResponse, JSONRPCErrorError> {
        self.start_search(connection_id, connection, params, None)
            .await
    }

    async fn start_search(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchSessionStartParams,
        mut ingress: Option<QueuedSearchStart>,
    ) -> Result<FuzzyFileSearchSessionStartResponse, JSONRPCErrorError> {
        validate_id(&params.session_id, "sessionId")?;
        self.register(connection_id, &connection)?;
        let FuzzyFileSearchSessionStartParams { session_id, roots } = params;
        if session_id.is_empty() {
            return Err(invalid_request("sessionId must not be empty"));
        }
        if roots.is_empty() {
            return Err(invalid_request("at least one search directory is required"));
        }
        let scope = connection
            .scope
            .get()
            .cloned()
            .ok_or_else(|| internal_error("file search scope missing after registration"))?;
        let (reply, response) = oneshot::channel();
        let id = {
            let mut state = connection
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closed || state.pending.contains_key(&session_id) {
                return Err(invalid_request(
                    "search connection is closed or session start is already pending",
                ));
            }
            // Reserve the replacement identity before releasing the fence. The
            // predecessor stays owned by this single retained task through join.
            if let Some(ticket) = &mut ingress {
                ticket.accepted();
            }
            let previous = state.sessions.remove(&session_id);
            let id = match state.admit() {
                Ok(id) => id,
                Err(error) => {
                    if let Some(previous) = previous {
                        state.sessions.insert(session_id, previous);
                    }
                    if let Some(ticket) = &mut ingress {
                        ticket.record_failure(&error);
                        ticket.restored_without_work();
                    }
                    return Err(search_error(error));
                }
            };
            let pending = PendingSearchSession::new(
                connection_id,
                session_id.clone(),
                Arc::clone(&self.outgoing),
                &connection.publishers,
                connection.failures.clone(),
                self.context.shutdown_requested.clone(),
            );
            let (finished, complete) = watch::channel(None);
            state.pending.insert(session_id.clone(), id);
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
            let context = self.context.clone();
            // Register while holding admission. Shutdown cannot miss a task
            // whose selected backend constructor has not been polled yet.
            connection.startups.spawn(async move {
                // The ticket was armed before this task existed. Bind outside
                // the connection lock before any predecessor or constructor wait.
                if let Some(ticket) = &ingress {
                    ticket.bind(pending.observer());
                }
                let mut cleanup = Ok(());
                let operation = async {
                    if let Some(previous) = previous
                        && let Err(error) = previous.close().await
                    {
                        if let Some(ticket) = &ingress {
                            ticket.record_failure(&error);
                        }
                        cleanup = Err(format!("{error:#}"));
                        if let Err(publisher) = pending.close().await {
                            cleanup = Err(format!(
                                "{error:#}; replacement publisher cleanup failed: {publisher:#}"
                            ));
                        }
                        return Err(error);
                    }
                    let cause = ingress
                        .as_ref()
                        .map(QueuedSearchStart::cause)
                        .unwrap_or_else(|| Arc::new(SearchStartCause::default()));
                    let session = match pending.start(&scope, &context, roots, &cause).await {
                        Ok(session) => session,
                        Err(failure) => {
                            cleanup = failure
                                .cleanup
                                .as_ref()
                                .copied()
                                .map_err(|error| format!("{error:#}"));
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
                            if let Some(ticket) = &ingress {
                                ticket.record_failure(&error);
                            }
                            cleanup = Err(format!("{error:#}"));
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
                        cleanup = Err(
                            "file search startup task panicked; cleanup was not confirmed"
                                .to_owned(),
                        );
                        Err(anyhow::anyhow!("file search startup task panicked"))
                    });
                if let Err(error) = &cleanup {
                    owned.failures.record(error);
                }
                {
                    let mut state = owned
                        .state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if state.pending.get(&owned_id) == Some(&id) {
                        state.pending.remove(&owned_id);
                    }
                    if result.is_err()
                        && state
                            .sessions
                            .get(&owned_id)
                            .is_some_and(|entry| entry.id() == id)
                    {
                        state.sessions.remove(&owned_id);
                    }
                }
                // Release the pending reservation before acknowledging stop or
                // start, so a joined restart never races stale bookkeeping.
                if let Some(ticket) = &mut ingress {
                    ticket.complete(cleanup.clone());
                }
                finished.send_replace(Some(cleanup));
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
        {
            let state = waiter
                .connection
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closed
                || state
                    .sessions
                    .get(&waiter.session_id)
                    .is_none_or(|entry| entry.id() != id)
            {
                return Err(invalid_request(
                    "file search start was released before acknowledgement",
                ));
            }
        }
        waiter.armed = false;
        Ok(FuzzyFileSearchSessionStartResponse {})
    }

    pub(crate) async fn fuzzy_file_search_session_update_response(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchSessionUpdateParams,
    ) -> Result<FuzzyFileSearchSessionUpdateResponse, JSONRPCErrorError> {
        validate_id(&params.session_id, "sessionId")?;
        self.register(connection_id, &connection)?;
        let (id, pending) = {
            let state = connection
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closed {
                return Err(invalid_request("search connection is closed"));
            }
            let Some(SessionEntry::Ready { id, session }) = state.sessions.get(&params.session_id)
            else {
                return Err(invalid_request(format!(
                    "fuzzy file search session not found: {}",
                    params.session_id
                )));
            };
            (
                *id,
                session
                    .prepare_update(params.query, self.context.max_query_bytes)
                    .map_err(search_error)?,
            )
        };
        // The real backend acknowledgement is awaited without a connection lock.
        pending.accepted().await.map_err(search_error)?;
        let state = connection
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed
            || state
                .sessions
                .get(&params.session_id)
                .is_none_or(|entry| entry.id() != id)
        {
            return Err(invalid_request(
                "file search session was released before query acknowledgement",
            ));
        }
        Ok(FuzzyFileSearchSessionUpdateResponse {})
    }

    pub(crate) async fn fuzzy_file_search_session_stop(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchSessionStopParams,
    ) -> Result<FuzzyFileSearchSessionStopResponse, JSONRPCErrorError> {
        validate_id(&params.session_id, "sessionId")?;
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

fn validate_id(value: &str, name: &str) -> Result<(), JSONRPCErrorError> {
    if value.len() > 256 {
        return Err(invalid_request(format!(
            "{name} exceeds the 256-byte UTF-8 limit"
        )));
    }
    Ok(())
}

fn search_error(error: anyhow::Error) -> JSONRPCErrorError {
    internal_error(format!("fuzzy file search failed: {error:#}"))
}

#[cfg(test)]
#[path = "search/search_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "search/runtime_tests.rs"]
mod runtime_tests;

#[cfg(test)]
#[path = "search/preparing_fixture.rs"]
mod preparing_fixture;
#[cfg(test)]
#[path = "search/preparing_tests.rs"]
mod preparing_tests;

#[cfg(test)]
#[path = "search/ingress_tests.rs"]
mod ingress_tests;

#[cfg(test)]
#[path = "search/ingress_regression_tests.rs"]
mod ingress_regression_tests;
