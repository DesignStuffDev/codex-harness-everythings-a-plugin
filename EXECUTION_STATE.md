# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Partial platform; P03 remains incomplete.**
Read [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md), root AGENTS.md and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md). Superseded ledgers and exact
preservation/resource chains remain in [EXECUTION_CHECKPOINT_HISTORY.md](EXECUTION_CHECKPOINT_HISTORY.md).

## Current checkpoint

Publication verified: evidence commit `c463a4f46318ccbbc811c202639d53f733d5831f`,
all55 selected Git blobs and both remote refs checked. Main remains unchanged.
Receipt: R/p03-http-stage-b-runtime-publication-01/PUBLICATION.json.
Unadopted auth-source archives now have verified GitHub preservation.

- Original VM commands execute; stale access blocker is cleared. Checkout
  `/workspace/codex-harness-everythings-a-plugin`, origin
  `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
  Preserve sibling `/workspace/codex-harness-next-components`; no new VM/checkout/chat.
- Last verified source WIP `d229296b29947ef1328cbe931c810276f220a23c`, tree
  `44570e71ff6978933a6d01ef34b2a2e1de4ea125`, branch
  `wip/p03-process-final-and-mcp-preservation-20261002`.
  Main remains separately verified at `781080f7e3c8bfe1953378001d777dff33d74bc3`.
  This evidence checkpoint changes no active Rust/SDK implementation.
- Production build02 passed offline in7m47s. Source8,947 files, map SHA256
  `7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e`.
  CLI `78d9194cd4329e9b83b75fe07389d524d8b1f425353d71df9a68607be2945e83`;
  manager `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.
  [Build/resource evidence](verification/2026-10-02/p03-http-stage-b-production-build/REPORT.md).
- All12 planned production-runtime slots have passing current-source attempts:
  13 successful commands because search retry needed a fresh migration. One failed
  GUI attempt remains preserved. This does not close all P03 acceptance.
  [First five gates](verification/2026-10-02/p03-http-stage-b-current-runtime-first-five/README.md)
  cover storage, migration-normal, ordinary GUI, attachments and migration-search.
  [Six later gates](verification/2026-10-02/p03-http-stage-b-current-runtime-shutdown/README.md)
  cover two migrations, slow normal/forced shutdown, held8 and the featured-owner
  postcheck; [search02 evidence](verification/2026-10-02/p03-http-stage-b-current-search-gui/README.md)
  covers the diagnostic retry.
  Every successful source wrapper matched the unchanged map above.
- Storage reuses an independently built native package for real CLI streaming,
  tools, cold resume and default restoration. Migration covers dry-run parity,
  apply/idempotence, recovery/cancellation. Attachments pass5 groups/17 commands:
  native upload, cold reference reuse, typed fallback, removal/default restoration.
  Reuse is not a new Rust build. Production attachment resolve, native response
  envelope inspection and nonempty attachment-state retention remain unproved.
- Ordinary and selected-search GUI each pass two real Chromium cycles. Approval,
  native command and Stop are cycle1 assertions; cycle2 covers cold history,
  interrupted state, continued streaming/search. Launch first-SIGINT returned0
  with tracked descendants absent. Inference is deterministic. In-app Browser
  remains unavailable; root visually reviewed selected current screenshots.
- Slow normal Launch waited46.4658s and recovered the canonical message; the46s
  gate holds before native forwarding, not native-internal admission. Forced
  second-SIGINT returned1 at2.0393s with unknown durability; cold continuation
  passed. The raw-input/response-only history gap remains open.
- Held Git/HTTP matrix: all8 case gates and featured-owner postcheck passed.
  Strict histograms include the direct command and reaped descendants; retain
  nonzero child statuses without causal attribution. No universal whole-host-clean
  or all-descendant-graceful claim. The89 require call sites are a static count,
  not executed assertions. Active-held constructor/lower-transport/205s hard-exit
  proof remains separate.
