use super::*;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn closing_waits_for_admitted_refresh_and_survives_observer_cancellation() -> anyhow::Result<()> {
    let refresh = Arc::new(McpRefresh::new());
    let permit = refresh.acquire().await?;
    let observer = tokio::spawn({
        let refresh = Arc::clone(&refresh);
        async move { refresh.wait_closed().await }
    });
    // Wait for actual close admission before cancelling its observer.
    tokio::time::timeout(Duration::from_secs(2), async {
        while !refresh.gate.is_closed() { tokio::task::yield_now().await; }
    }).await?;
    assert!(refresh.acquire().await.is_err());
    assert!(!observer.is_finished());
    observer.abort();
    let error = observer.await.err().ok_or_else(|| anyhow::anyhow!("observer completed before active refresh"))?;
    assert!(error.is_cancelled());
    assert!(tokio::time::timeout(Duration::from_millis(2), refresh.wait_closed()).await.is_err());
    drop(permit);
    tokio::time::timeout(Duration::from_secs(2), refresh.wait_closed()).await?;
    assert!(refresh.acquire().await.is_err());
    Ok(())
}

#[tokio::test]
async fn abandoned_publication_restores_dirty_state_without_reopening_shutdown() -> anyhow::Result<()> {
    let refresh = McpRefresh::new();
    refresh.invalidate();
    let permit = refresh.acquire().await?;
    assert!(refresh.claim());
    let publication = McpRefreshInvalidationGuard { refresh: &refresh, published: false };
    refresh.close();
    drop(publication);
    drop(permit);
    refresh.wait_closed().await;
    assert_eq!(refresh.is_pending(), true);
    assert!(refresh.acquire().await.is_err());
    Ok(())
}
