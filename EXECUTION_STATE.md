# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Incomplete platform; P03 lifecycle prerequisites remain open.**
Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Coverage: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required updater: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Earlier detailed ledgers are preserved, with process IDs redacted from the public copy, in
[EXECUTION_CHECKPOINT_HISTORY.md](EXECUTION_CHECKPOINT_HISTORY.md).

Immediate queue: publish the completed final65 production/storage/GUI/slow-shutdown
and eight-case matrix evidence; preserve the current542 test executable, then review
and test the staged featured-warmup owner before its production integration.
Close the remaining P03 task/transport ownership gates before P04 extraction.
The featured warmup ownership proposal and MCP proposals remain isolated under R.
Do not repeat the unchanged failed matrix or promote this partial platform to main.

## Identity and preservation

- Original checkout: /workspace/codex-harness-everythings-a-plugin.
  Preserve sibling /workspace/codex-harness-next-components; no new VM/checkout/chat.
- Origin: https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git.
- Upstream: openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478.
  Exact-tree import ae720ae9a98bad29ca2cff998e7d5baaf05cec86, tree
  147ac2447134294359c4071b0aeb495922760db7. Retain licenses/NOTICE.
- Local HEAD/index remain upstream; local main is stale. Never reset or rewrite
  the real index. Index SHA256:
  0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59.
- Root owns checkout/Git/compiler/cache mutations. Workers stage only under
  R = /workspace/recovery-backups/20260930T165936Z. A = /workspace/acceptance.
  Recheck resources before large builds; never kill Rust commands.
- Actual commands/tests executed in the recovered original VM. Executor access
  is restored; this is not full-v1 acceptance.
- Preserve older live GUI under /workspace/verified-component-checkpoint-v2-20260930
  and /workspace/remote-viewer-setup. It is not current-source proof.

Full earlier two-worktree/Git/SDK/evidence archive: R/codex-recovered-workspace.tar.zst,
SHA256 3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd.
Fresh primary pre-format archive includes 9,946 source/Git-metadata paths:
R/p03-replacement-current-source-before-format-01.tar.gz, SHA256
e6ab98fe2bd9625c10fe67979eaf308b4f7e4ee7892548482ff0750d9847df5e.
These are verified **VM-local** checkpoints. GitHub durability covers only published paths.

## Publication and current source

Main remains 781080f7e3c8bfe1953378001d777dff33d74bc3 (native installer source
65511842d7051b2a1f5cc52917f3ebb5c03be4f3). Do not promote unfinished P03 work.
Latest WIP ref verified after this checkpoint:
7adb464cd19c2ddfeff71ccdf13839d26979197b, tree
4e84d27238beb9efd839ea2336ede740aefe9a67, on
wip/p03-process-final-and-mcp-preservation-20261002.
Prior current58 runtime checkpoint: f8a5905bb666846a7885171280d9454a9b8a269b.
All 58 selected remote blobs and both refs were verified. Receipt:
R/p03-replacement-checkpoint-publication/PUBLICATION.json, SHA256
b042a69343b43d641ddac0876013370e5d900b8ab844a1a51bdd78cb63d86b9b.

Current P03 source: 65-path final manifest
R/p03-replacement-final-source.json, SHA256
bfad182c433bf88ec31b65d1e4d4234385b15be2f1e160645ab1bfff2ebb9040.
Full 8,928-file scoped map:
1c54717dd95ff85361d33076863129435f9516d3f815ff8690263fdda083549a.
Earlier formatted V2 (7d9b57fc…) corrects an inherited byte-equality flag in
preserved V1 (16f3d289…). Subsequent lint changes are recorded separately.
The post-format/lint production build and bounded installed-storage/GUI regressions
below passed on these exact bytes. The full P03 lifecycle gate remains open.
New offline support under upstream/ is outside this native/SDK scope.

## Implemented versus proved

Installed native replacement remains bounded to thread storage/manual migration,
inline attachments (manager-level subset), and bounded file search. The separately
packaged GUI is additive presentation. Observations, callback fixtures and maintenance
scripts are **not additional extracted subsystems**. Most engine/auth/model/context/
tool/session families remain coupled; inspect every inventory row.

- Current58 production build and installed-storage/migration, ordinary/search GUI,
  slow normal/forced Launch shutdown passed on their recorded source. Real Chromium/
  Playwright used deterministic inference. Normal slow shutdown waited 46.47s; forced
  shutdown reported durability unknown. Earlier failures remain failures.
  [Recovery evidence](verification/2026-10-02/P03_RECOVERED_HOST_EVIDENCE.md).
