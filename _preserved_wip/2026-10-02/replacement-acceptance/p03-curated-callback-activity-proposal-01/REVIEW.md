# Non-closing curated callback activity proposal

Recovery-only; unadopted, uncompiled, unrun. Apply after p03-curated-lifecycle-observation-proposal-01. Its startup_sync.rs preimage is that unit's exact proposed output; callback_scope.rs preimage is the current root checkout. No dependency/lock/schema changes. Apache LICENSE/NOTICE retained.

CuratedCallbackScope::activity() reads exact retained task-handle readiness and known records without closing admission, initializing a registry, polling/taking/joining handles, dispatching work or exposing payloads. The single scope lock keeps its record snapshot coherent; the global sticky closing fence is read atomically. The upgraded registry Arc drops after the scope mutex guard.

Finished_unjoined is deliberately not success: a panic/cancellation/suppression can produce the same readiness. Existing final scope waits must still join and report the original outcome. admission_closed identifies explicit local/global closure; failure/unexpected records can separately reject admission, so clients must also inspect those counters. No whole-host/MCP receipt follows.

Two proposed behavioral tests use actual tasks: repeated observation preserves exact handle identity and allows a second real dispatch, then explicit final observation joins both; a real panic is only finished-unjoined until final observation identifies failure and preserves rejected admission. Neither test has run.

For production A-to-B acceptance, first observe exact native generation success and handle finished, then B activity with exactly one finished-unjoined record, no running/awaiting/joining/known-failure/unexpected record and no explicit closure. This establishes the callback body is no longer executing without assuming its outcome. Observe typed MCP/skill and the already-active-turn hook behavior; final existing scope wait must yield completed=1, failed=0. A's closed scope must retain completed=0. The native finished observation does not join and may not include final thread teardown; final process-owner join remains mandatory.

Root validation after adoption/rebase: ordinary strict just test -p codex-core-plugins --lib, scoped to startup_sync::callback_scope::activity::tests for the narrow first check, followed by the agreed combined gate and lint/format. Do not run while another Rust/cache writer is active.
