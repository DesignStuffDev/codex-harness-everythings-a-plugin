//! The processor remains unfenced while the outer composition guard shuts down
//! its provider. This fixes the shutdown ordering independently of scheduling.

use std::sync::Arc;
use std::time::Duration;

use codex_app_server_protocol::FuzzyFileSearchSessionUpdateParams;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use pretty_assertions::assert_eq;
use tokio::time::timeout;

use super::Session;
use super::fixture;
use super::started;
use super::update;
use crate::file_search_services::SearchShutdownGuard;
use crate::outgoing_message::ConnectionId;
use crate::request_processors::search::SearchConnectionState;

#[tokio::test]
async fn outer_guard_shutdown_of_active_session_does_not_require_processor_fence() {
    let session = Session::new(/*update_permits*/ 1, /*close_ready*/ true);
    let (mut processor, mut receiver, provider) = fixture(session);
    let guard = SearchShutdownGuard::new(provider);
    processor.context.shutdown_requested = guard.shutdown_signal();
    let connection = Arc::new(SearchConnectionState::default());
    started(&processor, &connection).await;
    processor
        .fuzzy_file_search_session_update_response(
            ConnectionId(1),
            Arc::clone(&connection),
            update(),
        )
        .await
        .expect("active long-session query accepted");

    // Neither the processor nor connection has run its explicit close fence.
    timeout(Duration::from_secs(2), guard.finish())
        .await
        .expect("outer provider shutdown")
        .expect("normal backend cleanup");
    assert!(!connection.state.lock().expect("connection state").closed);
    assert!(receiver.try_recv().is_err());
    let error = processor
        .fuzzy_file_search_session_update_response(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchSessionUpdateParams {
                session_id: "search".to_string(),
                query: "late".to_string(),
            },
        )
        .await
        .expect_err("shutdown intent fences presentation before processor close");
    assert!(error.message.contains("session is closed"));
    timeout(Duration::from_secs(2), processor.shutdown())
        .await
        .expect("late processor cleanup")
        .expect("deliberate shutdown must not synthesize a presentation error");
    assert!(
        receiver.try_recv().is_err(),
        "normal shutdown must not emit sessionFailed"
    );
}

#[tokio::test]
async fn shutdown_intent_preserves_real_operation_failure_and_unconfirmed_cleanup() {
    let mut session = Session::new(/*update_permits*/ 1, /*close_ready*/ true);
    Arc::get_mut(&mut session)
        .expect("unshared fixture")
        .close_outcome = SearchCloseOutcome {
        operation: Err(SearchError::new(
            SearchErrorKind::SearchFailed,
            "search worker failed during shutdown",
        )),
        cleanup: CloseCleanup::Unconfirmed(SearchError::new(
            SearchErrorKind::TransportLost,
            "search join receipt lost during shutdown",
        )),
    };
    let (mut processor, mut receiver, provider) = fixture(session);
    let guard = SearchShutdownGuard::new(provider);
    processor.context.shutdown_requested = guard.shutdown_signal();
    let connection = Arc::new(SearchConnectionState::default());
    started(&processor, &connection).await;
    processor
        .fuzzy_file_search_session_update_response(
            ConnectionId(1),
            Arc::clone(&connection),
            update(),
        )
        .await
        .expect("active query");

    let error = timeout(Duration::from_secs(2), guard.finish())
        .await
        .expect("service receipt")
        .expect_err("shutdown intent cannot erase actual backend failures");
    assert_eq!(error.kind(), std::io::ErrorKind::Other);
    assert!(
        error
            .to_string()
            .contains("search worker failed during shutdown")
    );
    assert!(
        error
            .to_string()
            .contains("search join receipt lost during shutdown")
    );
    let error = timeout(Duration::from_secs(2), processor.shutdown())
        .await
        .expect("processor cleanup")
        .expect_err("presentation cleanup must retain the actual failure too");
    assert!(format!("{error:#}").contains("search worker failed during shutdown"));
    assert!(
        receiver.try_recv().is_err(),
        "deliberate shutdown suppresses late notifications"
    );
}
