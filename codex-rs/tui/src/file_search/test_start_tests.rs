//! Check the controlled backend's ownership so adapter tests cannot pass by
//! losing a pending owner or inventing a joined receipt when its runtime dies.
use super::*;
use pretty_assertions::assert_eq;
use std::future::Future;
use std::future::poll_fn;
use std::num::NonZeroUsize;
use std::task::Poll;
use tokio::time::timeout;

fn request() -> SearchOpen {
    SearchOpen {
        roots: vec!["/fixture".into()],
        options: FileSearchOptions::default(),
        budget: SearchBudget {
            max_index_entries: NonZeroUsize::MIN,
            max_index_bytes: NonZeroUsize::MIN,
            max_worker_threads: NonZeroUsize::MIN,
        },
    }
}

#[tokio::test]
async fn abandoned_start_drains_without_open_gate_and_does_not_cancel_its_sibling() {
    let first = Session::new(/*acknowledgements*/ 1);
    first.close_ready.send_replace(false);
    let sibling = Session::new(/*acknowledgements*/ 1);
    let (runtime, provider, backend) = fixture(vec![first.clone(), sibling.clone()]).await;
    backend.open_ready.send_replace(false);
    let first_ticket = backend.begin_open(request()).expect("first admission");
    let first_control = first_ticket.control();
    let sibling_ticket = backend.begin_open(request()).expect("sibling admission");
    drop(first_ticket.finish());
    assert!(
        first.closing.is_cancelled(),
        "unpolled drop latches cancellation"
    );
    assert!(!sibling.closing.is_cancelled());
    permit(&first.close_entered).await;
    let mut receipt = first_control.cancel_and_wait();
    poll_fn(|context| {
        assert!(
            receipt.as_mut().poll(context).is_pending(),
            "cleanup gate still owns work"
        );
        Poll::Ready(())
    })
    .await;
    drop(receipt);
    first.close_ready.send_replace(true);
    let receipt = timeout(Duration::from_secs(2), first_control.cancel_and_wait())
        .await
        .expect("cancellation needs no open-gate release");
    assert_eq!(
        receipt,
        SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::Confirmed
        }
    );
    assert_eq!(first_control.cancel_and_wait().await, receipt);
    assert!(!*backend.open_ready.borrow());
    assert!(!sibling.closing.is_cancelled());
    backend.open_ready.send_replace(true);
    let public = timeout(Duration::from_secs(2), sibling_ticket.finish())
        .await
        .expect("ready sibling")
        .expect("sibling handoff");
    first_control.request_cancel();
    assert!(
        !sibling.closing.is_cancelled(),
        "retained control targets only its original lease"
    );
    drop(public);
    runtime.shutdown().await.expect("empty adapter scope");
    assert_eq!(provider.shutdown().await, joined());
    assert!(
        backend.tasks.is_empty(),
        "provider receipt includes retained fixture owners"
    );
}

#[test]
fn lost_fixture_runtime_retains_original_failure_and_unconfirmed_cleanup() {
    let failure = SearchError::new(
        SearchErrorKind::SearchFailed,
        "fixture failed before runtime loss",
    );
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let (runtime, provider, backend) = executor.block_on(async {
        let session = Session::with_outcome(
            /*acknowledgements*/ 1,
            SearchCloseOutcome {
                operation: Err(failure.clone()),
                cleanup: CloseCleanup::Joined,
            },
        );
        fixture(vec![session]).await
    });
    backend.open_ready.send_replace(false);
    let ticket = {
        let _entered = executor.enter();
        backend
            .begin_open(request())
            .expect("accepted pending start")
    };
    let control = ticket.control();
    drop(executor);
    let observer = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("observer runtime");
    observer.block_on(async {
        let receipt = timeout(Duration::from_secs(2), control.cancel_and_wait())
            .await
            .expect("retained loss receipt");
        assert_eq!(receipt.operation, Err(failure.clone()));
        assert!(matches!(receipt.cleanup, StartCleanup::Unconfirmed(_)));
        assert_eq!(control.cancel_and_wait().await, receipt);
        let closed = backend.shutdown().await;
        assert_eq!(closed.operation, Err(failure));
        assert!(matches!(closed.cleanup, CloseCleanup::Unconfirmed(_)));
        assert!(backend.tasks.is_empty());
    });
    drop(ticket);
    drop(runtime);
    drop(provider);
}
