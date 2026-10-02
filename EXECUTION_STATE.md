# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Incomplete platform; P03 lifecycle prerequisites remain open.**
Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Coverage: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required updater: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Earlier detailed ledgers are preserved, with process IDs redacted from the public copy, in
[EXECUTION_CHECKPOINT_HISTORY.md](EXECUTION_CHECKPOINT_HISTORY.md).

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
Latest WIP ref verified before this checkpoint:
3ccba58b1077e9372458fe2994a72f30ee643e41, tree
df32f16bb0e64073d820b6ba24ea701a1a4b65e5, on
wip/p03-process-final-and-mcp-preservation-20261002.
Prior current58 runtime checkpoint: f8a5905bb666846a7885171280d9454a9b8a269b.
Check R/p03-replacement-checkpoint-publication/PUBLICATION.json and the remote ref
for the subsequent replacement checkpoint; an absent receipt means publication pending.

Current P03 source: 65-path final manifest
R/p03-replacement-final-source.json, SHA256
bfad182c433bf88ec31b65d1e4d4234385b15be2f1e160645ab1bfff2ebb9040.
Full 8,928-file scoped map:
1c54717dd95ff85361d33076863129435f9516d3f815ff8690263fdda083549a.
Earlier formatted V2 (7d9b57fc…) corrects an inherited byte-equality flag in
preserved V1 (16f3d289…). Subsequent lint changes are recorded separately.
No post-format/lint production build or runtime pass is claimed.
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

## Ordered next actions

1. Finalize/push this bounded replacement/lint/offline-planner checkpoint nonforce
   on WIP; verify changed blobs and both refs. Keep main unchanged.
2. Recheck production build headroom. Completed 54 invalidated-intermediate
   retirement recovered 732,139,520 allocated bytes; receipt
   R/p03-post-stage2-invalidated-production-cache-02/RETIREMENT.json,
   SHA256 87bf9bdbbca83b7265531b4b2efb440a3c167e18a8daf1ff9d3046fcc8b88ea0.
   Lint subsequently used some space. Preserve exact current58 CLI proof before
   compiler path reuse. Retire only exact proven-obsolete intermediates or
   separately archive/verify recoverable inactive artifacts. Never rerun completed
   355/138/54-cache or ten-snapshot actions. New build headroom is not yet certified.
3. Build a fresh production CLI/manager when headroom permits. Run the **complete**
   eight-case held Git/HTTP matrix with positive independent Git acknowledgment.
   Current58 attempt03 stays **4/8 accepted, fifth failed, three unrun**.
   Corrected fixture: R/p03-held-host-observer-ack-proposal-01, manifest14c27cff…;
   15 pure checks passed, no corrected native matrix run yet. Do not combine partial runs.
4. Run current-production storage/migration and GUI/session/cancellation/recovery/
   actual Launch Ctrl+C regressions with exact independent package-build/reuse proof.
   Older current58 results and build success cannot substitute for these gates.
5. Close MCP custody: staged upper manager ownership → successful-owner retirement →
   lower transport ownership → pinned SDK → aggregate late-admission fence.
   These proposals remain unadopted/uncompiled. C2b archive cancellation remains
   staged; transactional repo/SHA publication and host-death fencing are still needed.
6. Resume installed native auth/catalog and all dependency-ordered P04–P19 work.
   Keep source/component provenance and kernel exceptions explicit.
7. P18U: independently installed maintenance worker and external recovery bootstrap,
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
