//! Single-column, prepaid index storage for Codex's bounded native backend.
//!
//! Charges describe requested allocations on the pinned Rust Global allocator,
//! not RSS. Paths/column payload, parsed queries, traversal, thread stacks and
//! Rayon runtime bookkeeping require separate admission by the enclosing owner.

use std::alloc::Layout;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;

use parking_lot::Mutex;
use rayon::ThreadPool;

use crate::pattern::{CaseMatching, MultiPattern, Normalization, Pattern};
use crate::worker::Worker;
use crate::{boxcar, Config, Match, Matcher, Nucleo, Snapshot, State, Status, Utf32String};

/// Fixed storage admission or allocation failure; no candidate was truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityError {
    InvalidCapacity,
    ArithmeticOverflow,
    AllocationFailed,
    EntryLimit,
    PlanMismatch,
}

impl std::fmt::Display for CapacityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidCapacity => "unsupported bounded index capacity",
            Self::ArithmeticOverflow => "bounded index allocation layout overflow",
            Self::AllocationFailed => "bounded index allocation failed",
            Self::EntryLimit => "bounded index entry limit reached",
            Self::PlanMismatch => "bounded index allocation plan does not match construction",
        })
    }
}

impl std::error::Error for CapacityError {}

/// Checked target-layout plan, bound to the exact item type and worker count.
/// Construct and reserve its charge before starting the dedicated worker pool.
///
/// Fixed charge includes full arena buckets, candidate/snapshot/in-flight
/// buffers, matcher values and scratch slabs, pattern-column headers and the
/// owned Arc allocations. It excludes item/column payloads and query atoms.
/// Additional highlighting matchers cost [`Matcher::scratch_allocation_bytes`].
pub struct IndexAllocationPlan<T: Send + Sync + 'static> {
    entries: u32,
    workers: usize,
    bytes: usize,
    item: PhantomData<fn() -> T>,
}

impl<T: Send + Sync + 'static> IndexAllocationPlan<T> {
    pub fn new(entries: NonZeroUsize, workers: NonZeroUsize) -> Result<Self, CapacityError> {
        let entries: u32 = entries
            .get()
            .try_into()
            .map_err(|_| CapacityError::InvalidCapacity)?;
        let workers = workers.get();
        if workers > rayon::max_num_threads() {
            return Err(CapacityError::InvalidCapacity);
        }
        let count = entries as usize;
        let charges = [
            boxcar::Vec::<T>::fixed_allocation_bytes(entries)?,
            array_bytes::<Match>(count)?,
            array_bytes::<Match>(count)?,
            array_bytes::<u32>(count)?,
            array_bytes::<UnsafeCell<Matcher>>(workers)?,
            Matcher::scratch_allocation_bytes()
                .checked_mul(workers)
                .ok_or(CapacityError::ArithmeticOverflow)?,
            array_bytes::<(Pattern, crate::pattern::Status)>(3)?,
            arc_bytes::<boxcar::Vec<T>>()?,
            arc_bytes::<Mutex<Worker<T>>>()?,
            arc_bytes::<AtomicBool>()?,
            arc_bytes::<AtomicBool>()?,
        ];
        let bytes = charges.into_iter().try_fold(0usize, |total, charge| {
            total
                .checked_add(charge)
                .ok_or(CapacityError::ArithmeticOverflow)
        })?;
        Ok(Self {
            entries,
            workers,
            bytes,
            item: PhantomData,
        })
    }

    pub fn entry_limit(&self) -> u32 {
        self.entries
    }
    pub fn worker_count(&self) -> usize {
        self.workers
    }
    pub fn charged_bytes(&self) -> usize {
        self.bytes
    }
}

fn array_bytes<T>(count: usize) -> Result<usize, CapacityError> {
    Layout::array::<T>(count)
        .map(|layout| layout.size())
        .map_err(|_| CapacityError::ArithmeticOverflow)
}

fn arc_bytes<T>() -> Result<usize, CapacityError> {
    // Rust 1.95 ArcInner is repr(C): two atomic counters followed by its data.
    // This is a pinned-toolchain accounting assumption, not a stable Rust ABI.
    Layout::new::<[AtomicUsize; 2]>()
        .extend(Layout::new::<T>())
        .map(|(layout, _)| layout.pad_to_align().size())
        .map_err(|_| CapacityError::ArithmeticOverflow)
}

