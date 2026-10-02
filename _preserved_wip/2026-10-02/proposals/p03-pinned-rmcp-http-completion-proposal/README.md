# Pinned RMCP completion proposal — frozen 01

Recovery-only additive changes against cached `rust-sdk` revision
`3e636cab26c013eca5131103c03d20237f12c4df` (workspace `rmcp = 3.3.0`).
`UPSTREAM-PROVENANCE.json` binds every inspected upstream input; the exact
upstream licensing-transition notice and license are preserved as
`UPSTREAM-LICENSE`. The dependency checkout, workspace source, Git state,
lockfiles and caches have not been modified. No Rust command has been run.

This follows lower workspace manifest
`2b587ab16cf31e8798b0f5ceead85f6d1324ecf3ba78694292df0323b4bba2b4`.
That proposal remains valid with HTTP `Unconfirmed`; these SDK additions alone
do not change the workspace classifier or establish process-final MCP cleanup.

## Review units

1. `worker.rs` adds observed close without changing the existing transport API.
   The exact `JoinHandle` remains inside the transport while its mutable borrow
   is awaited. Once joined, the original outer join and inner operation result
   are retained in one immutable `Arc`. A canceled observer can retry the same
   handle. If legacy close consumed it first, observation returns `None`.
2. `streamable_http_client.rs` and the new completion module retain application
   requests, session cleanup metadata and the SSE `JoinSet` outside the fallible
   protocol body. An owner stop races that body, including startup/body waits.
   Normal completion, operation errors and cancellation converge on aborting
   and joining all owned SSE tasks. Recovery also joins its previous stream
   generation. A body panic is rethrown after this cleanup; it is not turned
   into an optional ordinary operation error.
3. New sibling tests exercise actual pinned worker implementations. They are
   authored validation only until root runs the scoped dependency/workspace
   gates. Existing protocol tests remain necessary regression coverage.

Known legacy session IDs are saved before reading initialize bodies in initial
bootstrap, reinitialization and modern-to-legacy fallback. An early parse failure
or canceled body read can therefore attempt DELETE for the last recorded ID.
A cloned worker has a distinct completion identity; cloning its read-only
observer preserves that worker's identity.

## Outcome contract

`WorkerCompletion::result()` preserves the original operation result separately
from the native Tokio join result. `HttpCleanupObserver::snapshot()` is absent
until common cleanup publishes its immutable receipt. A receipt is insufficient
without the exact worker join. Unexpected SSE task failures retain their first
original error and a saturating count; successful and expected aborted tasks are
joined and counted. Ordinary request errors retain their existing protocol path.

`HttpRemoteDeleteOutcome` records deletion of the last known session as deleted,
unsupported, failed with the original error, timed out, or no known session.
It is not a local ownership result. `NoKnownSession` does not prove that an
interrupted remote request created no state. Neither this receipt nor a DELETE
success certifies every historical remote session.

The intended workspace adapter must require both the exact worker join and the
local cleanup receipt. A normal optional protocol/DELETE error must not fail a
successful turn solely for that reason. Missing observation, a worker task
panic/cancellation, or an unexpected owned SSE task failure cannot become clean.
The lower independently retained concrete transport must remain the owner while
observation is pending. This SDK patch alone supplies no independent registry.

## Limits

- No bounded join guarantee: synchronous client construction, destructors,
  native runtime teardown, backend-private tasks and DNS may exceed a caller's
  deadline. Dropping application futures is not universal backend task joining.
- Runtime cancellation or a panic in cleanup/destruction can preempt common
  cleanup. Missing receipt and/or failed exact worker join remain uncertainty
  or failure; they must never synthesize a clean result.
- `RunningService::waiting()` still has its separate private-handle cancellation
  limitation. The lower workspace retained primary task protects ordinary
  observers, but its own runtime cancellation remains an explicit failure.
- No process-wide roster drain, new admission fence, historical-session
  aggregate, executor completion or descendant process-group proof is added.
- Root must decide dependency source/pin integration, update Cargo/Bazel locks,
  adapt the workspace HTTP classifier, and run actual core/MCP/host acceptance.

The existing HTTP protocol body will receive mechanical indentation when root
formats the dependency patch. That movement is separate from the semantic
ownership changes above; it is not evidence of validation.
