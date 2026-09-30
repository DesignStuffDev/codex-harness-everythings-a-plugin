use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_extension_api::ExtensionFuture;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::ThreadLifecycleContributor;
use codex_extension_api::ThreadStopInput;
use codex_protocol::models::BaseInstructions;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::ThreadMemoryMode;
use codex_thread_store::CreateThreadParams;
use codex_thread_store::InMemoryThreadStore;
use codex_thread_store::LiveThread;
use codex_thread_store::LiveThreadInitGuard;
use codex_thread_store::ThreadPersistenceMetadata;
use pretty_assertions::assert_eq;
use tokio::sync::Notify;
use tokio::time::timeout;

use super::runtime_lifecycle::SessionRuntimeOwner;
use super::runtime_lifecycle::shutdown_persistence;
use super::runtime_lifecycle::shutdown_runtime;
use super::runtime_lifecycle::supervise_session_loop;
use super::session::Session;
use super::startup::SessionLaunchGuard;
use super::startup::SessionStartup;
use crate::config::Config;

enum StopBehavior {
    Ready,
    Held(Arc<Notify>),
    Panic,
}

struct StopProbe {
    calls: AtomicUsize,
    entered: Notify,
    behavior: StopBehavior,
}

impl StopProbe {
    fn new(behavior: StopBehavior) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            entered: Notify::new(),
            behavior,
        })
    }
}

impl ThreadLifecycleContributor<Config> for StopProbe {
    fn on_thread_stop<'a>(&'a self, _input: ThreadStopInput<'a>) -> ExtensionFuture<'a, ()> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.entered.notify_one();
            match &self.behavior {
                StopBehavior::Ready => {}
                StopBehavior::Held(release) => release.notified().await,
                StopBehavior::Panic => panic!("synthetic thread-stop panic"),
            }
        })
    }
}

async fn fixture(probes: &[Arc<StopProbe>]) -> (Arc<Session>, Arc<InMemoryThreadStore>) {
    let (mut session, _) = super::tests::make_session_and_context().await;
    let store = Arc::new(InMemoryThreadStore::default());
    let config = session.get_config().await;
    let live = LiveThread::create(
        store.clone(),
        CreateThreadParams {
            creator_user_id: None,
            creator_account_id: None,
            session_id: session.session_id(),
            thread_id: session.thread_id(),
            extra_config: None,
            forked_from_id: None,
            parent_thread_id: None,
            source: SessionSource::Exec,
            thread_source: None,
            originator: "runtime-lifecycle-test".to_owned(),
            base_instructions: BaseInstructions::default(),
            dynamic_tools: Vec::new(),
            selected_capability_roots: Vec::new(),
            multi_agent_version: None,
            history_mode: Default::default(),
            history_base: None,
            subagent_history_start_ordinal: None,
            initial_window_id: uuid::Uuid::new_v4().to_string(),
            runtime_workspace_roots: None,
            metadata: ThreadPersistenceMetadata {
                cwd: Some(config.cwd.to_path_buf()),
                model_provider: config.model_provider_id.clone(),
                memory_mode: ThreadMemoryMode::Disabled,
            },
        },
    )
    .await
    .expect("create owned writer");
    session.services.live_thread = Some(live);
    session.services.thread_store = store.clone();
    let mut extensions = ExtensionRegistryBuilder::new();
    for probe in probes {
        extensions.thread_lifecycle_contributor(probe.clone());
    }
    session.services.extensions = Arc::new(extensions.build());
    (Arc::new(session), store)
}

#[tokio::test]
async fn cancelled_startup_cleanup_waiter_retains_teardown_before_discard() {
    let release = Arc::new(Notify::new());
    let probe = StopProbe::new(StopBehavior::Held(Arc::clone(&release)));
    let (session, store) = fixture(&[Arc::clone(&probe)]).await;
    let startup = Arc::new(SessionStartup::default());
    *startup.persistence.lock().await = LiveThreadInitGuard::new(session.live_thread().cloned());
    let mut launch = SessionLaunchGuard::new(Arc::clone(&startup));
    launch.constructed(Arc::clone(&session));
    let first = tokio::spawn(launch.cleanup());
    timeout(Duration::from_secs(5), probe.entered.notified())
        .await
        .expect("cleanup reached contributor");
    first.abort();
    assert!(first.await.expect_err("cancelled waiter").is_cancelled());

    let mut second = tokio::spawn({
        let startup = Arc::clone(&startup);
        async move { startup.cleanup().await }
    });
    assert!(
        timeout(Duration::from_millis(25), &mut second)
            .await
            .is_err()
    );
    assert_eq!(store.calls().await.discard_thread, 0);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
    release.notify_one();
    timeout(Duration::from_secs(5), second)
        .await
        .expect("retained cleanup completed")
        .expect("cleanup waiter");
    let calls = store.calls().await;
    assert_eq!((calls.discard_thread, calls.shutdown_thread), (1, 0));
    drop(session);
    assert!(
        startup
            .session
            .get()
            .expect("published session")
            .upgrade()
            .is_none()
    );
}