pub(crate) fn try_buffer<T>(capacity: usize) -> Result<Vec<T>, CapacityError> {
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(capacity)
        .map_err(|_| CapacityError::AllocationFailed)?;
    // Fresh exact allocations record exactly the requested capacity with pinned
    // Rust 1.95 Global. Do not silently adopt a different allocator's accounting.
    if std::mem::size_of::<T>() != 0 && buffer.capacity() != capacity {
        return Err(CapacityError::PlanMismatch);
    }
    Ok(buffer)
}

pub(crate) fn assert_prepaid_capacity<T>(
    buffer: &Vec<T>,
    capacity: Option<u32>,
    additional: usize,
) {
    if let Some(capacity) = capacity {
        let capacity = capacity as usize;
        assert!(
            buffer.capacity() >= capacity,
            "missing prepaid index buffer"
        );
        assert!(
            buffer
                .len()
                .checked_add(additional)
                .is_some_and(|len| len <= capacity),
            "bounded index invariant would grow a prepaid buffer"
        );
    }
}

/// A single immutable index generation. The restricted API intentionally omits
/// restart, unchecked injection and mutable access to the legacy matcher.
///
/// The owner must stop and join all producers before shutdown and retain every
/// pool OS-thread handle until joined. A constructor error drops the supplied
/// pool but does not itself join those handles. Stable Arc allocations retain
/// Rust's ordinary allocation-failure behavior; this is not universal OOM recovery.
pub struct BoundedNucleo<T: Send + Sync + 'static> {
    inner: Nucleo<T>,
    plan: IndexAllocationPlan<T>,
}

impl<T: Send + Sync + 'static> BoundedNucleo<T> {
    pub fn try_new_with_thread_pool(
        config: Config,
        notify: Arc<dyn Fn() + Send + Sync>,
        pool: ThreadPool,
        plan: IndexAllocationPlan<T>,
    ) -> Result<Self, CapacityError> {
        if pool.current_num_threads() != plan.workers {
            return Err(CapacityError::PlanMismatch);
        }
        let worker = Worker::try_new_bounded(plan.workers, config, notify.clone(), plan.entries)?;
        let pattern = MultiPattern::try_single()?;
        let snapshot = Snapshot {
            matches: try_buffer(plan.entries as usize)?,
            pattern: MultiPattern::try_single()?,
            item_count: 0,
            items: worker.items.clone(),
        };
        let inner = Nucleo {
            canceled: worker.canceled.clone(),
            should_notify: worker.should_notify.clone(),
            items: worker.items.clone(),
            worker: Arc::new(Mutex::new(worker)),
            pool,
            state: State::Init,
            notify,
            snapshot,
            pattern,
        };
        Ok(Self { inner, plan })
    }

    pub fn allocation_plan(&self) -> &IndexAllocationPlan<T> {
        &self.plan
    }

    pub fn injector(&self) -> BoundedInjector<T> {
        BoundedInjector {
            items: self.inner.items.clone(),
            notify: self.inner.notify.clone(),
        }
    }

    /// The caller must enforce its separate query-byte budget before parsing.
    pub fn reparse(
        &mut self,
        query: &str,
        case: CaseMatching,
        normalization: Normalization,
        append: bool,
    ) {
        self.inner
            .pattern
            .reparse(0, query, case, normalization, append);
    }

    pub fn pattern(&self) -> &MultiPattern {
        &self.inner.pattern
    }
    pub fn tick(&mut self, timeout: u64) -> Status {
        self.inner.tick(timeout)
    }
    pub fn snapshot(&self) -> &Snapshot<T> {
        self.inner.snapshot()
    }
    pub fn shutdown(self) {
        self.inner.shutdown();
    }
}

/// Cloneable producer for one prepaid, single-column index generation.
/// Its caller must reserve item/column payload bytes before constructing them.
pub struct BoundedInjector<T> {
    items: Arc<boxcar::Vec<T>>,
    notify: Arc<dyn Fn() + Send + Sync>,
}

impl<T> Clone for BoundedInjector<T> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            notify: self.notify.clone(),
        }
    }
}

impl<T> BoundedInjector<T> {
    /// Reserves one entry before moving already prepared payloads. Rejected
    /// values are dropped and do not advance the visible index count.
    pub fn try_push(&self, value: T, column: Utf32String) -> Result<u32, CapacityError> {
        let index = self.items.try_push_single(value, column)?;
        (self.notify)();
        Ok(index)
    }

    pub fn injected_items(&self) -> u32 {
        self.items.count()
    }
}

#[cfg(test)]
#[path = "bounded_tests.rs"]
mod tests;
