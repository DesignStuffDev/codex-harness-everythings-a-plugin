# File-search component — P02a planned, not implemented

This is a source-grounded proposal for the next search extraction in the
[canonical roadmap](../IMPLEMENTATION_ROADMAP.md#p02--native-search-and-workspace-services).
It records a read-only audit and proposed acceptance gates, not implementation or
runtime proof. P01 verification remains separate. Do not count this document, an
existing native crate, or a new adapter as completed extraction.

## Existing behavior and all four consumers

The native implementation is `codex-rs/file-search/src/lib.rs`:

- `create_session(roots, options, reporter, cancel_flag)` creates a reusable
  `FileSearchSession`; `update_query` changes the query without walking again.
- `FileSearchOptions` carries result limit, exclusion patterns, worker count,
  index highlighting and ignore policy.
- `SessionReporter::on_update` receives `FileSearchSnapshot` with query, matches,
  total match count, scanned entry count and walk completion. `on_complete` means
  idle for a query or cancellation; it does not destroy the reusable session.
- `FileMatch` carries score, root, relative path, file/directory kind and optional
  sorted, deduplicated character indices. `run` wraps a session for one-shot use;
  `run_main` additionally owns CLI presentation and the no-pattern directory
  listing behavior.

| Consumer | Actual native path | Required accounting |
| --- | --- | --- |
| App Server | `app-server/src/request_processors/search.rs`: `SearchRequestProcessor`; `app-server/src/fuzzy_file_search.rs`: `run_fuzzy_file_search`, `start_fuzzy_file_search_session` | Replace both one-shot and reusable sessions. Keep existing public RPC compatibility. |
| TUI mentions | `tui/src/app/event_dispatch.rs`: `StartFileSearch` / `FileSearchResult`; `tui/src/file_search.rs`: `FileSearchManager` | Inject the same selected service; this path does not call the App Server fuzzy RPCs. |
| Standalone search executable | `file-search/src/main.rs` -> `run_main` | Preserve flags, output, truncation warnings and no-pattern listing. Resolve component selection at executable composition, not inside the native algorithm. |
| Storage lookup fallback | `rollout/src/list.rs`: `find_thread_path_by_id_str_in_subdir` calls `file_search::run` after filename lookup misses | Keep explicitly pending until a dependency can be injected into the storage process, normally through P03. App Server/TUI replacement does not replace this private storage path. |

App Server currently asks for 50 results and up to 12 workers with indices;
TUI uses the native defaults of 20 results and two workers with indices. The
standalone CLI defaults to 64 results. These are consumer policies, not reasons
for three implementations.

Preserve native matching and traversal behavior: hidden entries, followed
symlinks, file/directory classification, deepest matching root, exclusions,
repository-scoped `.gitignore` behavior, the explicit ignore-disabled mode,
case-insensitive/normalized matching and index highlighting. App Server sorts
score descending then path ascending. Its public result fields include the
existing snake_case `match_type` and `file_name`; session fields use camelCase.
Do not casually rename that wire surface during an internal extraction.

Native traversal currently skips paths that cannot be represented as UTF-8.
The internal component wire must transport roots and paths losslessly using the
existing native-path codec, but that does not establish new native matching
support for non-UTF-8 entries. Keep the public App Server's current display-path
conversion distinct from the trusted same-OS component transport.

## Stage A: native lifetime repair before the process boundary

Make this a separately reviewable and testable change with no extraction claim.

`create_session` currently discards both `std::thread::JoinHandle`s for the walker
and matcher. `FileSearchSession::drop` sets a shutdown flag and enqueues a signal;
it does not join either thread. The walker checks cancellation after 1,024
accepted entries, while some early-return paths do not increment that counter.
Retain explicit worker ownership, provide an idempotent cancellation request and
a joined close operation, wake pending observation, and propagate worker failure.
Check cancellation before starting work and before early-return traversal paths;
do not promise a deadline for an OS filesystem operation that cannot be canceled.

The TUI has a concrete ownership cycle:

```text
SearchState.session -> FileSearchSession.inner -> SessionInner.reporter
    -> TuiSessionReporter.state -> SearchState
```

`FileSearchManager` has no `Drop` cleanup. Empty-query and cwd-reset paths break
the cycle by taking the session, but reconnect replaces the manager with an
active session still retained. Use weak reporter ownership and an explicit
manager lifecycle. Existing `on_user_query` and `update_search_dir` take/drop the
session while holding the same mutex acquired by reporter callbacks. Adding a
blocking join to that drop would deadlock. Take ownership under the mutex,
invalidate the generation, then cancel/close after releasing the mutex. Keep
joins off Tokio/UI executor threads and retain their owner if the waiter leaves.

Joining the two outer threads is not automatically proof that every OS worker
has exited. The pinned Nucleo revision `4253de9faabb4e5c6d81d946a5e35a90f87347ee`
owns a Rayon pool. `Nucleo::drop` signals cancellation and waits up to one second
for its worker lock; pinned `rayon-core` 1.13.0 `ThreadPool::drop` signals eventual
pool termination rather than joining OS threads. Audit and, if needed, expose
owned pool shutdown in that pinned dependency before claiming complete native
thread termination. Clearly distinguish:

- query idle: the current query has settled, and the session remains usable;
- lease quiescence: no owned scan, match computation or callback can act again;
- joined native close: the owned workers have exited with failure propagated;
- process close: the external service was reaped, proving its threads cannot
  survive, while forced termination is reported as forced cleanup.

Stage A tests must cover active manager drop/reconnect, clear/cwd change while a
callback contends for its lock, cancellation before/during a walk, shared-cancel
sibling independence, worker failure, repeated close and actual worker exit.
Existing `cancel_exits_run` only joins its outer test caller; it is not proof that
the native walker/matcher/pool were joined.

## Stage B: version 1 contract and actual presentation integration

Introduce a typed service facade and `file_search:default`, contract version 1.
Native implementation and process adapter implement the same facade. A separate
native executable package uses the existing search algorithm, not an unchanged
Codex executable wrapped as a plugin. Keep interfaces independent of `codex-core`.
The existing native crate depends on `ignore`, pinned `nucleo`, crossbeam, serde,
and CLI/Tokio support; those dependencies remain implementation details.

Resolve a catalog snapshot before constructing any search session. An absent
selection uses the native implementation; an invalid or failed selected service
returns an explicit error rather than silently starting native workers. Active
leases retain their selected package/configuration until closed; changes apply
to a newly composed runtime. Do not promise live replacement or cross-process
index persistence in version 1.

### Proposed methods

Reuse the accepted multiplexed persistent transport. It currently has
request/reply messages, not unsolicited persistent event frames. A bounded
snapshot long-poll avoids adding a second transport during P02a.

| Method | Input | Result / ownership boundary |
| --- | --- | --- |
| `file_search/open` | `contract_version: 1`, host-generated `lease_id`, `session_epoch`, explicit root snapshot and options | Validates version, roots, options and admission before creating work; returns negotiated limits/capabilities and initial revision. |
| `file_search/update_query` | Lease identity, increasing `query_epoch`, query text | Acknowledges acceptance of the latest query. Older/replayed epochs cannot replace newer work. |
| `file_search/next_snapshot` | Lease identity, `after_revision`, bounded `wait_ms` | Latest full snapshot with revision, session/query epochs and running/idle/closed/error state, or a bounded timeout without a fabricated update. |
| `file_search/release` | Lease identity | Reserved control request; idempotently fences admission, cancels accepted work, wakes readers and acknowledges only after the documented joined cleanup boundary. |

Host-generated lease identities must not be reused in the same process session.
Validate `contract_version` in `open` even after manifest validation. Reserve
control capacity before admitting `open`, using `ComponentSession::call_with_cleanup`.
Dropping its result/lease guard queues release without competing for ordinary RPC
capacity. The service owns accepted work independently of request waiters.
Release arriving before an open handler runs must prevent that handler from
creating an orphan session; bound any release-before-open tombstones.

The host owns authority decisions, connection identity, current root generation,
query epochs and presentation state. The worker owns its index, walker/matcher,
pending snapshot state and accepted cleanup. Roots are explicit native values
from the caller's existing authorized context; the worker must not choose an
ambient cwd, infer new roots or broaden authority. Preserve existing symlink
semantics without portraying a lexical root check as an OS sandbox.

Use the existing `codex_component_state_codec::native_path` representation for
trusted same-OS paths, or extract that codec into a small neutral dependency
without changing its encoding. Do not import the whole history API into a search
interface merely to serialize paths.

### Snapshot, connection and generation rules

Every observable snapshot is a complete replacement, carrying query text,
matches, counts, walk status, revision and both epochs. Separate query-idle state
from closed-session state. A newer accepted query supersedes older computation;
coalescing is allowed, but report skipped revisions explicitly and never lose
the latest terminal state. Use one ordered publisher per lease rather than
spawning untracked sends for individual callbacks.

App Server must scope session IDs and cancellation tokens by `ConnectionId`,
target notifications to the owning connection and close its leases on
disconnect. Today the maps and serialization key use bare strings, notifications
broadcast, and connection cleanup omits search. Preserve the public API shape
while enforcing ownership internally. One-shot native errors currently become
successful empty results; the new internal contract must distinguish errors.
Review the public compatibility mapping explicitly rather than using that legacy
behavior to conceal a broken selected implementation.

Fence stale work at delivery, not only before enqueueing. TUI internal results
must carry root/session/query generations: matching query text alone cannot
reject an old result for the same query after cwd navigation. App Server's
already-scheduled sends similarly need ownership/generation checks and ordered
update/completion delivery. A stop response must have a documented fence; do not
leave a grace interval in which obsolete updates are treated as correct.

### Proposed size, resource and failure contract

These are initial version 1 bounds to validate in implementation/conformance
tests, not claims about current enforcement:

- At most 16 retained leases per service, independent of transport RPC slots;
  at most one pending snapshot read per lease. This leaves capacity for query
  updates alongside snapshot polls on the existing 32 ordinary-call slots.
- Snapshot wait at most 1,000 ms. Dropping a poll does not release its search;
  the accepted bounded poll still finishes under service ownership.
- Query UTF-8 payload at most 64 KiB; serialized roots/options at most 256 KiB;
  at most 1,024 requested matches; serialized snapshot at most 1 MiB. Negotiate
  limits explicitly and reject oversized requests before starting work.
  Preserve tested native defaults; do not silently clamp caller options.
- Coalesce pending full snapshots into a bounded latest-value slot, not an
  unbounded callback/event queue. Bound error text and retained tombstones too.
- Configure and negotiate an index resource budget separately from result count.
  Native indexing currently grows with the tree. Exhaustion must produce an
  explicit typed resource error or declared incomplete result, never silently
  mark a partial index complete. Determine its practical entry/byte default from
  measured native behavior before declaring this acceptance gate passed.

Distinguish invalid input, unsupported version/option, unknown/closed lease,
stale epoch, resource exhaustion, search failure, transport loss and forced
shutdown. Preserve native per-entry traversal error handling where intentional;
worker panic or service disconnect is not an empty successful search.

Transport loss invalidates all leases. Do not replay accepted operations or
silently reconnect behind active handles; an explicit new session may rebuild
its volatile index. Canceling an observation waiter does not prove cancellation
of native work. Service shutdown fences admission, releases all leases, wakes
blocked reads, joins owned handlers/workers and only then acknowledges shutdown.
On deadline expiry the process supervisor may force termination/reap, reporting
cleanup failure rather than successful close. No storage-durability guarantee is
created by this read-only search service.

## Registration, packaging and staged coverage

Add the kind/version to `component-api/src/lib.rs` and the supported-kind/version
validation in `component-host/src/catalog.rs`. Add typed native/process facade,
service dispatcher and independent native/custom package templates. Generic
manager `Call` must not bypass persistent lease initialization; add a typed
diagnostic path or reject that use, as storage already does.

Inject selection into App Server `SearchRequestProcessor` and TUI
`FileSearchManager`, including startup/reconnect/cwd lifecycle. This is not an
`ExtensionRegistryBuilder` contributor: search is invoked directly by these
consumers. Compose the standalone executable with the same selection without
creating a native-implementation/process-adapter dependency cycle. Update Cargo
and Bazel declarations consistently when implementation begins.

The desktop plugin presently has no file-search interaction. Add a functional
file search/mention flow using the existing App Server API, showing real selected
results and inserting the chosen file into user input. Package it separately and
exercise its actual UI. An RPC client script or static mock is not GUI acceptance.

The storage fallback is an explicit pending dependency. A separately selected
thread-store process cannot discover/use a host-selected search service through
package dependency ordering alone. Defer its routing until P03 supplies an
authorized broker/injected dependency, or implement and review that narrow
dependency contract first. Native lifetime repairs benefit the fallback now,
but do not make that path independently replaceable. Keep this gap in coverage
reports until tested.

## Acceptance and regression gates

1. **Native lifetime:** pass Stage A tests before process extraction. Prove
   callback quiescence and owned worker termination separately; no timing-only
   assertion that a returned cancellation flag equals joined cleanup.
2. **Native/custom/default parity:** independently build the native package and
   a behavior-changing custom package outside the harness tree. Install/select
   both against unchanged host hashes; exercise real App Server, TUI and
   standalone CLI paths. Reset/remove and recover native defaults without
   recompiling the host. Report the storage fallback as pending until covered.
3. **Matching:** roots/overlap, ignores and exclusions, hidden entries, symlinks,
   Unicode indices, no matches, directories, repository disappearance, limits,
   query updates after idle and original CLI output behavior.
4. **Lifecycle and ordering:** genuine same-token one-shot cancellation,
   abandoned waiters, rapid query changes, repeated release, release/open races,
   manager drop/reconnect, same-query cwd replacement, multi-client same-ID
   isolation, disconnect and full App Server shutdown.
5. **Failure and resources:** malformed/version-mismatched requests, oversized
   inputs/results, retained-lease exhaustion, saturated ordinary admission while
   reserved release succeeds, blocked filesystem work, worker panic, broken
   pipes, no stale notification after the close fence, and process reaping.
6. **Real desktop interaction:** installed native and custom search results
   change the functional GUI and file insertion path. Perform requested manual
   Browser checks when available; report actual cloud Browser/preview limits
   separately and retain available automation/screenshots as distinct evidence.

Retain the native `codex-file-search` suite, the 12 current App Server fuzzy-search
tests and TUI popup/mention snapshots. The existing App Server cancellation-token
test does not cancel an earlier request using that token, and stop quietness
allows a 250 ms grace; neither substitutes for the new lifecycle gates. Run
rollout/thread-store lookup regressions for Stage A and for any later fallback
injection. No checks listed here have been run as part of this planning audit.

## Watchers remain a separate P02b service

Nucleo match notifications are not filesystem-change subscriptions. P02a must
not claim watcher extraction, live reindexing or overflow/rescan behavior merely
because it emits search snapshots.

`file-watcher/src/lib.rs` already provides `FileWatcher`,
`FileWatcherSubscriber::register_paths`, `WatchRegistration` and subscriber-local
receivers. Registration/subscriber drops unregister paths; receivers flush pending
paths before closing. Missing paths use an existing ancestor and move the actual
watch as directories appear. Preserve those semantics in its later contract.

Its Tokio event-loop handle is currently discarded, its raw event channel and
accumulated changed-path set are unbounded, and its drop test proves release of
the inner watcher rather than joined task termination. P02b needs separate
subscription ownership, overflow/rescan, bounded delivery and joined shutdown
work, with the authority/config prerequisites named in the roadmap. Git/worktree
services remain separate as well.
