# File-search API — Stage B contract library

This registered support crate has passed focused contract tests. It is not yet
connected to a selected native/process backend or exercised through an installed
search plugin. It is not evidence of completed file-search extraction.

The value types, option defaults and JSON serialization are copied from the
existing `codex-file-search` implementation without changing that crate. Native
re-exports and all consumer migrations belong to the integration step. The only
new reporter method is the source-compatible default `on_error` callback.

The API uses standard-library boxed futures and `serde`; it has no Tokio,
matcher, Core, component-host or history dependency. New domain frames do not
define the component wire format. The component adapter must encode paths
losslessly and enforce logical payload limits before deserialization.

Resource limits have no defaults. `ProviderLimits.resources` is an aggregate
ceiling over retained allocations; `SearchBudget` is one session's allocation.
The runtime must reserve checked sums across Preparing, Ready and Closing work.
`validate_within` checks only a requested allocation against a ceiling and is not
an admission controller. `max_worker_threads` includes all session-owned native
workers, not just the requested matcher parallelism. Byte accounting must cover
owned path/matcher representations and allocated capacity; it is not an RSS cap.
Actual native accounting, bounded queues and limit measurements remain pending.

Close and provider shutdown return `SearchCloseOutcome`, not an outer
`Result`. Its retained `operation` result is separate from `CloseCleanup`:
`Joined` releases the reservation even when the operation failed; `Unconfirmed`
keeps ownership quarantined until separate provider/process cleanup is proven.
Callers must inspect the cleanup receipt before propagating operation failure.
Overall success requires both a successful operation result and joined cleanup.
Forced termination remains unconfirmed rather than becoming a successful join.
Repeated close observers receive the same retained outcome. The explicit
conversion into `StartCleanup` preserves existing startup semantics and cannot
turn an accepted operation into `NotAdmitted`.

Native completion needs deliberate adaptation: `on_complete_tagged` currently
also reports external cancellation, and normal completion may arrive without a
new snapshot when the match set did not change. The adapter must distinguish
normal completion from cancellation and construct a consistent final frame;
it cannot blindly mark a previously incomplete snapshot as `Idle`. Verify equal
patterns, consecutive empty results, completion without a changed match set and
cancellation before the first snapshot during integration.

Tests cover legacy serialization, bounded diagnostics and cleanup receipts,
query identity/byte limits, positive budgets, structural frame/poll checks, and
close-receipt wire decoding with separate failure/cleanup fields. Actual failed
operations, joined cleanup and quota reuse require implementation tests.
They do not establish runtime enforcement, transition legality, authorized-root
membership, worker termination or independent plugin installation.

The governing contract is
[`component-sdk/FILE_SEARCH_COMPONENT_PLAN.md`](../../component-sdk/FILE_SEARCH_COMPONENT_PLAN.md).
