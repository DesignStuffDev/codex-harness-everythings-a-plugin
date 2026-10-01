//! Owns every native search worker until its actual thread has joined.

use crate::FileSearchSession;
use crate::SessionInner;
use crate::WorkSignal;
use crate::matcher_worker;
use crate::native_index::NativeAllocation;
use crate::native_index::NativeIndex;
use crate::walker_worker;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use crossbeam_channel::Receiver;
use crossbeam_channel::bounded;
use std::io;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::JoinHandle;

pub(super) fn start(
    inner: Arc<SessionInner>,
    work_rx: Receiver<()>,
    overrides: Option<ignore::overrides::Override>,
    allocation: NativeAllocation,
) -> Result<FileSearchSession, SearchStartError> {
    let (ready_tx, ready_rx) = bounded(1);
    let (finished_tx, finished) = bounded(1);
    let worker_inner = inner.clone();
    let supervisor = thread::Builder::new()
        .name("file-search supervisor".into())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                supervise(
                    worker_inner.clone(),
                    work_rx,
                    overrides,
                    ready_tx,
                    allocation,
                )
            }))
            .unwrap_or_else(|_| Err(anyhow::anyhow!("file-search supervisor panicked")));
            // Publish exactly one retained terminal failure, after every native
            // worker has joined. No error callback runs under a budget/queue lock.
            if let Err(error) = &result {
                let failure = worker_inner.first_failure().unwrap_or_else(|| {
                    error
                        .downcast_ref::<SearchError>()
                        .cloned()
                        .unwrap_or_else(|| {
                            SearchError::new(SearchErrorKind::SearchFailed, error.to_string())
                        })
                });
                worker_inner.fail(failure.clone());
                let _ = catch_unwind(AssertUnwindSafe(|| {
                    worker_inner.reporter.on_error(&failure)
                }));
            }
            let _ = finished_tx.send(());
            result
        })
        .map_err(|error| SearchStartError {
            operation: SearchError::new(
                SearchErrorKind::SearchFailed,
                format!("failed to start file-search supervisor: {error}"),
            ),
            cleanup: StartCleanup::NotAdmitted,
        })?;
    let session = FileSearchSession {
        inner,
        supervisor: Some(supervisor),
        finished,
    };
    if ready_rx.recv().is_err() {
        let outcome = session.close_outcome();
        return Err(SearchStartError {
            operation: outcome.operation.err().unwrap_or_else(|| {
                SearchError::new(
                    SearchErrorKind::SearchFailed,
                    "file-search supervisor exited before becoming ready",
                )
            }),
            cleanup: outcome.cleanup.into(),
        });
    }
    Ok(session)
}

fn supervise(
    inner: Arc<SessionInner>,
    work_rx: Receiver<()>,
    overrides: Option<ignore::overrides::Override>,
    ready: crossbeam_channel::Sender<()>,
    allocation: NativeAllocation,
) -> anyhow::Result<()> {
    let mut pool_threads = PoolThreads::default();
    let pool_panicked = Arc::new(AtomicBool::new(false));
    let panic_flag = pool_panicked.clone();
    let panic_inner = inner.clone();
    let pool = pool_threads.build(
        inner.threads,
        move |_| {
            // Rayon catches asynchronous task panics. Record them separately from
            // OS-thread joins. Retain the failure and wake the supervisor;
            // never invoke callbacks or join from this Rayon worker.
            panic_flag.store(true, Ordering::Release);
            panic_inner.fail(SearchError::new(
                SearchErrorKind::SearchFailed,
                "file-search Rayon task panicked",
            ));
        },
        spawn_pool_thread,
    )?;
    let notify_inner = inner.clone();
    let notify = Arc::new(move || {
        if !notify_inner.shutdown.load(Ordering::Acquire) {
            let _ = notify_inner.work_tx.send(WorkSignal::NucleoNotify);
        }
    });
    let mut nucleo = NativeIndex::create(pool, notify, allocation)?;
    let injector = nucleo.injector();
    let walker_inner = inner.clone();
    let walker = thread::Builder::new()
        .name("file-search walker".into())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                walker_worker(walker_inner.clone(), overrides, injector);
            }));
            if result.is_err() {
                walker_inner.fail(SearchError::new(
                    SearchErrorKind::SearchFailed,
                    "file-search walker panicked",
                ));
            }
            result.map_err(|_| anyhow::anyhow!("file-search walker panicked"))
        });
    let mut failures = Vec::new();
    match walker {
        Ok(walker) => {
            let mut walker = WalkerThread {
                handle: Some(walker),
                inner: inner.clone(),
            };
            if ready.send(()).is_err() {
                inner.shutdown.store(true, Ordering::Release);
            }
            match catch_unwind(AssertUnwindSafe(|| {
                matcher_worker(inner.clone(), work_rx, nucleo.matcher())
            })) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    inner.fail(
                        error
                            .downcast_ref::<SearchError>()
                            .cloned()
                            .unwrap_or_else(|| {
                                SearchError::new(SearchErrorKind::SearchFailed, error.to_string())
                            }),
                    );
                }
                Err(_) => inner.fail(SearchError::new(
                    SearchErrorKind::SearchFailed,
                    "file-search matcher or reporter panicked",
                )),
            }
            inner.shutdown.store(true, Ordering::Release);
            // ignore::WalkParallel joins its scoped children before returning.
            // Stop every injector before draining and destroying the matcher.
            if let Err(error) = walker.join() {
                failures.push(error.to_string());
            }
        }
        Err(error) => failures.push(format!("failed to start file-search walker: {error}")),
    }
    inner.shutdown.store(true, Ordering::Release);
    if catch_unwind(AssertUnwindSafe(|| nucleo.shutdown())).is_err() {
        failures.push("file-search matcher shutdown panicked".into());
    }
    failures.extend(pool_threads.join());
    if pool_panicked.load(Ordering::Acquire) {
        failures.push("file-search Rayon task panicked".into());
    }
    if let Some(error) = inner.first_failure() {
        Err(anyhow::Error::new(error))
    } else if failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(failures.join("; "))
    }
}

