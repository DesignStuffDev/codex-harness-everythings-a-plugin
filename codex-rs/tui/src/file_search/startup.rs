//! Retain one pending lease through startup, retirement and cleanup evidence.
use std::sync::Arc;

use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::StartCleanup;
use codex_file_search_runtime::FileSearchSession;

use super::coordinator::observe;
use super::coordinator::retain;
use super::reporter::TuiSessionReporter;
use super::runtime::RuntimeInner;
use super::state::Intent;
use super::state::lock;

pub(super) async fn open(
    inner: &RuntimeInner,
    intent: &Intent,
    reporter: Arc<TuiSessionReporter>,
) -> Option<FileSearchSession> {
    let current = intent.state.upgrade().is_some_and(|state| {
        let state = lock(&state);
        state.same_session(&intent.request) && state.preparing
    });
    if !current || inner.closing() {
        // This explicit local skip never invokes begin_open or admits a lease.
        return None;
    }
    let begun = inner.scope.begin_open(
        SearchOpen {
            roots: vec![intent.root.clone()],
            options: inner.options.clone(),
            budget: inner.budget,
        },
        reporter.clone(),
    );
    let (opened, control) = match begun {
        Ok(pending) => {
            let control = pending.control();
            let (published, previous) = intent.state.upgrade().map_or((false, None), |state| {
                let mut state = lock(&state);
                if state.same_session(&intent.request) && state.preparing {
                    (true, state.pending_start.replace(Arc::clone(&control)))
                } else {
                    (false, None)
                }
            });
            // Neither hooks nor the last drop of an external control run under
            // a manager mutex. A retirement before publication is latched here.
            if let Some(previous) = previous {
                previous.request_cancel();
            }
            if !published || inner.closing() {
                control.request_cancel();
            }
            (observe(inner, pending.finish()).await, Some(control))
        }
        Err(error) => (Err(error), None),
    };
    match opened {
        Ok(session) => {
            let (current, pending) = intent.state.upgrade().map_or((false, None), |state| {
                let mut state = lock(&state);
                if !state.same_session(&intent.request) || !state.preparing {
                    return (false, None);
                }
                state.preparing = false;
                state.session = Some(session.clone());
                (true, state.pending_start.take())
            });
            drop(pending);
            if current && !inner.closing() {
                return Some(session);
            }
            if let Some(control) = control {
                let receipt = observe(inner, control.cancel_and_wait()).await;
                if let Err(error) = receipt.operation {
                    inner.failed(error);
                }
                if let StartCleanup::Unconfirmed(error) = receipt.cleanup {
                    inner.unconfirmed(error);
                }
            }
            retain(inner, &observe(inner, session.close()).await);
        }
        Err(error) => {
            let (current, pending) = intent.state.upgrade().map_or((false, None), |state| {
                let mut state = lock(&state);
                if !state.same_session(&intent.request) {
                    return (false, None);
                }
                (true, state.pending_start.take())
            });
            drop(pending);
            let requested = !current || inner.closing();
            // Capture a known real cause before invoking a cleanup observer.
            if !requested || error.operation.kind() != SearchErrorKind::ClosedLease {
                inner.failed(error.operation.clone());
            }
            let receipt = match control {
                Some(control) => Some(observe(inner, control.cancel_and_wait()).await),
                None => None,
            };
            let clean = !matches!(&error.cleanup, StartCleanup::Unconfirmed(_))
                && receipt.as_ref().is_some_and(|receipt| {
                    receipt.operation.is_ok()
                        && !matches!(&receipt.cleanup, StartCleanup::Unconfirmed(_))
                });
            let expected =
                requested && error.operation.kind() == SearchErrorKind::ClosedLease && clean;
            if !expected {
                inner.failed(error.operation.clone());
            }
            if let Some(receipt) = receipt {
                if let Err(cause) = receipt.operation {
                    inner.failed(cause);
                }
                if let StartCleanup::Unconfirmed(cleanup) = receipt.cleanup {
                    inner.unconfirmed(cleanup);
                }
            }
            if let StartCleanup::Unconfirmed(cleanup) = error.cleanup {
                inner.unconfirmed(cleanup);
            }
            if current && !inner.closing() {
                reporter.fail(error.operation);
            }
        }
    }
    None
}
