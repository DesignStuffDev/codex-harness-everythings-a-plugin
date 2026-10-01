use crate::FileSearchOptions;
use crate::FileSearchSnapshot;
use crate::PendingSearchStart;
use crate::SearchBudget;
use crate::SearchCloseOutcome;
use crate::SearchError;
use crate::SearchErrorKind;
use crate::SearchStartError;
use std::future::Future;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub type SearchFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, SearchError>> + Send + 'a>>;

pub type SearchStartFuture<'a> = Pin<
    Box<dyn Future<Output = Result<Arc<dyn SearchBackendSession>, SearchStartError>> + Send + 'a>,
>;

/// Close/shutdown always returns both the operation result and cleanup evidence.
/// Transport loss is represented by an unconfirmed cleanup receipt, never an
/// outer error that would discard one of the two outcomes.
pub type SearchCloseFuture<'a> = Pin<Box<dyn Future<Output = SearchCloseOutcome> + Send + 'a>>;

/// Explicit authorized roots, existing matching policy, and resource allocation.
/// A backend must not choose an ambient cwd or silently broaden these roots.
#[derive(Debug, Clone)]
pub struct SearchOpen {
    pub roots: Vec<PathBuf>,
    pub options: FileSearchOptions,
    pub budget: SearchBudget,
}

/// Positive identity plus exact user text; an empty query remains valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub id: NonZeroU64,
    pub text: String,
}

impl SearchQuery {
    /// Checks identity and UTF-8 byte size before bounded admission.
    /// The owner must serialize this check with advancing its last admitted ID.
    /// Query normalization must not be used to replace identity or size checks.
    pub fn validate_after(
        &self,
        previous_id: u64,
        max_query_bytes: NonZeroUsize,
    ) -> Result<(), SearchError> {
        if self.id.get() <= previous_id {
            return Err(SearchError::new(
                SearchErrorKind::StaleEpoch,
                "file-search query identity must strictly increase",
            ));
        }
        if self.text.len() > max_query_bytes.get() {
            return Err(SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "file-search query exceeds the UTF-8 byte limit",
            ));
        }
        Ok(())
    }
}

/// Backend admission receipt; this does not mean matching has completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryAccepted {
    pub id: NonZeroU64,
}

/// A complete replacement state, never a delta or a relabeled previous query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchFrame {
    /// Strictly increasing session-local revision. Overflow closes with failure;
    /// identities must never wrap or be reused within a session.
    pub revision: u64,
    /// Zero is reserved for the initial state, before a query is admitted.
    pub query_id: u64,
    pub query: String,
    pub snapshot: Option<FileSearchSnapshot>,
    pub phase: SearchPhase,
}

