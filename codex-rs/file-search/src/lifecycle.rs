//! Owns every native search worker until its actual thread has joined.

use crate::FileSearchSession;
use crate::SessionInner;
use crate::WorkSignal;
use crate::matcher_worker;
use crate::walker_worker;
use crossbeam_channel::Receiver;
use crossbeam_channel::bounded;
use nucleo::Config;
use nucleo::Nucleo;
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
) -> anyhow::Result<FileSearchSession> {
    let (ready_tx, ready_rx) = bounded(1);
    let (finished_tx, finished) = bounded(1);
    let worker_inner = inner.clone();
    let supervisor = thread::Builder::new()
        .name("file-search supervisor".into())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                supervise(worker_inner, work_rx, overrides, ready_tx)
            }))
            .unwrap_or_else(|_| Err(anyhow::anyhow!("file-search supervisor panicked")));
            let _ = finished_tx.send(());
            result
        })?;
    let session = FileSearchSession {
        inner,
        supervisor: Some(supervisor),
        finished,
    };
    if ready_rx.recv().is_err() {
        session.close()?;
        anyhow::bail!("file-search supervisor exited before becoming ready");
    }
    Ok(session)
}

fn supervise(
    inner: Arc<SessionInner>,
    work_rx: Receiver<()>,
    overrides: Option<ignore::overrides::Override>,
    ready: crossbeam_channel::Sender<()>,
) -> anyhow::Result<()> {
    let mut pool_threads = PoolThreads::default();
    let pool_panicked = Arc::new(AtomicBool::new(false));
    let panic_flag = pool_panicked.clone();
    let panic_inner = inner.clone();
    let pool = pool_threads.build(
        inner.threads,
        move |_| {
            // Rayon catches asynchronous task panics. Record them separately from
            // OS-thread joins and do not let the handler itself panic or block.
            panic_flag.store(true, Ordering::Release);
            panic_inner.shutdown.store(true, Ordering::Release);
            let _ = panic_inner.work_tx.send(WorkSignal::Shutdown);
        },
        spawn_pool_thread,
    )?;
    let notify_inner = inner.clone();
    let notify = Arc::new(move || {
        if !notify_inner.shutdown.load(Ordering::Acquire) {
            let _ = notify_inner.work_tx.send(WorkSignal::NucleoNotify);
        }
    });
    let mut nucleo = Nucleo::new_with_thread_pool(
        Config::DEFAULT.match_paths(),
        notify,
        pool,
        /*columns*/ 1,
    );
    let injector = nucleo.injector();
    let walker_inner = inner.clone();
    let walker = thread::Builder::new()
        .name("file-search walker".into())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                walker_worker(walker_inner.clone(), overrides, injector);
            }));
            if result.is_err() {
                walker_inner.shutdown.store(true, Ordering::Release);
                let _ = walker_inner.work_tx.send(WorkSignal::Shutdown);
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
                matcher_worker(inner.clone(), work_rx, &mut nucleo)
            })) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => failures.push(error.to_string()),
                Err(_) => failures.push("file-search matcher or reporter panicked".into()),
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
    if failures.is_empty() {
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