- Search GUI01 failed on its cold cycle's60s file-result wait. Native startup had
  ENOSPC warnings, but causality is unproved. GUI02 used fresh executable SHM homes
  and an observation-only external driver, preserving28 assertions/deadlines.
  Both cycles passed;12 search responses each, no diagnostic overflow/errors or
  pending bodies at close. Preserve01; a retry pass does not explain its cause.

## Ordered next actions

1. Evidence publication is complete as recorded above. Preserve this checkpoint
   while taking the next guarded resource/consumer/auth action below.
2. Exact63-file ordinary-cache retirement completed after independent review,
   audit, second full guard/hash pass and fsynced journal. Receipt
   R/p03-post-runtime-ordinary-cache-action-01/RETIREMENT.json SHA256
   `62fe7f98b882612ec72f0f75ba6c4b0e992d0dc532e4cc53bd27509b4e94e023`.
   Source/index/runtime-selector metadata unchanged; all source, proof ELFs,
   archives/restorations/symlinks and runtime homes were outside the action.
   Overlay afterward785,997,824B; hard unused RAM464,089,088B, no new OOM.
   No Rust build is admitted. Historical CLI test closure still lacks930,639,872B
   of ordinary outputs plus302,977,024B test ELF before scratch/reserve;221,200,384B
   of that missing total comes from the63 retired files and is not hidden.
   Login's142 missing outputs have no exact size receipts. A fresh metadata audit
   found two stale CLI build-script digests, so the old CLI closure is not an
   exact zero-unresolved proof. The new plan conservatively retains all four
   same-package build/run contexts and explicitly records this limit.
   Bounded clean-page advice completed on31 inactive ordinary cache files; no
   candidate content reads, deletion or rewriting. Receipt
   R/p03-current-ordinary-clean-page-advice-01/RESULT.json SHA256
   `dcf6af5df49e454369ae679fbeea17aaf31a681a536d149065cb91e572c581ac`.
   Hard unused RAM afterward1,267,326,976B, overlay785,600,512B, no new OOM.
   The separately reviewed47-file obsolete-context retirement then completed:
   R/p03-post-runtime-exact-context-cache-action-01/RETIREMENT.json SHA256
   `074af2e0fbffa0503d4795fe588c5b48639d949ea06f54b096b4176b445c9271`.
   It removed1,084,129,280B of exact ordinary libraries, preserving21 recorded
   current/prospective roots and2,769 dependency records. Both missing old CLI
   build-script edges remain explicitly qualified through conservative protection.
   Source/index and four production runtime paths were unchanged; no observed
   references/aliases in the declared scan scope, with two pinned infrastructure
   daemon visibility exceptions. Proof archives, executable/restored binaries,
   symlinks and runtime homes were excluded. Root reviewed the immutable audit
   before a second guard/hash pass and fsynced per-file retirement journal.
   Overlay afterward1,867,800,576B, hard unused RAM about1.23GB, no new OOM.
   This admits a separate fresh resource check, not automatic build success.
   Unrelated historical feature contexts may need regeneration. Never replay
   completed actions after path reuse.

3. Current CLI gate completed successfully (root exec50524, exit0):3/3 tests,
   297 filtered, two unchanged snapshots, zero actual retries; two separate
   discovery invocations. Source8,947/map7a and production CLI/manager unchanged.
   Fresh test ELF SHA256
   `2c6a89e4c5386bdc2300208b4648be7bec8d6368d530d4c953180647ee9e9c8d`,
   307,079,712B at target/debug/deps/codex-65eee2a45383cad7. Preserve this new
   proof; old retirement actions for that path MUST NOT be replayed.
   A/p03-http-stage-b-cli-focused-01.terminal.json SHA256
   `cc2f9170ef7df481185a67657952800ffe34ba3ed6f27ef126b1d2c6d6085ecb`.
   Strict command/exit0, runner_error null, reaped histogram{0:1}. Build6m41s;
   full guarded command410.164s. OOM counters remain9/4. Overlay afterward
   629,207,040B and hard unused RAM2,067,320,832B; next compile requires its own
   budget. These controlled cleanup/error-composition tests do not replace real
   owner/watchdog/runtime gates. Frozen plan
   R/p03-http-stage-b-cli-focused-plan-01/MANIFEST.json SHA256
   `11c61cfd25710462e22d44934a0ef6e39d240e9f36f1c2694a62b5d1a863a255`.
   [Published-bundle candidate](verification/2026-10-02/p03-http-stage-b-cli-focused/README.md)
   holds sanitized result and source/runner/ELF hashes; publication pending.
   Next: distinct exec/TUI process-final tests and App Server lifecycle/search/
   storage plus actual same-process featured/curated replacement parent runners.
   Production/runtime passes do not replace those gates.
