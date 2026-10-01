# Codex Harness Compartmentalized — execution state

Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Scope/status: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required update track: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Full v1 is incomplete. Follow this queue across runs; do not infer completion from crate counts.

## Last verified source and publication

- Repository: `DesignStuffDev/codex-harness-everythings-a-plugin`; original cloud checkout
  `/workspace/codex-harness-everythings-a-plugin`. Preserve isolated saved work at
  `/workspace/codex-harness-next-components`.
- Verified cancellation source: `3b8a889738d27eb1edc95e3f63891ab9cf0b6017`, tree
  `8f64a2b3b70cbf52bfb89a065f2bf791b214cc2a`, published and remotely verified on
  `wip/p02b-preparing-cancellation-20261001`. Six final review slices extend `9bd3bc30`;
  their intermediate commits were not individually compiled. This checkpoint promotes the
  verified final source while retaining current main documentation and original failed evidence.
- Main verified checkpoint: `c28a1c33a856a987316f4b97b4c488d68055fffc`, tree
  `b40fa4d6f635dea2de7ca1856aaf96d5af58f37b`, remotely verified after nonforce publication.
  Parents are `a0be45bd` and `3b8a889`. It promotes73 source paths plus22 documentation/evidence/
  fixture/image paths. Among8978 published scoped entries, implementation hashes match the build;
  one historical-plan status header was updated afterward. Bubblewrap LICENSE remains the same
  `COPYING` symlink, outside the regular-file hash map. Receipt:
  `/workspace/recovery-backups/20260930T165936Z/p02b-preparing-promoted-publication.json`.
  Previous accepted runtime `a469cf4` and additive SDK `d22cea88` remain historical.
- Official upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`;
  exact-tree import `ae720ae9a98bad29ca2cff998e7d5baaf05cec86`, tree
  `147ac2447134294359c4071b0aeb495922760db7`. Retain LICENSE/NOTICE and lineage.
- The local HEAD/index intentionally remain at the original upstream pin. Local `main` is stale.
  Do not reset or fake local commits to match connector publication. Original index SHA256:
  `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.
  Use temporary indexes/exact trees; root owns checkout edits, Rust builds and Git writes.

## Verified search-lifecycle milestone

- Combined AS/client/TUI libraries: **6095 passed, 4 skipped**, zero retries. Initial E0624
  compile failure preserved; one parent-visibility correction preceded the pass. Lint passed
  unchanged. Formatting changed ten files mechanically, reviewed separately. See
  [library evidence](verification/2026-10-01/P02B_PUBLIC_STOP_EVIDENCE.md) and
  [format transition](verification/2026-10-01/P02B_PUBLIC_STOP_LINT_FORMAT_EVIDENCE.md).
- FullCLI build passed with all10148 scoped fingerprints unchanged. Frozen artifact:
  `/workspace/component-checkpoint-candidate-p02b-preparing-full-cli-20261001/codex`,
  SHA256 `1b72a190ba6ebcca68c4f0d4145f4ec129b975e0e18fcddfab322f8e7f9165e7`,
  634578704 bytes, mode0555/nlink1; original inode retained, no byte transformation.
- Real installed search: unchanged worker03(0.1.0) and worker04(0.2.0) normal compatibility;
  held-reply legacy cancellation and public sessionStart/Stop FIFO/replacement all passed.
  Release was Joined before unhold; exact late reply, sibling survival, ordinary shutdown,
  removal/native restoration and tracked PID absence passed. The relay holds a reply after
  native Ready: this specifically proves host/process Preparing, not native construction.
  [Build/installed evidence](verification/2026-10-01/P02B_PREPARING_NEWHOST_EVIDENCE.md).
- TUI integration: **23 passed, 4 skipped**, zero retries, unchanged source. All five former
  integration failures now pass; the separate library gate cleared the other twelve.
  Preserve the earlier5633pass/17fail/8skip report. No test assertion or snapshot was weakened.
  CLI env binding is a retrospective root-launch transcription, not wrapper-captured metadata;
  some cases use the standalone TUI. [Evidence](verification/2026-10-01/P02B_PREPARING_TUI_INTEGRATION_EVIDENCE.md).
