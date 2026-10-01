# P02 Stage B native backend: read-only design audit

Audit date: 2026-10-01. Source checkpoint: `17c366bb94f77b3d3895acd6f061a5cb308bd384` (unverified P02 WIP), derived from upstream `d42056091aded7feb1d88ac7e83972108b2aa478`. These are proposed Stage B changes, not installed-component proof. Read with [FILE_SEARCH_COMPONENT_PLAN.md](FILE_SEARCH_COMPONENT_PLAN.md).

Status: proposed implementation only. No checkout edits, API additions, builds or
runtime tests were performed for this audit. The API directory is unregistered
WIP. Stage A's accepted test evidence remains separate. Paths below are relative
to `/workspace/codex-harness-everythings-a-plugin` unless otherwise noted.

## 1. Current behavior and the actual completion gap

`codex-rs/file-search/src/lib.rs::matcher_worker` emits an update only when
`nucleo.tick().changed` and the snapshot/current pattern atoms agree. It emits
tagged completion whenever matching is not running and walking is complete.
Consequently, a final WalkComplete notification can yield completion without a
new update: the retained snapshot can still have `walk_complete=false`.

This is a Stage B contract adaptation requirement, not demonstrated evidence of
an identical-query Stage A regression. In pinned Nucleo:

- `third-party/nucleo/src/pattern.rs::MultiPattern::reparse` always sets Update
  or Rescore, even when the text is identical.
- `src/lib.rs::Nucleo::tick/tick_inner` starts that replacement while retaining
  the worker lock. The completed worker has `running=true` from `Worker::run`;
  the observing tick updates the snapshot and reports `changed=true` before idle.
- Therefore a settled admitted query gets a current-ID snapshot, including equal
  text and no matches; superseded queries can intentionally receive no completion.
- Current App Server result notifications omit `walk_complete`, so the identified
  final-walk flag gap alone does not prove current presentation behavior regressed.

Current external cancellation also invokes `on_complete_tagged`. Do not infer
successful idle from that callback when an external flag can reach native code.
The API README already records this caveat; no duplicate checkout edit is needed.

## 2. Smallest coherent native adapter

Implement `SearchBackend` and `SearchBackendSession` in a new private native
`file-search/src/backend.rs` module, after registration and value-type re-exports.
Expose only the constructor needed by runtime/plugin composition. Keep this
implementation out of the neutral API and out of Core.

Use one `FileSearchOwner` per selected native backend, shared by all scopes;
scope ownership stays in `file-search-runtime`. Do not recreate a native owner
per connection. Introduce an internal bounded constructor/factory so the backend
can pass `SearchBudget` and a resource ledger into `SessionInner`; retain the
existing legacy constructor's explicit behavior until its callers are migrated.
The storage lookup fallback remains a named non-provider route until its planned
dependency injection is implemented. Do not imply the API alone replaces it.

Always pass `None` as the native external cancellation flag for backend sessions.
`SearchBackend::open` has no external flag. The runtime facade owns and joins its
external-flag observer, which requests private session closure. This avoids
conflating the native legacy external-cancel callback with successful idle and
preserves shared-flag sibling independence without adding a public callback API.

The backend session state should own: accepted query ID/text; one latest complete
frame; checked revision; close/admission fence; one accepted poll slot; retained
typed failure; native lease; and independently retained close/result observation.
Public lease ownership must remain distinct from task ownership so dropping the
last public `Arc` requests close even while backend tasks retain state.

Reserve provider/session ownership before native startup. An observed open error
must wait for accepted cleanup and map its receipt to NotAdmitted, Confirmed or
Unconfirmed. Reuse `FileSearchOwner`'s abandoned-startup ownership; do not replace
it with an untracked `spawn_blocking`. Prevalidate clean input/budget rejection
before native admission so such errors do not poison the owner's cleanup report.

## 3. Final snapshot production, including equal queries and zero matches

Insertion point: `file-search/src/lib.rs::matcher_worker`, the `next_notify`
branch containing the tick, snapshot builder and tagged completion.

Factor the existing snapshot construction into a private helper. Build a fresh
snapshot from the current Nucleo snapshot when either an ordinary matching update
arrives OR normal completion is proven. The latter requires all of:

1. the worker consumed this query ID/text;
2. `!status.running && walk_complete`;
3. current/snapshot pattern atoms agree;
4. neither private shutdown nor external cancellation is observed.

Use the current consumed query ID/text and actual final walk flag. Publish this
snapshot before tagged completion. Do not relabel a previously stored frame.
Equal normalized patterns can legitimately produce equal matches, but their
fresh query identity must still be recorded after the worker processes them.
An inconsistent pattern at alleged idle is an explicit failure, not an empty
success or an indefinite missing-snapshot wait.

The backend reporter stores only frames for its currently accepted generation.
It turns tagged normal completion into Idle only after a matching completed-walk
snapshot exists. Initial state is Running with no snapshot, not fabricated Idle.
Exact duplicate observations may be coalesced without bumping revisions; every
actual replacement uses checked revision increment. Overflow terminates with a
typed failure. A newer accepted query clears the older snapshot, even for a/b/a.

