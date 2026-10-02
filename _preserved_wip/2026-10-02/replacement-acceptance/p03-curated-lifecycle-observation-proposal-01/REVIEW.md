# Curated lifecycle observation and production replacement seam

Status: recovery-only statically reviewed proposal; no primary checkout edit, Rust/Git invocation, host launch or acceptance run. This is a production diagnostic API prerequisite, not completed A-to-B acceptance. The original production plan remains immutable at ../p03-curated-replacement-production-acceptance-plan/PLAN.md.

## Established existing routes

- `app-server/src/in_process.rs` starts the actual `MessageProcessor` with `PluginStartupConfig::Current`; a child module can retain each private handle's existing `CuratedCallbackGuard::scope()` before local shutdown consumes it.
- Normal local A shutdown closes only A's callback scope. B can subscribe/replay the same lexical absolute home through the actual manager startup path. A fresh child test process is required for each case because worker admission/success is intentionally process-global.
- Trusted Git continues to preserve child-scoped `GIT_CONFIG_GLOBAL`. Its automatic-command policy strips repository-local config injection and selects its own trusted repository. A narrow ordinary global `insteadOf` rule for only the actual production URL can point to an owned loopback smart-HTTP Git fixture. No PATH wrapper, URL override, proxy/DNS/TLS weakening or host launch has been used here.
- Scope totals are public only through `wait`/`wait_until`, both of which close admission. A closed A rejects a later dispatch before creating a record: require A completed=0, not suppressed=1. With one generation in an isolated process, B completed=1 is meaningful at final observation.

## Material gap resolved by the native observation proposal

The previous only public native observer is returned by `PluginsManager::begin_curated_repo_sync_process_stop`. It calls `request_stop`, permanently closes admission and signals the exact control. It cannot establish success before B registration without changing the experiment. Repository/SHA visibility is earlier than installed-cache refresh and `WorkerGate::complete`, so filesystem polling or Git exit is not a success-latch receipt.

The proposed associated `PluginsManager::observe_curated_repo_sync()` returns `None` when the static gate has not been initialized; it does not initialize it. For an existing gate, one locked snapshot reports exact generation, gate closure, recorded native join disposition, exact retained handle `is_finished` readiness, bounded immutable operation category, quarantine and unexpected-handle count. It never joins/takes a handle, signals control, registers a callback, clears failure, reads error payloads or changes admission. A retry requires callers to distinguish the new generation.

`Succeeded` is published under the same worker mutex as its replay success latch, after repository and configured installed-cache refresh returned success. It precedes dispatch and callback body completion. Recorded `Running` deliberately stays `Running` for an unjoined finished native handle. Original errors and primary custody remain in their existing owners. No process-group, DNS/backend task, publication durability, all-host, or callback completion guarantee follows.

Four proposed tests use real exact worker handles and real callback scope tasks: held-work observations preserve handle/control/admission; success is replayed to a same-home scope without stop while a callback remains held, then explicit final joins are checked; quarantine and original private failure remain retained while diagnostics contain no payload; explicit retry changes generation without rewriting a prior snapshot. All are uncompiled/unrun.

## Remaining acceptance coordination, not a claim of execution

The three cases remain: B registers before Git release; success latch is observed after A closes and before B starts; B startup races Git release under an actual readiness barrier. Every case needs actual typed existing-thread state and exact final A/B scope observations, not gate-only snapshot tests.

Two refinements are required before freezing that fixture:

1. Native handle-finished observation and a non-closing callback activity snapshot can coordinate already-dispatched work without moving handles. Finished-but-unjoined must remain distinct from exact successful joins. Final existing stop/join/scope observers still determine outcomes. This extension is separately staged in p03-curated-callback-activity-proposal-01 and independently statically reviewed; raw record counts alone cannot establish callback completion. The replay fixture must observe the same generation with Succeeded plus native_handle_finished=Some(true) before B starts; Joined remains a later owner operation.
2. A new-version hook on the next turn is insufficient to isolate the curated callback: `tasks/mod.rs` activates plugin selection and can refresh hooks at each turn start. Hold an already-started B turn at the actual model response barrier, complete curated refresh, then trigger that same turn's Interrupt/Stop hook (which reads current `sess.hooks()` at the event). Observe the new MCP Ready notification for that existing B thread before a tool call, because the call itself can refresh dirty MCP state. Require behavior plus final callback outcome, not just an invalidation request.

Wrong-home `HomeAdmission::UnsupportedHome` is private to core-plugins and logged by manager. `in_process::start` returns a healthy client, not that typed admission result. The production fixture must not assert an inaccessible typed result or imply cross-home support. Existing typed core-plugins unit coverage remains separate.

## Adoption and commands

Review/adopt the observation unit only after the current native gate reaches terminal and bind/rebase its three preimages against root's final formatted source. No external dependencies, lockfile or schema changes are proposed. New tests are sibling modules. Root should run the ordinary strict wrapper around `just test -p codex-core-plugins --lib` with an appropriate nextest filter for `worker::observation::tests`, then the agreed scoped lint/format gate. This is a proposed command, not a run result.

The later acceptance child belongs in `app-server/src/in_process_curated_replacement_tests.rs`, declared from `in_process.rs`, with an owned external harness launching that exact test ELF/case once per phase. Build the child via root's `just test` flow and run its exact ELF under the unchanged strict subreaper; record test/source/trusted-Git hashes, exact fixture commits, typed phase receipts, callback outcomes, final native join and fixture waits. All test-process environment must be set by the parent before startup. The concrete acceptance source is being prepared separately against both observer units; no compilation or runtime claim follows from their static review.

Licenses: current repository Apache LICENSE/NOTICE copied alongside this proposal; no third-party code or new dependency included.