- Unchanged native storage0.2/contract2:13 runtime commands passed, including real CLI/tool/context/
  streaming/cold-resume behavior; fresh manual migrations01/02 each passed10 commands.
  Normal GUI passed two cold launches, search/reference flows, streaming, approval, Stop, reload,
  continuation and recovery. First SIGINT only to manager; exits0/.266s/.265s, tracked identities
  absent. [Storage/normal GUI evidence](verification/2026-10-01/P02B_PREPARING_STORAGE_GUI_EVIDENCE.md).
- Instrumented GUI clear-query Preparing passed two cold-launch cycles. Actual joined Release
  and empty original/cancel replies preceded unhold; sibling/replacement searches survived.
  Existing GUI regressions and first-manager-SIGINT shutdown passed0/.719s/.215s, all tracked
  identities absent. SIGINT occurs after unhold; pending-Open shutdown remains a distinct gap.
  [Held GUI evidence](verification/2026-10-01/P02B_PREPARING_GUI_HELD_EVIDENCE.md).
- Earlier separate native-owner88/native-backend96, API/native/worker169, process42 and runtime55
  scopes remain in their evidence files; do not add overlapping counts. Raw nonzero adopted
  child exits and original failures remain preserved. GUI uses Chromium/Playwright fallback and
  deterministic inference, not in-app Browser or live-provider proof. Storage's historical
  per-child exe-disappearance assertion is weaker than GUI/migration PID-absence assertions.

## Ordered next actions

1. P02 promotion is verified above. P03 shared transport has passed focused tests, lint and
   the newly linked standalone CLI's 24-command installed-package gate. Publish this coherent
   support checkpoint first; the locally verified declaration slice has its own subsequent gate.
   No root Rust command is currently active; recheck before editing or building.
2. The reviewed P03 private wire-codec slice is ADOPTED from `/tmp/p03-session-wire-slice1-stage`.
   Manifest SHA `fed07229d88d06db099834c21633e769427e559b6a97ae07ea086436bc17ffe0`.
   Seven preimages matched; explicit `#[path = "session_wire/raw_tests.rs"]` was added per
   AGENTS. Adoption report `/workspace/acceptance/p03-wire-slice1-adoption.json` binds all7 files.
   Before/proposed archive SHA14cdf3c5ee358412a994ef5ab051dac903393408f9b797bcf391300cbc027ece.
   The frozen stage is unchanged. This is shared
   support, not native extraction or broker activation.
3. P03 focused gate passed **87 tests, one skipped**: 81 host library and six manager launch
   cases; API compiled but contributed no executed tests. Strict subreaper exited 0 without
   runner error; three raw child exits -9 and one 0 remain recorded, without inferred causes.
   Scoped `just fix` passed unchanged; `just fmt` changed three files mechanically, independently
   reviewed. Preserve [tested hashes](verification/2026-10-01/P03_WIRE_SLICE1_EVIDENCE.md) and
   [format transition](verification/2026-10-01/P03_WIRE_SLICE1_LINT_FORMAT_EVIDENCE.md).
   Build passed with 8,861 scoped fingerprints unchanged. New frozen search CLI:
   `/workspace/component-checkpoint-candidate-p03-wire-cli-20261001/codex-file-search`, SHA
   `a52a960f424101e34d0027555d8ee7f223c7a685721d57477c2a4dcf1cb7dfa1`.
   Artifact receipt: `/workspace/acceptance/p03-wire-slice1-cli-artifact.json`.
   The unchanged worker04 and original manager passed the explicit reused-package gate: exact
   native/external/restored parity, CLI-only Ctrl+C exit130, selected failure exit1 without
   fallback, removal and observed PID absence. All 24 commands met their expected outcomes;
   strict subreaper exited0/null. [Runtime evidence](verification/2026-10-01/P03_WIRE_SLICE1_RUNTIME_EVIDENCE.md)
   preserves original independent-build proof and actual historical source bindings. Current
   fullCLI1b72/GUI evidence predates P03; new-codec full-host/GUI acceptance remains unclaimed.
