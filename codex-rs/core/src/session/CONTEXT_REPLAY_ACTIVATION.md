# Replay lifecycle activation checkpoint

`component_replay.rs` is staged and unlinked. No runtime behavior or test result
is claimed. Active core/workspace edits were paused for the shared state-codec
storage fidelity fix; the corresponding unapplied core patch is not yet complete.

The intended seam is the actual `Session::new` constructor and
`apply_rollout_reconstruction`, retaining the original reducer as the native
default and differential oracle. The existing immutable catalog in `Session::new`
already respects `SessionIsolation`; select `context_replay/default` from that
snapshot only. No mutable global selection or callback into core is needed.

Further review identified a surrounding startup-cleanup requirement: replay runs
after SessionConfigured, MCP installation and prewarm startup. A selected replay
error must close that already-created Session runtime as well as the replay
connection before returning. The current constructor error arm only discards live
persistence; the existing SessionStartup cleanup holder is supplied by managed
starts, which are isolated. An ordinary selected start needs explicit retained
constructed-session ownership and idempotent full runtime cleanup. This additional
shared seam is not implemented or approved for activation yet; do not treat the
staged ReplayOwner as ownership of every native service.

The staged helper creates a nonclone `ReplayOwner` synchronously before awaiting
process acquisition. A separately supervised task owns acquisition and cleanup.
Cancellation of constructor readiness drops the owner, revokes admission and
signals that task even when an early-published Session Arc is retained. Successful
startup must transfer the owner out of `Session::new` and into the submission-loop
task. It must remain owned across startup warnings and every await before that
transfer. The Session service Arc owns call access, not terminal lifetime.

`Session::new` must return `(Arc<Session>, Option<ReplayOwner>)`. Constructor error
cleanup explicitly closes the owner before discarding pending persistence; dropping
that cleanup future still leaves supervised cleanup owned. Loop termination must
await replay cleanup, including after loop panic or cancellation. Test constructors
must explicitly account for the returned owner rather than silently drop it.

Selected reconstruction receives the complete owned `ReplayInput`, including
resolved native history, review and truncation policies. Errors propagate through
`apply_rollout_reconstruction -> record_initial_history -> Session::new`. Neither
fallback nor partial initialization is permitted. Native calls retain the current
algorithm. Every existing test caller must handle the new Result explicitly.

The helper creates a host-only ticket with ThreadId, SessionId and random epoch.
The final state commit holds SessionState and the helper's ownership gate together,
checks `shutting_down`, validates the ticket, then publishes all companion state
without awaits. Any later estimated-prefill update repeats this validation. The
lock order is SessionState then ownership gate; invalidation never acquires them
in reverse. `shutdown_session_runtime` must mark state as shutting down and call
`begin_close` under that state lock before any teardown await. This serializes
revocation with history commit even when service Arcs remain retained.

## Prerequisite before runtime activation

Persistent transport currently reports some failed handshakes or forced shutdowns
before the ChildGuard's detached reap finishes. One-shot forced reap and normal
persistent close have separate existing evidence; that does not prove forced
persistent cleanup. The transport owner must make bounded kill-and-reap completion
part of persistent acquisition/termination before replay startup cleanup can claim
to be fully joined. Do not work around this by dropping a connect future.

Required tests include native differential reconstruction; selected startup failure;
cancellation during acquisition/reconstruction; late responses after shutdown;
retained Session/adapter Arcs; constructor owner transfer; loop cancellation;
complete companion-state atomic commit; no fallback; isolation; independent build,
install and invocation with an unchanged host hash. Lossless codec tests and native
storage parity are prerequisites. Live context mutation/epoch/recovery ownership
remains a distinct future component contract.
