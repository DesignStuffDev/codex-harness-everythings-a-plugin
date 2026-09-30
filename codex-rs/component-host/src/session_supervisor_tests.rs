#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::Duration;

use pretty_assertions::assert_eq;
use tokio::time::timeout;

use super::Tasks;

#[tokio::test]
async fn completed_worker_error_is_preserved_while_the_sibling_is_cancelled_and_joined() {
    let reader = tokio::spawn(async { Err(anyhow::anyhow!("worker failed before cleanup")) });
    timeout(Duration::from_secs(5), async {
        while !reader.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut tasks = Tasks {
        reader: Some(reader),
        writer: Some(tokio::spawn(std::future::pending())),
    };
    let error = timeout(Duration::from_secs(5), tasks.stop_and_join())
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(
        error.root_cause().to_string(),
        "worker failed before cleanup"
    );
    assert!(tasks.reader.is_none());
    assert!(tasks.writer.is_none());
}