4. Advance native auth source custody: preserved reload proposal, then permanent
   refresh-failure publication. Neither is adopted, compiled or tested. Verify
   preimages; run causal tests, scoped login/model-provider regression, lint/format
   and later rebuilt-host checks. Recovered disk alone does not admit login tests:
   core_test_support links core even when only five tests are selected.
5. Successful refresh/request dispatch still needs conditional durable storage
   ownership across participating writers; a pre-save recheck is insufficient.
   Preserve equal-value/absence ABA, cross-manager/process, keyring/Auto recovery
   and legacy-writer limits. Then use MCP proposals in order: Session prewarm/refresh
   custody, connection retirement, lower transport, pinned SDK, late admission.
   Resolve marketplace queue transfer before closing per-processor HTTP pools.
6. Activate negotiated broker/model-service composition; extract actual native
   OpenAiModelsManager discovery/merge/cache with separate native/custom builds,
   installation, selection, cancellation, removal/default and GUI proof.
   Continue P04–P19. Required P18U still needs installed maintenance/external recovery,
   real later-upstream integration preserving custom plugins/UI, coordinated versions,
   migrations, incompatible-update rejection and rollback. Offline17+22 planner/
   lineage fixtures do not satisfy those gates.

## Preservation and execution rules

- R = `/workspace/recovery-backups/20260930T165936Z`; A = `/workspace/acceptance`.
  Full two-worktree/Git/SDK/evidence archive `R/codex-recovered-workspace.tar.zst`,
  SHA256 `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
  Primary pre-format archive SHA256
  `e6ab98fe2bd9625c10fe67979eaf308b4f7e4ee7892548482ff0750d9847df5e`.
  Keep later snapshots, failures, unadopted MCP/auth proposals and old restored
  proofs. VM-local archives are not externally durable backups.
- Auth reload/refresh source archives are explicitly unadopted under
  `verification/2026-10-02/unadopted-proposals/auth-source-ownership/`; external
  preservation requires verified GitHub publication.
- Root owns source/Git/compiler/cache mutations. Local HEAD/index remain upstream,
  local main is stale. Do not reset/stage the real index. Index SHA256
  `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.
  Keep the separate older live GUI/remote-viewer directories. They are not current
  runtime proof. Viewer networking is optional; do not repeat its unchanged blocker.
- Build02/plan03 bindings stay frozen. Continuation04 changed only fresh directories,
  ran6 remaining commands, no new OOM. GUI02 dispatcher/strict/source reports are
  separate. Never replay cache retirement/restoration after compiler path reuse.
  Genuine Cargo download archives now have verified but ephemeral tmpfs backing.
- Source `/workspace/toolchains/component-verification-env.sh`, use one compiler
  job and fresh disk/RAM/process guards. Use `just test`, never direct `cargo test`;
  scoped `just fix` and required `just fmt`. Full workspace approval is not recorded.
  No force push, secrets dumps, scheduled polling, unattended live deployment or
  hot replacement. Context7 and in-app Browser remain unavailable here.

Coverage remains three bounded native replacement families: thread storage/manual
migration, inline attachments and file search. GUI is an additive presentation
package. HTTP/auth custody and SDK/host infrastructure are not additional extracted
families. Most engine subsystems and final clean-install/plugin/UI/updater gates
remain unfinished.
