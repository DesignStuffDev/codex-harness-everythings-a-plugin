# Pinned HTTP completion follow-on (design only)

The workspace pins modelcontextprotocol/rust-sdk 3e636cab26c013eca5131103c03d20237f12c4df, rmcp=3.3.0. This inspection uses those installed sources, not online SDK documentation. No dependency patch, checkout, Cargo lock, or toolchain change is included here.

## Current evidence and operational meaning

WorkerTransport::close at transport/worker.rs:405 takes its JoinHandle, joins it, then discards the inner WorkerQuitReason. A cancelled close observer can also drop that exact handle. WorkerContext and WorkerSendRequest contain private fields and are non-exhaustive, so a workspace wrapper cannot correctly reconstruct their generic context to interpose on the unchanged worker.

StreamableHttpClientWorker::run has early error returns and question-mark propagation after session creation that can bypass its common cleanup block. Normal cleanup at streamable_http_client.rs:1764 cancels request futures and aborts SSE tasks, but does not join the SSE JoinSet. DELETE error and timeout are logged rather than returned at1798. Exposing WorkerQuitReason alone therefore would not establish full local task completion.

Unconfirmed must describe missing completion evidence, separately from optional MCP operation failure. It must not be silently upgraded to clean, nor must every ordinary successful turn automatically become an error solely because the SDK lacks an observation API. The whole-host promotion gate remains blocked on unconfirmed obligations. This is an explicit interim compatibility limitation, not an acceptable permanent v1 cleanup solution.

## Smallest coherent additive SDK adapter change to review next

1. Add an observation API to the pinned WorkerTransport that preserves the original terminal worker result and exact task through cancelled observers. Keep the existing transport surface compatible. The workspace retained transport owner remains primary and must never call a timeout API that consumes and drops the handle. A narrow take-exact-worker-ownership adapter may be preferable to replicating the entire transport.
2. Restructure only StreamableHttpClientWorker::run cleanup so all exits after resource acquisition converge on owned cleanup. Preserve the original operation result. Cancel/drop application POST futures, abort the existing SSE JoinSet, then drain every exact SSE task. Expected cancellation and known join panic must remain distinct.
3. Record two independent outcomes: local owned-task completion and remote protocol-session deletion. Unsupported DELETE is normal protocol behavior. Remote DELETE failure/timeout is diagnostics/evidence, not proof that a local task still runs and not automatically an ordinary turn failure. Never report remote cleanup confirmed when it failed.
4. The workspace RmcpClient roster observes every generation's service and transport receipt. Late initialization attaches to the closed roster and is signalled/drained. The host's MCP aggregate must report unknown/pending/failed obligations before resources vanish, separately from curated and session-loop receipts. Embedded owner replacement drains its own scope and must not close a process-global registry.
5. Do not claim joining application HTTP tasks joins all reqwest runtime/DNS/background operations, or proves arbitrary process-group completeness. Maintain those exact scope limitations.

## Required tests before promotion

Use the pinned real worker and bounded loopback fixture, not a substitute simplified worker. Hold an accepted body/SSE task; cancel one close observer, retry, release and observe the same exact completion. Force a worker operation error after session acquisition and assert common cleanup still drains SSE tasks. Force SSE-task panic, requested cancellation, DELETE failure, unsupported DELETE and DELETE timeout; assert the local ownership result and original operation result do not overwrite each other. Include normal optional offline startup/operation failure so a successful user operation is not spuriously converted into a shutdown error. The full host test must reject absent/incomplete MCP receipts even if command exit is zero.

This design is unimplemented and untested. Any pinned SDK adapter revision requires separate review, dependency provenance, the normal Cargo/Bazel lock workflow, and root execution after the active Rust gate.
