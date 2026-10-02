use super::*;
use crate::HttpConstructionObservation;
use crate::constructor_custody::Registry;
use pretty_assertions::assert_eq;

fn isolated_pool(registry: &Arc<Registry>) -> RouteAwareClientPool {
    let mut pool = RouteAwareClientPool::new(
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        ClientRouteClass::Api,
    );
    pool.construction_lease = PoolLease::with_registry(Arc::clone(registry));
    pool
}

#[tokio::test]
async fn native_constructor_returns_its_joined_result_and_populates_the_route_cache() {
    let registry = Registry::new(/*capacity*/ 4);
    let pool = isolated_pool(&registry);
    let (route, client, backend) = pool
        .client_for_url_with_resolver("http://constructor.test/", |_| async {
            Ok(OutboundProxyRoute::Direct)
        })
        .await
        .unwrap();
    assert_eq!(
        (route, backend),
        (
            OutboundProxyRoute::Direct,
            SelectedTlsBackend::TransportDefault
        )
    );
    assert!(
        pool.clients
            .lock()
            .unwrap()
            .contains_key(&OutboundProxyRoute::Direct)
    );
    drop(client);
    assert_eq!(
        registry
            .wait_until(std::time::Instant::now() + Duration::from_secs(/*secs*/ 5))
            .await,
        HttpConstructionObservation {
            retained: 0,
            pending: 0,
            failed: 0,
            joined: 1,
            cancelled: 0,
            recovery_attempts: 0,
        }
    );
}

#[test]
fn canceled_native_constructor_publishes_only_while_a_public_peer_survives() {
    enum PublicLifetime {
        PeerRetained,
        LastLeaseDropped,
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(/*val*/ 1)
        .build()
        .unwrap();
    runtime.block_on(async {
        for lifetime in [
            PublicLifetime::PeerRetained,
            PublicLifetime::LastLeaseDropped,
        ] {
            let registry = Registry::new(/*capacity*/ 4);
            let pool = isolated_pool(&registry);
            let peer = match lifetime {
                PublicLifetime::PeerRetained => Some(pool.clone()),
                PublicLifetime::LastLeaseDropped => None,
            };
            let clients = Arc::clone(&pool.clients);
            let build = Arc::clone(&pool.client_build);
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
            let blocker = tokio::task::spawn_blocking(move || {
                started_tx.send(()).unwrap();
                release_rx
                    .recv_timeout(Duration::from_secs(/*secs*/ 5))
                    .unwrap();
            });
            tokio::time::timeout(Duration::from_secs(/*secs*/ 5), started_rx)
                .await
                .unwrap()
                .unwrap();
            let mut waiting = Box::pin(
                pool.client_for_url_with_resolver("http://constructor.test/", |_| async {
                    Ok(OutboundProxyRoute::Direct)
                }),
            );
            let construction = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(std::future::Future::poll(waiting.as_mut(), cx))
            })
            .await;
            assert!(construction.is_pending());
            assert!(build.try_lock().is_err());
            drop(waiting);
            drop(pool);
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            let completed = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), build.lock())
                .await
                .unwrap();
            assert_eq!(
                clients
                    .lock()
                    .unwrap()
                    .contains_key(&OutboundProxyRoute::Direct),
                peer.is_some()
            );
            drop(completed);
            assert_eq!(
                registry
                    .wait_until(std::time::Instant::now() + Duration::from_secs(/*secs*/ 5))
                    .await,
                HttpConstructionObservation {
                    retained: 0,
                    pending: 0,
                    failed: 0,
                    joined: 1,
                    cancelled: 0,
                    recovery_attempts: 0,
                }
            );
        }
    });
}

#[test]
fn process_close_rejects_cached_use_and_queued_native_publication() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(/*val*/ 1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let registry = Registry::new(/*capacity*/ 4);
        let pool = isolated_pool(&registry);
        let (_, client, _) = pool
            .client_for_url_with_resolver("http://constructor.test/", |_| async {
                Ok(OutboundProxyRoute::Direct)
            })
            .await
            .unwrap();
        drop(client);
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let blocker = tokio::task::spawn_blocking(move || {
            started_tx.send(()).unwrap();
            release_rx
                .recv_timeout(Duration::from_secs(/*secs*/ 5))
                .unwrap();
        });
        tokio::time::timeout(Duration::from_secs(/*secs*/ 5), started_rx)
            .await
            .unwrap()
            .unwrap();
        let mut waiting = Box::pin(
            pool.client_for_url_with_resolver("http://constructor.test/", |_| async {
                Ok(OutboundProxyRoute::TransportDefault)
            }),
        );
        let construction = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(std::future::Future::poll(waiting.as_mut(), cx))
        })
        .await;
        assert!(construction.is_pending());
        assert!(pool.client_build.try_lock().is_err());
        registry.begin_close(std::time::Instant::now() + Duration::from_secs(/*secs*/ 5));
        let cached_error = pool
            .client_for_url_with_resolver("http://constructor.test/", |_| async {
                Ok(OutboundProxyRoute::Direct)
            })
            .await
            .unwrap_err();
        assert!(matches!(
            cached_error,
            RouteAwareClientPoolError::Construction(_)
        ));
        assert_eq!(
            RouteAwareRequestError::Route(cached_error).failure_class(),
            None
        );
        release_tx.send(()).unwrap();
        blocker.await.unwrap();
        let queued_error = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), waiting)
            .await
            .unwrap()
            .unwrap_err();
        assert!(matches!(
            queued_error,
            RouteAwareClientPoolError::Construction(_)
        ));
        {
            let clients = pool.clients.lock().unwrap();
            assert_eq!(
                (
                    clients.contains_key(&OutboundProxyRoute::Direct),
                    clients.contains_key(&OutboundProxyRoute::TransportDefault),
                ),
                (true, false)
            );
        }
        assert_eq!(
            registry
                .wait_until(std::time::Instant::now() + Duration::from_secs(/*secs*/ 5))
                .await,
            HttpConstructionObservation {
                retained: 0,
                pending: 0,
                failed: 0,
                joined: 2,
                cancelled: 0,
                recovery_attempts: 0,
            }
        );
    });
}
