//! Immediate native admission and one-lease cancellation observations.

use super::*;
use codex_file_search_api::SearchStartCancellationFuture;
use codex_file_search_api::SearchStartCancellationOutcome;
use codex_file_search_api::SearchStartControl;

pub(super) fn begin_open(
    provider: Arc<Provider>,
    request: SearchOpen,
) -> Result<PendingSearchStart, SearchStartError> {
    let request = provider
        .limits
        .prepare_open(request, &provider.base_dir)
        .map_err(not_admitted)?;
    let (ready_tx, ready_rx) = oneshot::channel();
    let (session, token) = {
        let mut state = lock(&provider.state);
        if state.stopping {
            return Err(not_admitted(closed()));
        }
        if state.sessions.len() >= provider.limits.max_sessions.get() {
            return Err(not_admitted(exhausted(
                "native file-search provider session capacity exhausted",
            )));
        }
        let allocation = amounts(request.budget);
        let ceiling = amounts(provider.limits.resources);
        let mut next = [0; 3];
        for index in 0..3 {
            next[index] = state.used[index]
                .checked_add(allocation[index])
                .filter(|value| *value <= ceiling[index])
                .ok_or_else(|| {
                    not_admitted(exhausted(
                        "native file-search aggregate resource capacity exhausted",
                    ))
                })?;
        }
        let id = state.next_id.checked_add(1).ok_or_else(|| {
            not_admitted(exhausted(
                "native file-search provider identities exhausted",
            ))
        })?;
        let session = Arc::new(NativeSession::new(id, allocation, provider.limits));
        #[cfg(test)]
        {
            *lock(&session.before_pending) = lock(&provider.before_pending).clone();
            *lock(&session.before_handoff) = lock(&provider.before_handoff).clone();
        }
        let token = provider.tasks.token();
        state.next_id = id;
        state.used = next;
        state.sessions.insert(id, session.clone());
        (session, token)
    };
    let public: Arc<dyn SearchBackendSession> = Arc::new(NativeLease {
        inner: session.clone(),
    });
    let guard = SessionTaskGuard {
        provider: provider.clone(),
        session: session.clone(),
        armed: true,
        _token: token,
    };
    let runtime = provider.runtime.clone();
    let retained = session.clone();
    runtime.spawn(async move {
        let mut guard = guard;
        run_session(provider, retained, request, ready_tx).await;
        guard.armed = false;
    });
    let control = Arc::new(NativeStartControl {
        session: session.clone(),
    });
    Ok(PendingSearchStart::new(
        control,
        Box::pin(async move {
            match ready_rx.await {
                Ok(Ok(())) => {
                    #[cfg(test)]
                    {
                        let before_handoff = lock(&session.before_handoff).clone();
                        if let Some(before_handoff) = before_handoff {
                            before_handoff();
                        }
                    }
                    // Ready in the channel is not a public handoff. This fence
                    // decides whether close or transfer of this exact lease won.
                    let closing = session.is_closing();
                    if !closing {
                        return Ok(public);
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => session.request_close(),
            }
            let outcome = observe_cancel(session.cancelled.subscribe()).await;
            Err(SearchStartError {
                operation: outcome.operation.err().unwrap_or_else(closed),
                cleanup: outcome.cleanup,
            })
        }),
    ))
}

struct NativeStartControl {
    session: Arc<NativeSession>,
}

impl SearchStartControl for NativeStartControl {
    fn request_cancel(&self) {
        self.session.request_close();
    }

    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        Box::pin(observe_cancel(self.session.cancelled.subscribe()))
    }
}

async fn observe_cancel(
    mut receiver: watch::Receiver<Option<SearchStartCancellationOutcome>>,
) -> SearchStartCancellationOutcome {
    loop {
        if let Some(outcome) = receiver.borrow_and_update().clone() {
            return outcome;
        }
        if receiver.changed().await.is_err() {
            let error = failed("native file-search start cancellation receipt owner disappeared");
            return SearchStartCancellationOutcome {
                operation: Err(error.clone()),
                cleanup: StartCleanup::Unconfirmed(error),
            };
        }
    }
}