4. Agree the bounded, opt-in leaf-broker contract before parallel implementation, then rebase stale
   unlinked drafts. Use retained active startup/decode/reply/flush ownership, exact negotiated
   grant equality, explicit aggregate budgets and guarded typed lifecycle calls. See
   [P03 source audit](P03_SOURCE_AUDIT.md). No worker-supplied configuration may create authority.
5. Proceed to native model-catalog/cache extraction after required authority/composition seams:
   provider-owned endpoint capability, atomic native policy epochs, isolated-session behavior,
   bounded pending work and retained cleanup. External native/custom workers must visibly alter
   real model listing/selection with unchanged host bytes. General Session cleanup, model-v2,
   auth/config, inference and storage's private search fallback remain explicit obligations.
6. Continue P04–P19 against the canonical inventory. Keep source/symbol lineage current now.
   P18U must implement an installable updater and external recovery bootstrap, then demonstrate
   a real later-upstream isolated integration plus rejected breaking update/failed activation
   rollback. No polling schedule or live update is enabled.

## Resume and preservation rules

- Read AGENTS and the cloud-runtime skill; verify this original task-bound environment, repository,
  both worktrees, active processes, current/enforced policy, mount space and cgroup memory.
  Never substitute a blank checkout/local VM. Shell Git auth is unavailable; supported GitHub
  connector publication works. Recheck refs immediately before nonforce updates; no unrelated settings.
- Recovery root: `/workspace/recovery-backups/20260930T165936Z`. Full both-worktree archive
  `codex-recovered-workspace.tar.zst`, SHA
  `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
  Later source incrementals, original Git state, staged patches and publication receipts remain.
  These are cloud-local checkpoints. GitHub protects included published source only; it does not
  back up private state, credentials or executable artifacts. Preserve isolated unfinished work.
- Keep original manager `/workspace/component-checkpoint-candidate-p02b-search-cli-20261001-02/codex-component`
  (SHA eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a,mode0700), both frozen fullCLIs,
  worker03/04 source/package receipts, storage package, source archives and all failed evidence.
  Package helper guards bind exact modes/fingerprints; never alter artifacts to bypass checks.
- Rust1.95 environment: `/workspace/toolchains/component-verification-env.sh`; override jobs1,
  debug0/incremental0 already set. Use `just test`, zero retries and the unchanged
  `component-sdk/tests/subreaper_runner.py` (SHA fe01097ae1741cbcb76e15fb07c0c936108a4dc35c08b4264c1caa9eb1e08875).
  Do not kill Rust. Source wrapper baseline f2cc is stale annotation; actual hash maps are authoritative.
- Memory/tmpfs is a material constraint. Some generated rmeta/rlib paths link to `/tmp` or `/dev/shm`.
  New twelve-library relocation:1478025216B, original paths retained,60 metadata files unchanged.
  Recovery reports `p02b-additional-completed-library-shm-relocation.json` and
  `p02b-additional-shm-post-verification.json` record exact aliases/hashes. Prior four-library report
  `p02b-completed-library-shm-relocation.json` remains. One old TUI alias was rebuilt regular;
  preserve it and its different prior backing. SHM is volatile cache, not source backup. If missing
  after restart, while Rust is idle remove only verified recorded dangling generated-cache links;
  allow Cargo regeneration. Never remove a rebuilt regular file based on an old symlink record.
- GUI reports/logs/readiness URLs are private. Publish only reviewed whitelisted summaries/images.
  Tested staged-runner source archive and member manifest are under `verification/2026-10-01/fixtures/`
  and `p02b-preparing-source-fixtures.json`; exact path-bound provenance checks require reviewed
  restaging on another machine. Do not disable them. Viewer networking remains optional and separate:
  current enforced policy exposes no incoming preview bridge or approved relay hostname.
