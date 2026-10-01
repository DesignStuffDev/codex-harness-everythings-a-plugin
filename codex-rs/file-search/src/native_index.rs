//! Native storage selection; bounded owners never expose legacy restart/injection.

use crate::IndexedEntry;
use crate::MatchType;
use crate::SessionInner;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use nucleo::BoundedInjector;
use nucleo::BoundedNucleo;
use nucleo::CapacityError;
use nucleo::Config;
use nucleo::IndexAllocationPlan;
use nucleo::Injector;
use nucleo::Nucleo;
use nucleo::Snapshot;
use nucleo::Status;
use nucleo::Utf32String;
use nucleo::pattern::CaseMatching;
use nucleo::pattern::MultiPattern;
use nucleo::pattern::Normalization;
use std::sync::Arc;

pub(super) enum NativeAllocation {
    Legacy,
    Bounded(IndexAllocationPlan<IndexedEntry>),
}

pub(super) enum NativeIndex {
    Legacy(Nucleo<IndexedEntry>),
    Bounded(BoundedNucleo<IndexedEntry>),
}

/// Only matching operations shared by the legacy and prepaid immutable index.
/// No consumer can obtain mutable legacy state or restart a bounded generation.
pub(super) trait NativeMatcher {
    fn reparse(&mut self, query: &str, append: bool);
    fn pattern(&self) -> &MultiPattern;
    fn tick(&mut self, timeout: u64) -> Status;
    fn snapshot(&self) -> &Snapshot<IndexedEntry>;
}

impl NativeMatcher for Nucleo<IndexedEntry> {
    fn reparse(&mut self, query: &str, append: bool) {
        self.pattern
            .reparse(0, query, CaseMatching::Ignore, Normalization::Smart, append);
    }
    fn pattern(&self) -> &MultiPattern {
        &self.pattern
    }
    fn tick(&mut self, timeout: u64) -> Status {
        Nucleo::tick(self, timeout)
    }
    fn snapshot(&self) -> &Snapshot<IndexedEntry> {
        Nucleo::snapshot(self)
    }
}

impl NativeMatcher for BoundedNucleo<IndexedEntry> {
    fn reparse(&mut self, query: &str, append: bool) {
        BoundedNucleo::reparse(
            self,
            query,
            CaseMatching::Ignore,
            Normalization::Smart,
            append,
        );
    }
    fn pattern(&self) -> &MultiPattern {
        BoundedNucleo::pattern(self)
    }
    fn tick(&mut self, timeout: u64) -> Status {
        BoundedNucleo::tick(self, timeout)
    }
    fn snapshot(&self) -> &Snapshot<IndexedEntry> {
        BoundedNucleo::snapshot(self)
    }
}

impl NativeIndex {
    pub fn create(
        pool: rayon::ThreadPool,
        notify: Arc<dyn Fn() + Send + Sync>,
        allocation: NativeAllocation,
    ) -> Result<Self, SearchError> {
        let config = Config::DEFAULT.match_paths();
        match allocation {
            NativeAllocation::Legacy => Ok(Self::Legacy(Nucleo::new_with_thread_pool(
                config, notify, pool, /*columns*/ 1,
            ))),
            NativeAllocation::Bounded(plan) => {
                BoundedNucleo::try_new_with_thread_pool(config, notify, pool, plan)
                    .map(Self::Bounded)
                    .map_err(|error| {
                        let kind = if error == CapacityError::PlanMismatch {
                            SearchErrorKind::SearchFailed
                        } else {
                            SearchErrorKind::ResourceExhausted
                        };
                        SearchError::new(kind, error.to_string())
                    })
            }
        }
    }

    pub fn matcher(&mut self) -> &mut dyn NativeMatcher {
        match self {
            Self::Legacy(index) => index,
            Self::Bounded(index) => index,
        }
    }

    pub fn injector(&self) -> EntryInjector {
        match self {
            Self::Legacy(index) => EntryInjector::Legacy(index.injector()),
            Self::Bounded(index) => EntryInjector::Bounded(index.injector()),
        }
    }

    pub fn shutdown(self) {
        match self {
            Self::Legacy(index) => index.shutdown(),
            Self::Bounded(index) => index.shutdown(),
        }
    }
}

#[derive(Clone)]
pub(super) enum EntryInjector {
    Legacy(Injector<IndexedEntry>),
    Bounded(BoundedInjector<IndexedEntry>),
}

impl EntryInjector {
    pub fn push(
        &self,
        inner: &SessionInner,
        full_path: &str,
        relative_path: &str,
        match_type: MatchType,
    ) -> Result<(), SearchError> {
        match self {
            Self::Legacy(injector) => {
                injector.push(
                    IndexedEntry {
                        full_path: Box::from(full_path),
                        match_type,
                    },
                    |_, columns| {
                        columns[0] = Utf32String::from(relative_path);
                    },
                );
                Ok(())
            }
            Self::Bounded(injector) => {
                let column = Utf32String::allocation_plan(relative_path).map_err(|error| {
                    SearchError::new(SearchErrorKind::ResourceExhausted, error.to_string())
                })?;
                let bytes = full_path
                    .len()
                    .checked_add(column.charged_bytes())
                    .ok_or_else(|| {
                        SearchError::new(
                            SearchErrorKind::ResourceExhausted,
                            "native entry payload size overflow",
                        )
                    })?;
                let budget = inner.budget.as_ref().ok_or_else(|| {
                    SearchError::new(
                        SearchErrorKind::SearchFailed,
                        "bounded index has no retained budget",
                    )
                })?;
                budget.reserve(bytes)?;
                let mut path = String::new();
                path.try_reserve_exact(full_path.len()).map_err(|_| {
                    SearchError::new(
                        SearchErrorKind::ResourceExhausted,
                        "native path allocation failed",
                    )
                })?;
                if path.capacity() != full_path.len() {
                    return Err(SearchError::new(
                        SearchErrorKind::SearchFailed,
                        "native path allocator differs from its exact charge",
                    ));
                }
                path.push_str(full_path);
                let column = column.allocate().map_err(|error| {
                    SearchError::new(SearchErrorKind::ResourceExhausted, error.to_string())
                })?;
                injector
                    .try_push(
                        IndexedEntry {
                            full_path: path.into_boxed_str(),
                            match_type,
                        },
                        column,
                    )
                    .map(|_| ())
                    .map_err(|error| {
                        SearchError::new(SearchErrorKind::ResourceExhausted, error.to_string())
                    })
            }
        }
    }
}
