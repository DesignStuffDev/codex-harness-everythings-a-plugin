//! Prepaid native index storage; reservations survive until the index owner drops.

use crate::FileSearchOptions;
use crate::IndexedEntry;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use nucleo::IndexAllocationPlan;
use nucleo::Matcher;
use std::sync::Arc;
use std::sync::Mutex;

pub(super) struct NativeBudget {
    limits: SearchBudget,
    state: Mutex<Reservations>,
}

struct Reservations {
    entries: usize,
    bytes: usize,
    failure: Option<SearchError>,
}

impl NativeBudget {
    /// Checks all fixed storage and dedicated OS workers before native startup.
    pub fn prepare(
        options: &FileSearchOptions,
        budget: SearchBudget,
    ) -> Result<(IndexAllocationPlan<IndexedEntry>, Arc<Self>), SearchError> {
        // One supervisor, one outer walker, T ignore workers and at most T
        // Rayon workers. Shared Tokio jobs are bounded by their owning runtime.
        let workers = options
            .threads
            .get()
            .checked_mul(2)
            .and_then(|workers| workers.checked_add(2))
            .ok_or_else(|| {
                SearchError::new(
                    SearchErrorKind::ResourceExhausted,
                    "native file-search worker allocation overflow",
                )
            })?;
        if workers > budget.max_worker_threads.get() {
            return Err(SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "native file-search worker budget exhausted",
            ));
        }
        let plan =
            IndexAllocationPlan::new(budget.max_index_entries, options.threads).map_err(|_| {
                SearchError::new(
                    SearchErrorKind::ResourceExhausted,
                    "native file-search fixed allocation is unsupported",
                )
            })?;
        let scratch = if options.compute_indices {
            Matcher::scratch_allocation_bytes()
        } else {
            0
        };
        let bytes = plan.charged_bytes().checked_add(scratch).ok_or_else(|| {
            SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "native file-search fixed allocation overflow",
            )
        })?;
        if bytes > budget.max_index_bytes.get() {
            return Err(SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "native file-search byte budget cannot cover fixed index storage",
            ));
        }
        Ok((
            plan,
            Arc::new(Self {
                limits: budget,
                state: Mutex::new(Reservations {
                    entries: 0,
                    bytes,
                    failure: None,
                }),
            }),
        ))
    }

    /// Atomically reserves one entry and its prepared path/column payload charge.
    /// Failed admission changes neither counter and permanently fences admission.
    pub fn reserve(&self, payload_bytes: usize) -> Result<(), SearchError> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(error) = &state.failure {
            return Err(error.clone());
        }
        let entries = state.entries.checked_add(1);
        let bytes = state.bytes.checked_add(payload_bytes);
        let message = if entries.is_none_or(|entries| entries > self.limits.max_index_entries.get())
        {
            Some("native file-search index entry budget exhausted")
        } else if bytes.is_none_or(|bytes| bytes > self.limits.max_index_bytes.get()) {
            Some("native file-search index byte budget exhausted")
        } else {
            None
        };
        if let Some(message) = message {
            let error = SearchError::new(SearchErrorKind::ResourceExhausted, message);
            state.failure = Some(error.clone());
            return Err(error);
        }
        // Both checked values passed their ceilings under this same lock.
        if let (Some(entries), Some(bytes)) = (entries, bytes) {
            state.entries = entries;
            state.bytes = bytes;
            Ok(())
        } else {
            unreachable!("checked reservation must retain both values");
        }
    }
}

#[cfg(test)]
#[path = "native_budget_tests.rs"]
mod tests;
