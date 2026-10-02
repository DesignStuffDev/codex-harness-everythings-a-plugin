use super::finish_with;
use std::cell::Cell;

#[tokio::test]
async fn cleanup_is_awaited_on_success_and_error() {
    for operation_failed in [false, true] {
        let cleaned = Cell::new(false);
        let operation = if operation_failed {
            Err(anyhow::anyhow!("operation failed"))
        } else {
            Ok(())
        };
        let result = finish_with(operation, async {
            cleaned.set(true);
            Ok(())
        })
        .await;
        assert!(cleaned.get());
        assert_eq!(result.is_err(), operation_failed);
    }
}

#[tokio::test]
async fn cleanup_failure_is_reported_after_success() {
    let result = finish_with(Ok(()), async { anyhow::bail!("cleanup failed") }).await;
    let Err(error) = result else {
        panic!("cleanup failure must fail the executable");
    };
    assert!(error.to_string().contains("cleanup failed"));
}

#[tokio::test]
async fn cleanup_failure_retains_the_operation_error() {
    let result = finish_with(Err(anyhow::anyhow!("operation failed")), async {
        anyhow::bail!("cleanup failed")
    })
    .await;
    let Err(error) = result else {
        panic!("both errors must fail the executable");
    };
    let message = format!("{error:#}");
    assert!(message.contains("operation failed"));
    assert!(message.contains("cleanup failed"));
}
