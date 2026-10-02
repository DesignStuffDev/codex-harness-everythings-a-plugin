# Curated callback ownership and routing — scoped evidence

The combined callback implementation passed **953/953 library tests** (429 App Server,
524 core-plugins), with zero skips/retries and unchanged strict-runner exit 0/null error.
This is native lifecycle preparation with actual curated routing. It adds no independently
installable component and does not complete process-final shutdown or replacement delivery.

The ordered source commits are:

1. `8d3beeff520177c3dee7562d168253ed7bdeafda`, tree
   `75dbe7618f88f6a36ef9784ab5592c496c28331d`: scope state machine, exports and tests (3 paths).
2. `0c1ed130daf3dc36768e35109c576336d9f66005`, tree
   `f2b6c900d8225325affc0252d6f6680f500e0e5a`: actual curated routing and App Server lifetime
   guards/regressions (13 paths), parented on the first commit.

The first parent is publication `6856eb25b2570ecebe036d07a3a515b708893b9d`, which carries
native completion source27d. Only their **combined** integration was tested; the scope-only
intermediate was not independently tested. Root read back all 16 final source blobs. Branch
advancement and the evidence child require the separate publication receipt. Upstream remains
`openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`; Apache LICENSE/NOTICE are unchanged.

Each processor's scope is strongly registered before runtime spawning. Dispatch reserves custody
under the mutex that closes admission, spawns outside it and attaches the exact Tokio handle.
Reserved tasks recheck closure before action admission. Cancelled, failed, pending and unexpected
records retain custody. Finished handles are harvested outside the mutex with no intervening
await; an unexpectedly pending handle is restored. Original join errors and spawn-panic payloads
remain retained. Closed clean scopes retire; open idle, pending and failed scopes remain reachable.

The real curated manager path uses a typed, payload-free callback and rechecks native stop after
cache clearing. Its old spawn trampoline is replaced by registered dispatch. Initial curated
startup awaits plugin/skills cache clear, MCP invalidation requests and hook refresh. Account
curated work awaits existing best-effort MCP reload/invalidation. Generic marketplace/remote
callbacks, trust/config queues and immediate account refresh retain their prior routes. Native
Rust callback signatures changed; component wire contracts, manifests and SDK versions did not.

Local guards close admission on embedded-handle drop, runtime/processor cancellation, ordinary
shutdown and forced shutdown. Processor cleanup awaits registered callback bodies before later
thread teardown; outer guards retain access after processor failure and combine primary/cleanup
errors. Existing outer budgets still apply. This is not a binary-wide final owner or global fence.

The unchanged strict subreaper ran `just test --locked --retries 0 --test-threads 2 -p
codex-core-plugins -p codex-app-server --lib`. Attempt01 exited 101 during core-plugins libtest
linking (`collect2`, signal 7 / Bus error), before any test. Its source map remained unchanged;
strict result was 101/null, not success. Zero free workspace bytes were observed after the
failure; that is a resource observation, not proof of its sole cause. Root recovered only guarded
generated outputs and archive-verified inactive source922 CLI hardlinks. Original failure reports
are retained separately; the command and test source were unchanged for retry02.

Retry02 passed 953 tests in nextest 40.796 seconds. The strict runner exited 0/null in 70.553267
seconds; its 49 raw reaps include 12 nonzero statuses without executable/cause attribution.
This is not an all-child-success claim. All 11 newly added tests have individual PASS records:
seven callback-scope ownership/race tests, three App Server guard tests and the real manager
cache-clear/dispatch boundary test. Existing shutdown-cancellation coverage also gained a scope
assertion. `--lib` excludes separate integration targets; this command did not rerun the PTY suite.

The 8,906-entry failed and successful source maps are both
`dbcac4b64bb506cc7058d14ceccfad05477147644b9bc501379c345cd287fbb3`.
First scoped lint exited 0 without editing source but warned about a test's lexical MutexGuard
lifetime across await. Root added two braces around the existing synchronous guard/use/drop/
join/assert body, ending before its existing await. Both reviewers found unchanged assertions
and operation order. Second scoped lint exited 0 with no warnings or edits; its map is
`795a1cdc60c34becb591139f921a01393e66b178efc1c98bda61d5cf35935a54`.
Global formatting exited 0 and changed 13 adopted paths only. Both reviewers read the exact
36,243-byte diff: import/module order, wrapping, indentation, block/closure layout and optional
trailing commas. Final map:
`c4c476492f356181b4552fdaaa7bab1b4465837a5dc10dab8068b19c83f2bdd4`.
Tests were not repeated solely for lint/style; tested, lexical-corrected and formatted bytes
remain separate snapshots. The final source archive has 17 verified members, 140,474 bytes,
SHA256 `2e29ff6603bd1711a920315d92a4b6cf842562f17b5ad7a04962723d58abf4f8`.

Completion counts cover exact registered actions, not best-effort failures logged by callees.
MCP invalidation requests refresh; it does not prove session prewarm or other delegated work
terminated. **Exact MCP task custody and joining remain a next gate.** Native sync still captures
its original manager/callback: keeping replacement B's scope open does not deliver A's later
completion to B. Home/generation-aware subscriptions or transfer/replay, registration/completion
races, a registry closing fence linearized against new scopes, and actual binary-owned final
cleanup on success/error/early-return paths remain required. Pending/cancelled observers must
retain custody. One shared deadline and explicit force/uncertainty policy are needed; synchronous
work and native teardown can outlast observation deadlines.

Source922's installed storage/migration/GUI runtime passes and source27d's 587 scoped tests are
separate source-specific checkpoints. No fresh full CLI, GUI, in-app Browser/live-provider,
held production Git/HTTPS, replacement-delivery, slow-shutdown or process-final acceptance is
claimed here. C2b extraction, repository/SHA recovery journal, host-death fencing, native auth/
catalog extraction, remaining components and P18U upstream integration/rollback remain open.

[Structured results](p03-curated-callback-scope-results.json) preserve exact failed/green reports,
style transitions and publication inputs. [Lineage](../../upstream/p03-curated-callback-scope-lineage.json)
maps pinned upstream files and custom boundaries. Archives/snapshots are cloud-local recovery
checkpoints; remote source blobs and any later branch/evidence publication have distinct receipts.
The original frozen GUI remains separate. Eight historical mutable CLI aliases now dangle until
source922 is restored from its verified archive or a new CLI is rebuilt and source-bound.