Callbacks take only a brief state lock and wake a retained observer. They do not
await, block on a full channel, acquire owner locks that a join holds, or join
their own session. Publication validates again at delivery against current ID.

## 4. Cancellation, premature termination, close and typed errors

`request_close` synchronously fences all handles before requesting native close.
It wakes polls and may record Cancelled; this is not a joined-cleanup receipt.
After native workers and retained backend operations drain, publish Closed only
for clean completion. Keep a prior Failed state/error observable after drainage.
Never turn resource exhaustion, panic or transport loss into an empty Idle result.

Native failure can end the supervisor without a reporter callback. The backend
must retain a task that observes `ManagedFileSearchSession::is_finished`, then
awaits its consuming close and captures the actual result. A bounded Tokio timer
is an adequate first implementation, as existing App Server one-shot observation
already uses; a private wakeable native completion receipt can later remove that
polling. Do not add one blocking OS observer thread per search.

Important insertion point: `async_owner.rs::Completion`, `OwnerInner::started`,
`launch_close`, and `bounded_failure` currently flatten errors into strings.
The new adapter must preserve a typed first failure (e.g. ResourceExhausted)
before this loss, or evolve the internal retained outcome to carry a cloneable
typed error. Do not recover categories by matching diagnostic strings. Keep
constructor cleanup conservative where `lifecycle::start` itself can fail while
joining; do not blanket-convert constructor error receipts to confirmed cleanup.

The API now proposes `SearchCloseOutcome`, which separates the retained
operation result from `CloseCleanup::Joined` or `Unconfirmed`. Native reservations
can be released after actual joined cleanup even when the operation failed; that
operation failure remains observable. Adapt internal owner completion so it
retains both facts rather than reconstructing a join receipt from an error string.
Never infer joined cleanup from an arbitrary remote error or successful search.
Unknown remote cleanup stays quarantined until separate provider/process cleanup
is proven. Preserve the same retained outcome for repeated close observers.

At most one accepted snapshot poll survives its observing future. Reserve the
slot before spawning/awaiting; retain a bounded wait and clear the slot only when
the owned poll finishes. Close wakes it. A second poll is resource exhaustion.
Updates acknowledge bounded native admission, not matching completion. The query
receipt must echo exactly the admitted ID. No implicit replay or reconnect.

## 5. Native queue bounds are part of this slice

Current `create_session` uses an unbounded channel for QueryUpdated,
NucleoNotify, WalkComplete and Shutdown. An outer actor's one-in-flight update is
insufficient: native enqueue acknowledgment can run far ahead of matching.

Replace the bounded backend path's signal queue with a mutex-protected latest
query slot plus sticky notification/walk-complete/shutdown flags and a capacity-1
nonblocking wake channel. Validate/increment query identity under the same small
admission fence before replacing the latest query. A full wake channel already
represents pending work; never block the reporter or walker on it. The receiver
takes pending work atomically, and concurrent writers still leave a wake/flag.
Test that shutdown cannot be lost behind query or injector notification floods.

Apply explicit query-byte, option/root, result/snapshot and poll-wait limits too.
The current neutral budget intentionally supplies no measured defaults for them;
native/process construction must receive or negotiate a consistent explicit
policy. Index limits do not bound query patterns, output clones or wire spools.

## 6. Actual dedicated worker allocation

For requested `T > 0`, peak session-owned OS workers are:

`1 supervisor + 1 outer walker + T ignore workers + min(T, Rayon maximum) pool workers`.

The supervisor itself runs the matcher loop; there is no extra matcher thread.
Pinned ignore 0.4.25 `WalkParallel::visit` at cached `src/walk.rs:1406–1429`
spawns every traversal worker, including T=1, unless there is no initial work.
Reserve checked `2*T + 2` as a conservative pre-start ceiling. Typical native
TUI T=2 needs six, App Server T=12 needs 26. Reject overflow, insufficient budget
or unsupported parallelism before `lifecycle::start`; never silently clamp.

Use `lifecycle.rs::PoolThreads::build` and its actual joins as the pool ownership
boundary. Keep the allocation through Preparing/Ready/Closing until all native
joins finish, including partial startup failure. Tokio startup/close blocking
jobs use shared runtime capacity; bound them via retained-owner admission rather
than incorrectly counting them as dedicated per-session worker threads.

## 7. Index entry and byte accounting: practical minimum that meets the API

Atomic entry admission belongs in `walker_worker` after existing skip/root/type
decisions but BEFORE `Arc::from(full_path)`, `Utf32String::from`, or Injector push.
One shared ledger atomically reserves both entry count and byte charge with checked
arithmetic. No partial reservation on failure. Concurrent callbacks may finish
already admitted inserts, but cannot reserve after the failure fence. Retain all
successful charges until index destruction; there is no per-query refund.

