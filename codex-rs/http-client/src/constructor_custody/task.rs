//! Primary handle/result records shared by native, cleanup and disposal work.
//! Recovery may replace only positively joined unit/cancellation outcomes.

use super::lock;
use std::any::Any;
use std::future::Future;
use std::future::poll_fn;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use tokio::sync::Semaphore;
use tokio::task::JoinError;
use tokio::task::JoinHandle;

pub(super) enum Record<T> {
    AwaitingHandle,
    Running(JoinHandle<T>),
    Joined(Option<Result<T, JoinError>>),
    SpawnFailed { _payload: Box<dyn Any + Send> },
}

pub(super) struct Task<T> {
    pub(super) record: Mutex<Record<T>>,
    pub(super) poller: Semaphore,
    failed: AtomicBool,
    pub(super) cancelled: AtomicBool,
    pub(super) cancellations: AtomicUsize,
    pub(super) attachment_waker: futures::task::AtomicWaker,
}

impl<T> Task<T> {
    pub(super) fn new() -> Self {
        Self {
            record: Mutex::new(Record::AwaitingHandle),
            poller: Semaphore::new(/*permits*/ 1),
            failed: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            cancellations: AtomicUsize::new(0),
            attachment_waker: futures::task::AtomicWaker::new(),
        }
    }

    pub(super) fn attach(&self, spawned: std::thread::Result<JoinHandle<T>>) {
        *lock(&self.record) = match spawned {
            Ok(handle) => Record::Running(handle),
            Err(payload) => {
                self.failed.store(true, Ordering::Release);
                Record::SpawnFailed { _payload: payload }
            }
        };
        self.attachment_waker.wake();
    }

    pub(super) fn poll_join(&self, cx: &mut Context<'_>) -> Poll<()> {
        let mut record = lock(&self.record);
        match &mut *record {
            Record::Running(handle) => match Pin::new(handle).poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(result) => {
                    self.failed.store(result.is_err(), Ordering::Release);
                    let cancelled = result.as_ref().is_err_and(JoinError::is_cancelled);
                    self.cancelled.store(cancelled, Ordering::Release);
                    if cancelled {
                        self.cancellations.fetch_add(1, Ordering::AcqRel);
                    }
                    *record = Record::Joined(Some(result));
                }
            },
            Record::AwaitingHandle => {
                self.attachment_waker.register(cx.waker());
                return Poll::Pending;
            }
            Record::Joined(_) | Record::SpawnFailed { .. } => {}
        }
        Poll::Ready(())
    }

    pub(super) async fn join(&self) {
        let Ok(_permit) = self.poller.acquire().await else {
            return;
        };
        poll_fn(|cx| self.poll_join(cx)).await;
    }

    pub(super) fn try_join(&self) {
        let Ok(_permit) = self.poller.try_acquire() else {
            return;
        };
        let finished = match &*lock(&self.record) {
            Record::Running(handle) => handle.is_finished(),
            Record::AwaitingHandle | Record::Joined(_) | Record::SpawnFailed { .. } => false,
        };
        if finished {
            let mut cx = Context::from_waker(futures::task::noop_waker_ref());
            let _ = self.poll_join(&mut cx);
        }
    }

    pub(super) fn take(&self) -> Option<Result<T, JoinError>> {
        match &mut *lock(&self.record) {
            Record::Joined(result) => result.take(),
            Record::AwaitingHandle | Record::Running(_) | Record::SpawnFailed { .. } => None,
        }
    }

    pub(super) fn joined(&self) -> bool {
        matches!(&*lock(&self.record), Record::Joined(_))
    }
    pub(super) fn idle(&self) -> bool {
        matches!(&*lock(&self.record), Record::AwaitingHandle)
    }
    pub(super) fn uncertain(&self) -> bool {
        self.failed.load(Ordering::Acquire) && !self.cancelled.load(Ordering::Acquire)
    }
}

impl Task<()> {
    pub(super) fn succeeded(&self) -> bool {
        matches!(&*lock(&self.record), Record::Joined(Some(Ok(()))))
    }
    /// Only a joined cancellation or successful unit result can be retired.
    /// Panics, failed admission and live handles remain in their original record.
    pub(super) fn prepare_retry(&self) -> bool {
        let Ok(_permit) = self.poller.try_acquire() else {
            return false;
        };
        let mut record = lock(&self.record);
        let recoverable = match &*record {
            Record::Joined(Some(Ok(()))) => true,
            Record::Joined(Some(Err(error))) => error.is_cancelled(),
            Record::AwaitingHandle
            | Record::Running(_)
            | Record::Joined(None)
            | Record::SpawnFailed { .. } => false,
        };
        if recoverable {
            *record = Record::AwaitingHandle;
            self.failed.store(false, Ordering::Release);
            self.cancelled.store(false, Ordering::Release);
        }
        recoverable
    }
}
