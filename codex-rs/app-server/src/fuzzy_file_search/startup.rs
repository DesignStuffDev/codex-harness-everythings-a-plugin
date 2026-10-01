//! One retained request owner drains startup and its exact cancellation receipt.
use super::*;

pub(super) enum Started {
    Ready(FileSearchSession, Binding),
    Cancelled,
}

pub(super) async fn open(
    scope: &FileSearchScope,
    request: file_search::SearchOpen,
    observer: Arc<SearchObserver>,
) -> Result<Started, file_search::SearchStartError> {
    if observer.cancellation_requested() {
        return Ok(Started::Cancelled);
    }
    let pending = scope.begin_open(request, observer.clone())?;
    let control = pending.control();
    let binding = Binding {
        observer: observer.clone(),
        control: control.clone(),
    };
    if observer.bind_start(control.clone()) {
        control.request_cancel();
    }
    let finish = pending.finish();
    tokio::pin!(finish);
    let result = tokio::select! {
        biased;
        _ = observer.cancelled() => {
            control.request_cancel();
            finish.await
        }
        result = &mut finish => result,
    };
    // Final handoff fence. Request retirement may have raced a ready response;
    // the retained capability still names this exact resulting lease.
    let requested = observer.cancellation_requested();
    if requested || result.is_err() {
        let cancelled = control.cancel_and_wait().await;
        let clean_cancel = requested
            && cancelled.operation.is_ok()
            && matches!(
                &cancelled.cleanup,
                file_search::StartCleanup::NotAdmitted | file_search::StartCleanup::Confirmed
            );
        return match result {
            Ok(session) => {
                drop(session);
                if clean_cancel {
                    Ok(Started::Cancelled)
                } else {
                    Err(file_search::SearchStartError {
                        operation: cancelled.operation.err().unwrap_or_else(|| {
                            match &cancelled.cleanup {
                                file_search::StartCleanup::Unconfirmed(cause) => cause.clone(),
                                file_search::StartCleanup::NotAdmitted
                                | file_search::StartCleanup::Confirmed => closed(),
                            }
                        }),
                        cleanup: cancelled.cleanup,
                    })
                }
            }
            Err(mut error) => {
                if clean_cancel
                    && error.operation.kind() == file_search::SearchErrorKind::ClosedLease
                    && matches!(
                        &error.cleanup,
                        file_search::StartCleanup::NotAdmitted
                            | file_search::StartCleanup::Confirmed
                    )
                {
                    return Ok(Started::Cancelled);
                }
                // The facade's finish result already retains the original
                // operation cause; a later release error must not replace it.
                if !matches!(&error.cleanup, file_search::StartCleanup::Unconfirmed(_))
                    && let file_search::StartCleanup::Unconfirmed(cause) = cancelled.cleanup
                {
                    error.cleanup = file_search::StartCleanup::Unconfirmed(cause);
                }
                Err(error)
            }
        };
    }
    result.map(|session| Started::Ready(session, binding))
}

fn closed() -> file_search::SearchError {
    file_search::SearchError::new(
        file_search::SearchErrorKind::ClosedLease,
        "file search start was released before acknowledgement",
    )
}

// Transfer this binding with the ready public lease. The observer remains an
// exact-lease cancellation authority through the connection's handoff window.
// Unbind when the request/session owner drops, breaking the reporter/control
// cycle. Abandonment requests cleanup but cannot prove a join.
pub(super) struct Binding {
    observer: Arc<SearchObserver>,
    control: Arc<dyn file_search::SearchStartControl>,
}
impl Drop for Binding {
    fn drop(&mut self) {
        self.observer.unbind_start(&self.control);
        self.control.request_cancel();
    }
}