/// Retains traversal even if the supervisor unexpectedly unwinds during cleanup.
struct WalkerThread {
    handle: Option<JoinHandle<anyhow::Result<()>>>,
    inner: Arc<SessionInner>,
}

impl WalkerThread {
    fn join(&mut self) -> anyhow::Result<()> {
        match self.handle.take() {
            Some(handle) => handle
                .join()
                .map_err(|_| anyhow::anyhow!("file-search walker supervisor panicked"))?,
            None => Ok(()),
        }
    }
}

impl Drop for WalkerThread {
    fn drop(&mut self) {
        self.inner.shutdown.store(true, Ordering::Release);
        let _ = self.join();
    }
}

/// Pool destruction requests termination; these handles establish actual exit.
/// Declare this owner before the pool so unwinding drops the pool before joining.
#[derive(Default)]
struct PoolThreads {
    handles: Vec<JoinHandle<()>>,
}

impl PoolThreads {
    fn build(
        &mut self,
        count: usize,
        panic_handler: impl Fn(Box<dyn std::any::Any + Send>) + Send + Sync + 'static,
        mut spawn: impl FnMut(rayon::ThreadBuilder) -> io::Result<JoinHandle<()>>,
    ) -> anyhow::Result<rayon::ThreadPool> {
        // Rayon terminates a partially built registry before build returns or
        // unwinds. Its surviving threads can only be joined after that point.
        let result = catch_unwind(AssertUnwindSafe(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(count)
                .thread_name(|index| format!("file-search matcher {index}"))
                .panic_handler(panic_handler)
                .spawn_handler(|worker| {
                    self.handles.push(spawn(worker)?);
                    Ok(())
                })
                .build()
        }));
        match result {
            Ok(Ok(pool)) => Ok(pool),
            Ok(Err(error)) => {
                let mut failures = vec![format!("failed to start file-search pool: {error}")];
                failures.extend(self.join());
                anyhow::bail!(failures.join("; "))
            }
            Err(_) => {
                let mut failures = vec!["file-search pool startup panicked".into()];
                failures.extend(self.join());
                anyhow::bail!(failures.join("; "))
            }
        }
    }

    fn join(&mut self) -> Vec<String> {
        let mut failures = Vec::new();
        for handle in self.handles.drain(..) {
            if handle.join().is_err() {
                failures.push("file-search pool thread panicked".into());
            }
        }
        failures
    }
}

impl Drop for PoolThreads {
    fn drop(&mut self) {
        // Fallback cleanup also joins after an unexpected supervisor unwind.
        // Such an unwind is reported by the supervisor rather than as success.
        let _ = self.join();
    }
}

fn spawn_pool_thread(worker: rayon::ThreadBuilder) -> io::Result<JoinHandle<()>> {
    let mut builder = thread::Builder::new();
    if let Some(name) = worker.name() {
        builder = builder.name(name.to_owned());
    }
    if let Some(size) = worker.stack_size() {
        builder = builder.stack_size(size);
    }
    builder.spawn(move || worker.run())
}

#[cfg(test)]
#[path = "pool_lifecycle_tests.rs"]
mod tests;
