//! Fresh integration-test process: global process-final admission never resets.
//! This covers constructor composition, not transport shutdown or a hard exit.

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_core_plugins::PluginStartupProcessShutdown;
use codex_http_client::ClientRouteClass;
use codex_http_client::HttpClientFactory;
use codex_http_client::HttpConstructionError;
use codex_http_client::NetworkPolicy;
use codex_http_client::OutboundProxyPolicy;
use codex_http_client::RouteAwareClientPool;
use codex_http_client::RouteAwareClientPoolError;
use codex_http_client::RouteAwareRequestError;
use std::collections::BTreeSet;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::time::Instant;
use tokio::time::timeout_at;
use url::Url;

async fn serve_once(listener: TcpListener, deadline: Instant) -> Result<()> {
    let (mut socket, peer) = timeout_at(deadline, listener.accept()).await??;
    ensure!(
        peer.ip().is_loopback(),
        "fixture accepted a non-loopback peer"
    );
    let mut request = [0_u8; 4096];
    let mut used = 0;
    while !request[..used]
        .windows(4)
        .any(|window| window == b"\r\n\r\n")
    {
        ensure!(
            used < request.len(),
            "fixture request exceeded header limit"
        );
        let read = timeout_at(deadline, socket.read(&mut request[used..])).await??;
        ensure!(
            read > 0,
            "fixture peer closed before sending a complete request"
        );
        used += read;
    }
    ensure!(
        request[..used].starts_with(b"GET /constructor-custody HTTP/1.1\r\n"),
        "fixture received an unexpected HTTP request"
    );
    timeout_at(
        deadline,
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"),
    )
    .await??;
    timeout_at(deadline, socket.shutdown()).await??;
    Ok(())
}

async fn require_closed(
    pool: &RouteAwareClientPool,
    endpoint: &Url,
    deadline: Instant,
) -> Result<()> {
    let result = timeout_at(deadline, pool.get(endpoint.as_str()).send()).await?;
    ensure!(
        matches!(
            result,
            Err(RouteAwareRequestError::Route(
                RouteAwareClientPoolError::Construction(HttpConstructionError::Closed)
            ))
        ),
        "pool did not reject the request with typed construction closure"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn process_final_aggregate_closes_cached_and_fresh_native_http_pools() -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(/*secs*/ 5);
    let listener = timeout_at(deadline, TcpListener::bind((Ipv4Addr::LOCALHOST, 0))).await??;
    let endpoint = Url::parse(&format!(
        "http://{}/constructor-custody",
        listener.local_addr()?
    ))?;
    let policy =
        NetworkPolicy::unmanaged().restrict_to_endpoints(BTreeSet::from([endpoint.clone()]));
    let factory =
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault).with_network_policy(policy);
    let pool = RouteAwareClientPool::new(factory.clone(), ClientRouteClass::Api);
    let scenario = async {
        let response = timeout_at(deadline, pool.get(endpoint.as_str()).send()).await??;
        ensure!(
            response.status().as_u16() == 200,
            "fixture response was unsuccessful"
        );
        let body = timeout_at(deadline, response.text()).await??;
        ensure!(body == "ok", "fixture response body differed");

        let shutdown = PluginStartupProcessShutdown::begin(deadline.into_std());
        let observed = timeout_at(deadline, shutdown.wait_until(deadline.into_std())).await?;
        ensure!(
            observed.is_complete(),
            "aggregate ownership was not complete"
        );
        ensure!(
            observed.curated.clean_native_ownership(),
            "curated native ownership was unclean"
        );
        ensure!(
            observed.constructors.joined == 1,
            "actual constructor accounting differed"
        );
        ensure!(
            observed.constructors.retained == 0
                && observed.constructors.pending == 0
                && observed.constructors.failed == 0,
            "constructor ownership was not clean"
        );

        // Repeated process-final observation must not reopen either pool.
        let repeated = PluginStartupProcessShutdown::begin(deadline.into_std());
        require_closed(&pool, &endpoint, deadline)
            .await
            .context("cached pool")?;
        let fresh = RouteAwareClientPool::new(factory, ClientRouteClass::Api);
        require_closed(&fresh, &endpoint, deadline)
            .await
            .context("fresh pool")?;
        let final_observed = timeout_at(deadline, repeated.wait_until(deadline.into_std())).await?;
        ensure!(
            final_observed.is_complete(),
            "repeated aggregate observation was incomplete"
        );
        ensure!(
            final_observed.constructors == observed.constructors,
            "rejected requests changed constructor accounting"
        );
        Ok::<_, anyhow::Error>(())
    };
    // Both futures and every socket are owned here. No detached test-server
    // task/JoinHandle survives a failed branch or the absolute fixture timeout.
    let (server_result, scenario_result) = timeout_at(deadline, async {
        tokio::join!(serve_once(listener, deadline), scenario)
    })
    .await
    .context("shared five-second composition deadline expired")?;
    server_result.context("owned loopback server failed")?;
    scenario_result
}