#[tokio::test]
async fn dropping_constructed_launch_owns_cleanup_with_retained_session() {
    let probe = StopProbe::new(StopBehavior::Ready);
    let (session, store) = fixture(&[Arc::clone(&probe)]).await;
    let startup = Arc::new(SessionStartup::default());
    *startup.persistence.lock().await = LiveThreadInitGuard::new(session.live_thread().cloned());
    let mut launch = SessionLaunchGuard::new(Arc::clone(&startup));
    launch.constructed(Arc::clone(&session));
    drop(launch);
    timeout(Duration::from_secs(5), startup.cleanup())
        .await
        .expect("launch cancellation cleaned up");
    assert!(session.state.lock().await.shutting_down);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
    let calls = store.calls().await;
    assert_eq!((calls.discard_thread, calls.shutdown_thread), (1, 0));
}

#[tokio::test]
async fn transferred_owner_uses_normal_shutdown_once_after_waiter_cancellation() {
    let release = Arc::new(Notify::new());
    let probe = StopProbe::new(StopBehavior::Held(Arc::clone(&release)));
    let (session, store) = fixture(&[Arc::clone(&probe)]).await;
    let startup = Arc::new(SessionStartup::default());
    *startup.persistence.lock().await = LiveThreadInitGuard::new(session.live_thread().cloned());
    let mut launch = SessionLaunchGuard::new(startup);
    launch.constructed(Arc::clone(&session));
    let owner = launch.into_runtime_owner().await;
    drop(owner);
    timeout(Duration::from_secs(5), probe.entered.notified())
        .await
        .expect("owner drop started cleanup");
    let wait = shutdown_runtime(&session);
    let first = tokio::spawn(wait.wait());
    first.abort();
    let _ = first.await;
    release.notify_one();
    timeout(Duration::from_secs(5), shutdown_runtime(&session).wait())
        .await
        .expect("runtime close joined")
        .expect("runtime close");
    shutdown_persistence(&session)
        .wait()
        .await
        .expect("writer close");
    shutdown_persistence(&session)
        .wait()
        .await
        .expect("same writer close");
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
    let calls = store.calls().await;
    assert_eq!((calls.discard_thread, calls.shutdown_thread), (0, 1));
}

#[tokio::test]
async fn panicking_contributor_does_not_skip_later_cleanup_or_writer() {
    let panics = StopProbe::new(StopBehavior::Panic);
    let later = StopProbe::new(StopBehavior::Ready);
    let (session, store) = fixture(&[Arc::clone(&panics), Arc::clone(&later)]).await;
    let outcome = SessionRuntimeOwner::new(Arc::clone(&session))
        .close()
        .wait()
        .await;
    assert_eq!(
        outcome,
        Err(Arc::<str>::from(
            "session cleanup stages failed: thread stop contributor"
        ))
    );
    assert_eq!(shutdown_runtime(&session).wait().await, outcome);
    assert_eq!(
        (
            panics.calls.load(Ordering::SeqCst),
            later.calls.load(Ordering::SeqCst)
        ),
        (1, 1)
    );
    assert_eq!(store.calls().await.shutdown_thread, 1);
}

#[tokio::test]
async fn aborted_loop_cleans_up_before_any_termination_waiter_without_arc_cycle() {
    let release = Arc::new(Notify::new());
    let probe = StopProbe::new(StopBehavior::Held(Arc::clone(&release)));
    let (session, store) = fixture(&[Arc::clone(&probe)]).await;
    let weak = Arc::downgrade(&session);
    let task = tokio::spawn(std::future::pending::<()>());
    let abort = task.abort_handle();
    let termination = supervise_session_loop(task, SessionRuntimeOwner::new(Arc::clone(&session)));
    abort.abort();
    timeout(Duration::from_secs(5), probe.entered.notified())
        .await
        .expect("supervisor runs without a completion waiter");
    assert_eq!(store.calls().await.shutdown_thread, 0);
    drop(session);
    release.notify_one();
    timeout(Duration::from_secs(5), termination.clone())
        .await
        .expect("termination includes cleanup");
    assert_eq!(store.calls().await.shutdown_thread, 1);
    assert!(
        weak.upgrade().is_none(),
        "completion must not retain Session"
    );
    termination.await;
}

#[tokio::test]
async fn panicked_loop_still_joins_native_cleanup_and_persistence() {
    let probe = StopProbe::new(StopBehavior::Ready);
    let (session, store) = fixture(&[Arc::clone(&probe)]).await;
    let termination = supervise_session_loop(
        tokio::spawn(async { panic!("synthetic loop panic") }),
        SessionRuntimeOwner::new(Arc::clone(&session)),
    );
    timeout(Duration::from_secs(5), termination)
        .await
        .expect("panic cleanup completed");
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
    let calls = store.calls().await;
    assert_eq!((calls.discard_thread, calls.shutdown_thread), (0, 1));
}