impl SearchFrame {
    /// Validates identity and idle-state coherence at a backend boundary.
    /// Monotonic revisions, epochs and negotiated payload limits require the
    /// receiving session's state and must additionally be checked there.
    pub fn validate(&self) -> Result<(), SearchError> {
        if self.query_id == 0 && !self.query.is_empty() {
            return Err(SearchError::new(
                SearchErrorKind::InvalidInput,
                "file-search initial identity cannot carry a submitted query",
            ));
        }
        if let Some(snapshot) = &self.snapshot {
            if snapshot.query_id != self.query_id || snapshot.query != self.query {
                return Err(SearchError::new(
                    SearchErrorKind::InvalidInput,
                    "file-search snapshot identity or text does not match its frame",
                ));
            }
            if snapshot.matches.len() > snapshot.total_match_count
                || snapshot.total_match_count > snapshot.scanned_file_count
            {
                return Err(SearchError::new(
                    SearchErrorKind::InvalidInput,
                    "file-search snapshot counts are inconsistent",
                ));
            }
            if self.phase == SearchPhase::Idle && !snapshot.walk_complete {
                return Err(SearchError::new(
                    SearchErrorKind::InvalidInput,
                    "idle file-search frame requires a completed walk",
                ));
            }
        } else if self.phase == SearchPhase::Idle {
            return Err(SearchError::new(
                SearchErrorKind::InvalidInput,
                "idle file-search frame requires a current-query snapshot",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchPhase {
    Running,
    /// Settled query with a matching snapshot and completed walk; session remains reusable.
    Idle,
    /// Cancellation observed; owned work may still require joined cleanup.
    Cancelled,
    /// Joined cleanup was confirmed; no worker or callback can act again.
    Closed,
    /// Operation failure, with joined cleanup still observed separately.
    Failed(SearchError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchPoll {
    Changed(SearchFrame),
    /// Bounded wait expired without a revision newer than the supplied cursor.
    Unchanged {
        revision: u64,
    },
}

impl SearchPoll {
    /// Validates a reply against the cursor used for its retained poll.
    /// Gaps in changed revisions explicitly represent coalesced replacements.
    pub fn validate_after(&self, after_revision: u64) -> Result<(), SearchError> {
        match self {
            Self::Changed(frame) => {
                frame.validate()?;
                if frame.revision <= after_revision {
                    return Err(SearchError::new(
                        SearchErrorKind::StaleEpoch,
                        "changed file-search reply did not advance its revision",
                    ));
                }
            }
            Self::Unchanged { revision } => {
                if *revision != after_revision {
                    return Err(SearchError::new(
                        SearchErrorKind::StaleEpoch,
                        "unchanged file-search reply has a different revision",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Owns one selected implementation and its accepted sessions and cleanup.
///
/// Native and process backends implement the same object-safe contract. Before
/// accepting work, reserve bounded ownership including any paired release. A
/// dropped startup waiter requests release but must not orphan accepted work.
/// An observed startup failure includes an explicit cleanup receipt. Transport
/// loss invalidates leases; never silently reconnect or replay accepted work.
/// Backend selection/configuration stays fixed for this provider's lifetime.
pub trait SearchBackend: Send + Sync {
    /// Reserve one bounded start and return its cancellation authority immediately.
    ///
    /// Retain ownership before publishing the ticket. Cancellation must reach
    /// actual Preparing work, survive observer abandonment and target only this
    /// lease. Do not implement this by wrapping an uncancellable startup future.
    /// Reject synchronously with NotAdmitted when no work was accepted.
    fn begin_open(&self, request: SearchOpen) -> Result<PendingSearchStart, SearchStartError>;

    /// Convenience observer over the required owned start. Admission occurs on
    /// this call; dropping the returned future before polling requests release.
    fn open(&self, request: SearchOpen) -> SearchStartFuture<'_> {
        match self.begin_open(request) {
            Ok(pending) => pending.finish(),
            Err(error) => Box::pin(async move { Err(error) }),
        }
    }

    /// Nonblocking, idempotent admission fence and cleanup request for all work.
    fn request_shutdown(&self);

    /// Fences admission and drains sessions, retained handlers and owned workers.
    /// The owner survives dropped waiters; repeated observers see the retained
    /// outcome. Report a retained operation failure independently of joined
    /// cleanup. Forced termination or lost cleanup proof is Unconfirmed, never
    /// a successful join, and must not erase an earlier operation failure.
    fn shutdown(&self) -> SearchCloseFuture<'_>;
}

/// Shared session lease with global close semantics across cloned `Arc` handles.
///
/// Accepted work belongs to the backend, not an observing future. Dropping the
/// last public lease requests close; internal task references must not suppress
/// that action. A scope closes its own leases without closing sibling scopes or
/// the backend. Implementations must avoid locks across callbacks, waits or joins.
pub trait SearchBackendSession: Send + Sync {
    /// Acknowledges actual backend admission of a strictly newer positive ID.
    /// The bounded latest-query slot may coalesce superseded work; the receipt
    /// must echo this exact ID and does not promise a snapshot for skipped queries.
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted>;

    /// Returns the latest full replacement after the supplied revision or a
    /// bounded timeout. Reject a future cursor and waits above negotiated limits.
    /// Keep at most one accepted poll per lease through completion, even if its
    /// waiter drops; a second concurrent poll is resource exhaustion. Closing
    /// wakes polls, retains terminal state, and never relabels cancellation idle.
    fn next_snapshot(&self, after_revision: u64, wait: Duration) -> SearchFuture<'_, SearchPoll>;

    /// Nonblocking, idempotent fence affecting all handles to this session only.
    /// It must not mutate an external cancellation flag shared with siblings.
    fn request_close(&self);

    /// Requests close and observes cleanup of workers, callbacks and retained
    /// operations. Retain cleanup if the waiter drops and the same outcome for
    /// repeated observers. Joined acknowledges actual drainage, independently
    /// of a retained operation failure. Release the lease reservation on Joined
    /// and still propagate that operation failure through close and shutdown.
    /// Unconfirmed retains ownership until separate provider/process recovery;
    /// query idle, cancellation, queued release and forced termination cannot
    /// be relabeled Joined. Never erase either error to report overall success.
    fn close(&self) -> SearchCloseFuture<'_>;
}