A path-length-only counter does not satisfy the current byte contract. Relevant
allocations, all source-verified in pinned Nucleo, are:

| Allocation | Exact insertion/accounting point |
| --- | --- |
| Full absolute-path Arc | Native walker before `Arc::from`; include backing header/alignment in the declared charge. |
| Match column | `matcher/src/utf32_str.rs::Utf32String::from`; ASCII byte storage or feature-dependent grapheme-to-char storage, including the CRLF case. Preflight a conservative checked upper bound; account conversion capacity. |
| Entry arena | `src/boxcar.rs::Vec::with_capacity`, `get_or_alloc`, `Entry::layout`. Requested initial 2,048 allocates geometric buckets totaling 4,064 slots. Slots include entry state, column headers and padding. |
| Candidate arrays | `Worker::matches`, `in_flight`, and `Snapshot::update`'s complete match-vector clone. Result limit/top-N does not bound these arrays. Charge capacities. |
| Matcher scratch | `matcher/src/matrix.rs::MatrixSlab::new`, `Layout::new::<MatcherData>()` per pool matcher plus optional native indices matcher. Derive target layout; do not use the approximate source comment as a measurement. |

Recommended minimum: a narrow **bounded-capacity Nucleo construction path** with
preflight allocation charges, reusing the current matcher/scoring algorithm.
Prepay the required arena layout and candidate/snapshot/in-flight capacities for
the chosen entry ceiling, and cap/disable eager allocation beyond the reserved
arena. Ensure `reserve`, Rayon `par_extend` and snapshot `clone_from` cannot exceed
that capacity. Retained arena preallocation also avoids concurrent losing-bucket
allocations in `get_or_alloc`; existing lazy allocation occurs before CAS.
Charge fixed storage and all worker permits before creating the pool/Nucleo.

Reject entry ceilings beyond Nucleo's supported range (`u32::MAX - 32`) before
its panic paths. Use checked target-layout arithmetic and fallible reservations;
do not discover overflow after admission. A bounded constructor may reserve
more memory up front than legacy lazy growth; measure that cost before selecting
defaults, and reject budgets that cannot pay the fixed/minimum allocation.

Native path/column payload then consumes the remaining shared byte allocation
before each injection. If retaining dynamic candidate capacities instead of
preallocation, add allocation-aware reservations in the vendor before every
growth, including temporary old/new storage. Merely checking Vec capacity after
allocation is not enforcement and is not an acceptable shortcut.

On exhaustion, preserve ResourceExhausted, fence query/entry admission, request
private cleanup and join. Do not send successful WalkComplete for a truncated
index. Preserve existing intentional per-entry filesystem-error handling,
UTF-8 path skips, symlinks, deepest-root choice, ignore rules and matching scores.

This remains an explicitly defined index allocation charge, not an RSS cap.
Traversal queues/ignore metadata, allocator overhead, thread stacks, query
patterns, result/highlight buffers, callback clones and shared runtime storage
need their separate bounds/accounting. Preserve MPL notices/provenance and add
these exact vendor symbols to lineage when implementation changes them.

## 8. Required focused tests before integration acceptance

1. Deterministically delay WalkComplete until a matching snapshot is consumed;
   require a fresh final snapshot with `walk_complete=true` before tagged idle,
   even when the following tick has `changed=false`.
2. Sequential identical text with increasing IDs, normalized-equivalent text,
   a/b/a, and consecutive different and identical no-match queries. For each
   non-superseded ID require a valid final current-ID snapshot then Idle; reject
   wrong-pattern intermediate nonempty results. No assertion on first stream
   update being final.
3. Cancel before the first snapshot and during traversal: no fabricated Idle,
   private closure leaves a sibling's external flag untouched, close joins all
   workers, repeated observers get the same retained result.
4. Reporter/matcher/supervisor panic without completion: bounded poll wakes with
   Failed, consuming close joins, and owner shutdown retains the failure.
5. Concurrent injection at entry and byte limits, long Unicode/CRLF paths,
   roots/entries skipped by existing policies, and arithmetic/layout overflow:
   charge before allocation, never exceed the ledger, no partial-index success.
6. Fixed-floor, arena bucket boundary, candidate capacity, indices-matcher and
   worker-budget rejection. Include partial pool/startup failure and abandoned
   startup/close waiters; provider sums retain reservations through actual joins.
7. Query/notification floods while matcher work or a reporter is gated: bounded
   storage, latest ID wins, one retained poll, shutdown remains prompt once gated
   work is released. A cancelled poll waiter cannot admit another concurrent poll.
8. Preserve the existing native traversal/scoring/indices suite; then run actual
   App Server/TUI and separately built plugin conformance against the same backend
   contract. These are future gates, not checks executed by this audit.

Recommended implementation order: native final-snapshot/queue/lifecycle bridge,
then explicitly charged bounded Nucleo construction and entry ledger, then native
backend conformance, then runtime/process composition and independent install.
Do not claim Stage B acceptance after only the API or a path-byte counter.