- Stage1: just test -p codex-core-plugins --lib: **542 passed**;
  A/p03-observation-stage1-tests-01, map6f11c751….
- Stage2: just test -p codex-app-server --lib: **431 passed, one intentionally
  ignored parent-driven child**; A/p03-replacement-stage2-tests-01, map
  1d8ccc03b00f0e2a39573d96220f1695d2c1817d74ef846aa0e0a141a1946d4b.
  Strict 0/null; adopted statuses include SIGKILL/SIGPIPE/141 without attribution.
- New App Server library ELF: target/debug/deps/codex_app_server-4a5773d640636e8f,
  SHA256 26777e075411f0a4e7b6761d106aad0c365a2c342bc5ff74a8aa49adf2ca3291.
  Compiler output selection observed; output hashed after the suite. Old same-name
  ELF remains archived in R/p03-process-final-app-server431-proof-preserved.tar.zst.
- Replacement run01 failed with Tokio stack overflow before B. Preserve
  A/p03-replacement-stage3-runtime-01. Parent now sets RUST_MIN_STACK=8388608,
  matching just test; assertions/deadlines unchanged.
- Run02: **pending/replay/race all passed** against the new ELF.
  A/p03-replacement-stage3-runtime-02: A callback 0/B callback 1, real Git, updated typed
  metadata/skill/MCP behavior, exactly one v2 interrupt hook, native handle joined
  without quarantine. Strict 0/null; all 10 adopted statuses0; no forced fixture cleanup.
  MCP transport custody remains unproved and whole_host_clean remains false.
  This is same-home embedded-client replacement, not component hot swapping.
- Just fmt passed: 38 files, 34 Rust mechanically reviewed, four Python ASTs equal;
  60 frozen evidence files unchanged. No tests rerun solely for formatting.
  Scoped Core Plugins Clippy passed unchanged. App Server's first lint failed
  after removing a test-used import; the import moved into cfg(test), and the
  retry passed with two equivalent test-fixture fixes. Final formatting passed
  unchanged. Assertions/deadlines remain unchanged; no tests rerun solely for lint.
  [Lint transitions](verification/2026-10-02/P03_REPLACEMENT_LINT_EVIDENCE.json).
- Offline impact planner:17 in-memory fixtures plus22 existing lineage checks passed.
  Real local Git invocation returned expected unresolved exit2 for the same upstream
  revision and stale composition. No update integrated/approved/activated.
  [Tool scope](upstream/UPSTREAM_IMPACT_README.md).
- Final65 production build and installed storage/four migrations/ordinary and selected
  GUI/normal and forced slow Launch shutdown are now independently recorded in
  [current production runtime evidence](verification/2026-10-02/P03_CURRENT_PRODUCTION_RUNTIME_EVIDENCE.md).
  All89 private artifact hashes were reverified before adopting this sanitized report.
  Both earlier full held-host matrices remain failed, with exact inner/outer codes.

## Ordered next actions

1. Recheck production build headroom. Completed 54 invalidated-intermediate
   retirement recovered 732,139,520 allocated bytes; receipt
   R/p03-post-stage2-invalidated-production-cache-02/RETIREMENT.json,
   SHA256 87bf9bdbbca83b7265531b4b2efb440a3c167e18a8daf1ff9d3046fcc8b88ea0.
   Lint subsequently used some space. Preserve exact current58 CLI proof before
   compiler path reuse. Retire only exact proven-obsolete intermediates or
   separately archive/verify recoverable inactive artifacts. Never rerun completed
   355/138/54-cache or ten-snapshot actions. The additional 44 obsolete failed-TUI
   intermediates are now retired (465,612,800 allocated bytes); receipt
   R/p03-post-lint-invalidated-tui-cache-02/RETIREMENT.json, SHA256
   d8cd971b1e518f6f29664cbd3ade77abe5e9e1fe64a1ea250d050fe79ef8b8dc.
   The first CLI archive guard found 16 existing arg0 aliases and stopped before
   mutation. Revised archive-only guards pin those identities unchanged. The
   current58 CLI is now fully archived and restoration-verified at
   R/p03-current58-cli-proof-preserved-01.tar.zst (139,748,196 bytes), SHA256
   d5368ee56cd9b4a78e3ad60ca52463c44c6d509aadb47cc367aa8d84eb9b2251.
   Separate retirement03 rechecked inactive ownership and retained all 16 symlinks;
   only the two original hardlinks were removed. Receipt SHA256
   1c9a90a85f73eb3ad6cedfa8717f0368317c8dcfc3e15195be759f315cb82405,
   R/p03-current58-cli-proof-preservation-03/RETIREMENT.json. Exact restored proof
   remains in /tmp/p03-current58-cli-proof-restored-01; do not delete it casually.
   All GUI/session homes remain intact. Resource summary before retirement:
   R/p03-replacement-resource-checkpoint-01.json. This archive is VM-local only.
   Production build started with 1,958,793,216 free overlay bytes; preflight
   R/p03-replacement-production-preflight-01.json. Do not rerun completed actions.
