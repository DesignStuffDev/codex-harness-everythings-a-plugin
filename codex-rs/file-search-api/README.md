# File-search API — neutral contracts

This crate began as an isolated Stage B support library. That historical
support-only status has been superseded by native/process backend integration:
see the accepted [selected-search evidence](../../verification/2026-10-01/p02b-selected-search-results.json)
and [TUI/GUI consumer evidence](../../verification/2026-10-01/p02b-tui-consumer-results.json).
Those records distinguish real installed-worker proof from focused API tests;
neither claims completed whole-harness extraction. Per-start Preparing
cancellation remains the follow-on integration described below.

The value types, option defaults and JSON serialization originate in the native
`codex-file-search` implementation and are now shared through neutral re-exports.
The reporter's `on_error` callback has a source-compatible default implementation.

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
Native accounting, bounded ownership and measured allocation floors are now
covered by [native-budget evidence](../../verification/2026-10-01/p02b-native-budget-results.json)
and the selected-search gates above. Those resource ceilings are not throughput
benchmarks or RSS guarantees.

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

Native completion still requires deliberate mapping: `on_complete_tagged`
also reports external cancellation, and completion can arrive without a new
snapshot when matches are unchanged. Backend adapters must preserve the tested
distinction between normal completion and cancellation, and cannot mark an
incomplete snapshot `Idle`. Equal patterns, empty results and completion without
a changed match set remain regression cases when changing those adapters.

Tests cover legacy serialization, bounded diagnostics and cleanup receipts,
query identity/byte limits, positive budgets, structural frame/poll checks, and
close-receipt wire decoding with separate failure/cleanup fields. Actual failed
operations, joined cleanup and quota reuse require implementation tests.
They do not establish runtime enforcement, transition legality, authorized-root
membership, worker termination or independent plugin installation.

The governing contract is
[`component-sdk/FILE_SEARCH_COMPONENT_PLAN.md`](../../component-sdk/FILE_SEARCH_COMPONENT_PLAN.md).

Rust source-contract revision **2** requires
`SearchBackend::begin_open(SearchOpen) -> Result<PendingSearchStart, SearchStartError>`.
`open` remains a convenience observer over that required method. It reserves on
call, and an unpolled dropped observer cancels the same start. Custom Rust backend
implementers must migrate; this is not source-compatible with revision 1. The
installed JSON wire contract remains version 1 while its schema is unchanged.

`PendingSearchStart` and `SearchStartControl` define one-start cancellation authority.
An external backend constructs a ticket only after retaining the actual owner;
the ticket requests cancellation when abandoned and transfers terminal results
unchanged. `finish()` preserves that guard even before its first poll. Controls
must provide bounded, idempotent, infallible intent and independently retained
cleanup receipts. The ticket does not implement backend cleanup, exactly-once
join or capacity release. Its public integration tests prove carrier ownership
and handoff behavior only; actual Preparing cancellation remains an integration
step for native/process backends, runtime scopes and their consumers.