2. **Production build passed**: completed session 41614, prefix
   A/p03-replacement-production-build-01, final65 bfad182c / map1c54717d.
   Command: cargo build --locked -p codex-cli --bin codex -p codex-component-host
   --bin codex-component. Source wrapper records actual before/after; one compiler
   writer and TMPDIR=/tmp/p03-replacement-production-build-01. Never kill Rust.
   Build finished 10:40:52 UTC, exit0, unchanged full scope, 6m22s; no additional
   OOM kills. Source receipt SHA256
   14e91233defe092c937970956a6bdb11a0670566001d388ba8428a4b66bd6658.
   Binary receipt SHA256 cd588d2b27200e0553f8cb8e236c5c2e25cc8e94ded2dff4ecf32bd883133041.
   New CLI dc7f1e84d5bcb311fdc7751b1538319f6205a892203edfc74c2993103f18f158;
   manager remains cached unchanged f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954.
   CLI compiler intent was observed and output hashed after success; top-level CLI
   and exact selected deps output are the same inode. Runtime acceptance is separate.
   **Fresh eight-case matrix failed**: completed session57902, prefix
   A/p03-replacement-held-host-01, wrapper1 with unchanged source. First two CLI
   cases passed; third git-app-server-eof failed fixture_protocol_failed with
   auxiliary_request_after_stop_trigger. Host exited0 in about0.033s; no forced
   fixture cleanup. Five cases were not run. Investigate auxiliary request ordering
   without weakening fixture assertions; this is not a full matrix pass. The gate needs a **complete**
   eight-case held Git/HTTP matrix with positive independent Git acknowledgment.
   Current58 attempt03 stays **4/8 accepted, fifth failed, three unrun**.
   Corrected fixture: R/p03-held-host-observer-ack-proposal-01, manifest14c27cff…;
   Its 15 pure checks passed before the fresh failed native matrix. Do not combine partial runs.
   Prepared runplan: R/p03-replacement-held-host-runplan-01, manifest456032cc…;
   requires the new successful production receipt and actual CLI fingerprint.
3. Run current-production storage/migration and GUI/session/cancellation/recovery/
   actual Launch Ctrl+C regressions with exact independent package-build/reuse proof.
   Older current58 results and build success cannot substitute for these gates.
   Independent storage gate passed while the failed matrix is diagnosed:
   A/p03-replacement-newhost-storage-01, original package proofab41bc4a…,
   new CLI dc7f1e84… and unchanged managerf054d84a…. Thirteen behavior checks and
   all three storage child terminations passed; strict0/null, adopted -9:8/0:1.
   This is package reuse, not a new independent Rust build. Fresh normal migration
   passed ten commands, strict0/null, adopted -9:6/0:1. Ordinary GUI passed two
   Chromium/Playwright cold cycles, approvals/streaming/Stop/search/actual Launch
   Ctrl+C, no page errors and unchanged packages/host; strict0/null, adopted0:1.
   Prefixes: A/p03-replacement-newhost-{storage,migration-normal,gui-normal}-01.
   Do not report all adopted statuses zero for storage/migration.
   Root viewed gui-normal recovered-2.png; in-app Browser was unavailable.
   Selected-search migration and two selected-search GUI cold cycles passed with
   prefixes A/p03-replacement-newhost-{migration-search,gui-search}-01. The actual
   App Server used the independently built native.file-search-local package; the
   original external build proof is reused, not rerun. Manager shutdowns took
   about0.27s; tracked processes absent, binaries/packages unchanged. Root viewed
   search-results-2.png from that actual Chromium run.
   Fresh migrations for both slow-storage fixtures passed ten commands each;
   prefixes A/p03-replacement-newhost-migration-slow-{normal,forced}-01. Each strict
   runner returned0/null with adopted -9:6/0:1. Actual Launch normal shutdown
   waited46.317s, exited0, confirmed native cleanup and cold recovered the held
   canonical user-message event. Forced shutdown exited1 after2.042s, explicitly
   reported durability unknown, and cold recovery correctly lacked that unforwarded
   event; subsequent operation and normal shutdown passed. Prefixes
   A/p03-replacement-newhost-slow-{normal,forced}-01; strict0/null with adopted0:6
   each. All tracked processes absent. Admission was held in the installed relay
   before native forwarding; this does not prove the native internal admission
   budget or fix response-only interrupted-history projection.
   New test homes use
   /tmp/p03-replacement-newhost-*-01-runtime in this same VM to conserve overlay;
   wrapper/strict receipts remain in A. These are test homes, not another checkout.
   Connection-boundary diagnostics preserve all89 original fixture assertions.
   A/p03-held-connection-observations-policy-01 passed23 pure checks. A separately
   fingerprinted single EOF diagnostic completed0 on the same native bytes;
   independent observer, unchanged strict runner and case checks also passed.
   Its auxiliary connection was already held before shutdown, so this does not
   reproduce or close the original race. Full diagnostic matrix
   A/p03-replacement-held-diagnostic-matrix-01 completed1: first two accepted,
   third EOF rejected by the outer frozen_observer_real_git_missing check despite
   inner-case success; five unrun. The independent acknowledgment currently covers
   fallback mode only; an R-only proposal will require it in Git mode too, retaining
   all deadlines and rejection assertions. Review that correction before the next
   native matrix; do not repeat the unchanged failing setup. Instrumentation adds observations,
   not a native semantic fix; retain both failed matrices and do not infer
   startup-race closure from a later pass. The source audit separately found an
   unowned featured-cache warmup task; its scoped ownership proposal is R-only,
   unadopted and untested, and the encrypted auxiliary request is unattributed.
   **Fresh complete matrix passed after the separate readiness correction**:
   A/p03-held-all-mode-ack-policy-01 passed31 fixture checks; then
   A/p03-replacement-held-all-mode-ack-01 passed all8 cases in one unchanged-source
   run. Fixtureac8ae90e… / observer11e5c906… require independent exact-live-Git
   acknowledgment in both modes before held readiness, using the same60s budget.
   All original89 assertions and strict5s drain remain. Each case retained its
   expected host status, reported connection EOF, and used no fixture rescue;
   each strict runner returned0/null. Git adopted codes -9:2/0:1 per case;
   fallback adopted128:1/0:1. ACK generation is bound to the independent observer,
   not necessarily the inner sampler or one exact Git operation/transport owner.
   All auxiliary holds preceded stop in this run. The original early-auxiliary
   race remains unclosed; no native fix or whole-host/MCP custody is inferred.
   [Complete matrix evidence](verification/2026-10-02/P03_CURRENT_HELD_MATRIX_EVIDENCE.md)
   and frozen fixture sources retain exact31-check/eight-case fingerprints.
   Preserve the failed matrices and separate successful single diagnostic.
   Before the next scoped build, only82 proven source-invalidated failed-TUI
   intermediate files were retired, recovering665,604,096 allocated bytes.
   R/p03-post-new-production-invalidated-tui82-cache-01/RETIREMENT.json SHA256
   d2e7b28b122312078ad7f73e3dcef50443871c3e9bf40f227e4bc963b2578ea0.
   Root independently verified the conservative1783-node protected graph and all
   invalidation edges. Current proof executables, index,225 archives, both source
   worktrees and old restored CLI are retained. Do not rerun this completed action.
4. Close MCP custody: staged upper manager ownership → successful-owner retirement →
   lower transport ownership → pinned SDK → aggregate late-admission fence.
   These proposals remain unadopted/uncompiled. C2b archive cancellation remains
   staged; transactional repo/SHA publication and host-death fencing are still needed.
5. Resume installed native auth/catalog and all dependency-ordered P04–P19 work.
   Keep source/component provenance and kernel exceptions explicit.
6. P18U: independently installed maintenance worker and external recovery bootstrap,
   real later upstream integration preserving custom plugins/UI, coordinated versions/
   migrations, incompatible-update rejection and failed-update rollback. No polling
   or unattended live deployment. Remote viewer networking remains separate/optional.

## Resume constraints

Read root AGENTS.md, verify source/process ownership/disk/cgroup headroom/remote refs.
Use /workspace/toolchains/component-verification-env.sh, one compiler/cache writer,
and source/strict wrappers. Use just test and scoped packages; the complete workspace
suite still requires separate user approval. Never kill Rust. Do not dump auth/config,
credentials, raw runtime reports or environment values. Verified milestone publication
is authorized; force push and unrelated settings/account changes are not.
In-app Browser and Context7 are unavailable here; do not claim their checks ran.
Label real Chromium fallback and deterministic inference. Existing authenticated
remote viewer connectivity remains separately blocked.
