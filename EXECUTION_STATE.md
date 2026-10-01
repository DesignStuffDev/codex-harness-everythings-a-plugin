# Execution state — read first on resume

Updated 2026-10-01 UTC. Follow [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md), [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Full v1 is incomplete. This records verified partial milestones and the ordered next work.

## Published checkpoint

- Main evidence checkpoint `052f9f5cce8da85bc5904201d83283daf10b68d2` precedes this documentation update.
  Latest cancellation WIP is `272993ff0e6376e7ba52403e3b52d7f16449cd16`, tree
  `93201cd97a14b351edda30d35318a80a8402db64`, on `wip/p02b-preparing-cancellation-20261001`.
  It adds required SDK source revision2, worker service and runtime pending controls, all nine original
  fixture implementer migrations, and explicit package metadata. API/native/worker169 and runtime55
  focused tests passed on recorded scopes; AS/TUI fixtures await consumer package gates.
  Nine review-sized historical commits are not nine independently compiled milestones.
  Main runtime is still accepted a469cf4 plus additive SDK support. AS production adoption is now
  in the original checkout under test; TUI production remains staged. Installed new Preparing proof pending.

- Latest additive SDK source `d22cea88aa23e35a619d996fc731122450e51313`, tree
  `fbf28305751239cd376cefaec0ea970048096e08`, follows reviewed evidence/roadmap `789be2e7`.
  It adds pending-start control/ticket/receipt primitives only; no new backend or installed-runtime claim.

- Main source `a469cf4be85fda40c64e563ece3e1639e4abdbac`, tree
  `3216dfb131f72939c986902053665cbad85ce610`, parent docs `b68db514687f6d4466b8d960b0e88010ddc179c9`.
  GitHub connector publication and remote verification completed. 215 source paths over prior main,
  34 new blobs versus preceding App Server source tree 86c0. This is not whole-harness completion.
- Previous App Server source `30c2674600cc84da161562c407515b61511b7c3a` and standalone
  search source `651b0b87a281934e5cd7c639021049fad8d393ac` remain historical checkpoints.
- Frozen build/runtime bytes differ from publication in exactly two documented scope entries:
  startup.rs ed1cb→270bc: two calls formatted after runtime; no format-only rebuild/retest.
  GUI acceptance helper 67a→ffff→022ba: metadata/catalog fixture fixes, separately exercised in GUI04.
  Raw build/final scopes 10,118 have 10,116 equal entries; published retained intersection has 8,946 exact
  matches plus those 2 exceptions. Never substitute one denominator or claim full source equality.
- Native thread storage, inline attachment storage and native filesystem search are extracted subsets.
  Standalone CLI, App Server and TUI now exercise installed search; the GUI uses actual selected search.
  GUI presentation is additive; model/auth remain adapters. Loop/context/tools/policy are still coupled.

## Latest verification — separate scopes, no invented total

- [Required startup/service](verification/2026-10-01/P02B_REQUIRED_START_SERVICE_EVIDENCE.md):
  169/169 passed (96 native,52 component,21 API), no retries/skips; initial compile101 preserved.
  [Runtime Preparing](verification/2026-10-01/P02B_RUNTIME_PREPARING_EVIDENCE.md): 55/55 passed
  (44 runtime,5 plugin-library,6 actual worker-process cases). Initial compile101 and 53/55 regression
  preserved. Only production observer fallback changed for the final pass; assertions unchanged.
  Both scoped lint/format passed; exact pre/post maps distinguish tested and formatted source.
- Independent worker04 build succeeded from new exported source and initially empty external target,
  seven commands0, strict subreaper0/null, source parked. Worker SHA
  `a3f73fed15dd7b19ab0f1daed55a74212d4b2f5d2ba5cdebee2a616d2f2419a1`, 10,362,672 bytes,
  package0.2.0/wire1/metadata `preparing_cancel_receipt:1`. This declaration is not installed-runtime proof.
  Frozen fullCLI7a63 and original manager eb969 fingerprints unchanged. Report:
  `/workspace/acceptance/p02b-search-independent-04/independent-build.json`.
  Package tooling24 tests passed in an exact unchanged staged scope. Original0.1 package retained.


- [Process pending control](verification/2026-10-01/P02B_PREPARING_PROCESS_EVIDENCE.md): 42/42,
  11 new cases with real stdio peer, unchanged source, unchanged subreaper0/null. This is not the native worker.
- [Native owner](verification/2026-10-01/P02B_PREPARING_NATIVE_OWNER_EVIDENCE.md): 88/88 including9new.
  [Native backend](verification/2026-10-01/P02B_PREPARING_NATIVE_BACKEND_EVIDENCE.md): 96/96 including8new;
  it includes the prior88, so do not sum them. Actual constructor/sibling/runtime-destruction checks passed.
  Both clean second lint runs and formatting passed; tested versus post-lint/format hashes remain distinct.
  Native-backend lint follow-up removed an unused private14-line wrapper, with no new behavior claim.

- [Startup SDK support](verification/2026-10-01/P02B_STARTUP_SDK_EVIDENCE.md): 19/19 API tests
  (11 existing +8 new), unchanged scope, no retries/skips, subreaper 0/null. Scoped fix passed
  unchanged; formatting changed three test-only wrapping hunks. No format-only retest.

- [Full CLI/TUI/GUI evidence](verification/2026-10-01/P02B_TUI_CONSUMER_EVIDENCE.md):
  TUI selected retry 23 passed; full regression 5,620 passed, 4 skipped; zero retries and unchanged scope.
  Initial linker SIGBUS 101 ran zero tests; Cargo.lock changed. Disk-zero observation is not proven ENOSPC.
  Scoped fix changed one conditional; test-only cfg annotation and formatting are recorded separately.
- Locked fullCLI build passed with 10,118 unchanged fingerprints. Actual 634,206,672-byte CLI SHA256
  `7a63e9406d1605bac0a84e1b703735caeb211ceccf148337acf07614c7c0032d` was relocated exactly,
  original inode retained, nlink 1, both mutable Cargo aliases removed; no stripping or transformation.
- Actual installed TUI gate 01: five manager commands 0; native 0/installed 0/config-rejection 1/restored 0.
  Real @queries, draft clearing, /cd and /quit. Selected worker SIGSTOP 0.6s prevented a fresh match,
  SIGCONT delivered it through same worker; root switch retained worker. All 25 tracked PIDs absent.
  Preserve six adopted Git exit 128 receipts; command/subreaper 0 does not mean all descendants exit 0.
- Existing Rust thread-storage package 0.2.0/contract 2 reused unchanged; no new Rust package build.
  Nested 13 commands passed, including 4 real CLI turns, tool/context/model-event behavior, cold resume,
  three selected storage processes and native restoration with history. Inference is deterministic.
- Manual migration01 and fresh02 each passed 10 commands and real App Server lifecycle checks:
  exact native/selected dry-run parity, unchanged scoped storage snapshot, migrated→already_paginated,
  cold CLI/App Server history and cancellation. No attachment-specific or crash-durability claim.
- Real GUI04: Chromium/Playwright 49+38 browser commands passed, zero page errors; actual selected
  native search, real response barriers, click/keyboard reference insertion, draft/query/root/task fences,
  streaming, approval, cancellation and cold recovery. Inference is a deterministic fixture.
  First SIGINT sent only to component manager during active turns; exits 0 in 0.266/0.269s; 24/12 tracked
  manager descendants absent and browser drainage passed. This is not requested in-app Browser proof.
- Preserve GUI01/02/03 failures: all zero GUI runs. 01 manager-mode mismatch; 02 metadata schema;
  03 catalogue expectations after 3 successful setup commands and exclusive fixture claim.
  Fresh manual02 fixture used for04; claimed first fixture preserved. No assertion weakened or chmod.
- Desktop unit 14, controller 7 and terminal-parser 5 passed separately; these fixture/unit checks alone
  are not the actual GUI/PTy evidence above. No live-provider credentials or private URLs published.
- [App Server](verification/2026-10-01/P02B_APP_SERVER_CONSUMER_EVIDENCE.md): 457 library/client,
  351 protocol/runtime(+1 skip), SDK 27, public RPC 13; actual installed stdio parity and cleanup passed.
  [Standalone](verification/2026-10-01/P02B_SELECTED_SEARCH_EVIDENCE.md): 222 focused; 178 native/worker;
  four races×50=200 no retries; independent build03 and 24-command runtime02 accepted.
- Earlier [StageA](verification/2026-10-01/P02_SEARCH_PREREQUISITE_EVIDENCE.md), native contracts,
  matcher/index/budget/client-shutdown evidence remain historical. Do not relabel old tests as new.

## Ordered next actions

1. Read AGENTS, cloud-runtime skill, canonical roadmap and this state. Revalidate original environment,
   repository/worktrees, remote refs, preserved index/source, active processes, memory and disk.
2. This documentation checkpoint records reviewed evidence/lineage/state for source a469cf4.
   Verify the latest remote ref/publication receipt before new work; do not repeat publication blindly.
3. Required begin_open, both production backends, worker service and runtime are implemented/tested
   in the WIP checkpoint above. Do not redo them. Independent worker04 is built and parked; preserve it.
   AS production stage `/tmp/p02b-preparing-consumers-stage` is adopted (8 files), under serialized
   AS/client library regression `p02b-preparing-app-server-tests`; inspect completion before edits.
   TUI production `/tmp/p02b-preparing-tui-consumers-stage` and tests `/tmp/p02b-tui-pending-tests-stage`
   remain staged only; inspect final combined manifest/review before adoption, then run full TUI gate.
   Complete scoped lint/format, freeze rebuilt full host, test old0.1 and new0.2 installation,
   cancellation while Preparing with sibling, GUI and manager-only SIGINT/cleanup regression.
   Preserve first cause, NotAdmitted, pre-spawn guards, actual join and same-lease handoff fences.
   Worker04 can test backward compatibility using app_server_acceptance.py with frozen fullCLI7a63
   and server-mode codex; standalone CLI harness expects the distinct CLI02 and would fail fingerprint binding.

4. Complete supported replacement/upgrade/rejection/removal and remaining consumer gates; retain
   remote reconnect/daemon and private rollout lookup as explicit gaps. Move shared ownership into P03.
5. Follow P03 broker/transport/dependency ownership, then model/provider/auth, turn/agent orchestration,
   context/history/compaction, tools/execution/security, auxiliary state, configuration/events/platform
   services. Keep native extraction, additive plugins, and adapter-only contracts distinct in inventory.
6. Retest affected streaming/approvals/cancellation/persistence/recovery and GUI in every relevant cycle.
   Actual in-app Browser remains unavailable here; use truthful fallback evidence without blocking core work.
7. Implement required P18U upstream integration/update component after contracts stabilize: original
   path/symbol maps, isolated chosen-revision candidate, compatibility/security/real-host/UI gates,
   migration, coordinated host/plugin versions, rollback artifacts and independent bootstrap recovery.
   Demonstrate one real upstream update plus breaking/failed rollback; no periodic polling yet.
8. Preserve and publish verified milestones non-destructively. Continue successive feasible components;
   run/resources are finite and remaining whole-platform work must never be described as finished.

## Recovery and reproducibility

Primary `/workspace/codex-harness-everythings-a-plugin`; isolated recovered WIP
`/workspace/codex-harness-next-components`. Origin DesignStuffDev/codex-harness-everythings-a-plugin.
Official source pin `d42056091aded7feb1d88ac7e83972108b2aa478`; retain Apache LICENSE/NOTICE and Nucleo MPL.
Original work branch remains at upstream import. Root audited all 8,834 staged entries equal HEAD.
Current index metadata SHA `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`;
older 2cc2/2e0 images remain preserved. Never reset/clean/bulk-stage either worktree.
Git shell authentication fails; connector publication succeeds. Local main/WIP refs intentionally lag
remote main/WIP publication receipts; revalidate remotes and use exact parent/tree objects. Do not invent a
same-tree local commit or reset original work to hide ref drift. Publication reports are authoritative.

Recovery directory `/workspace/recovery-backups/20260930T165936Z/`:
- Full both-tree/SDK/docs/evidence/Git `codex-recovered-workspace.tar.zst`, SHA
  `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
- Published source checkpoint `p02b-full-consumer-source-checkpoint.tar.gz`, SHA
  `d570aed0ae603bba00c5f3a95a4b2109954577d2d44adb26e1051c461e6067ad`.
- Post-format 223-path `p01-source-20261001T084942Z.tar.gz`, SHA
  `acf2258611dec4e7f35b4bd1a27bec4dbaea1d8d2031c97fbf26eb215eed94d5`; keep 084914 preformat too.
- Publication reports: `p02b-full-consumer-source-publication.json`, prior AS source/docs and CLI reports.
- Three completed generated test executables retained as exact verified gzip archives before only their
  mutable cache aliases were retired. Report `p02b-completed-test-cache-preservation.json` preserves
  identities/checks. Source, evidence, frozen hosts, libraries, depfiles/fingerprints protected.
Archives and binaries are cloud-local checkpoints. GitHub gives external durability only to published files.

Frozen fullCLI: `/workspace/component-checkpoint-candidate-p02b-full-cli-20261001/`.
GUI04 uses unchanged original CLI02 manager for exact independent-build fingerprint (mode included).
Preserve independent search package03, storage0.2 packages, old frozen AS/CLI/GUI hosts and screenshots.
Source `/workspace/toolchains/component-verification-env.sh`; Rust 1.95/jobs1/debug0/incremental0.
One owner serializes Rust; use just test/scoped fix/fmt and required schemas/Bazel lock. Wrapper
`/tmp/run-p02-check.py` records actual scopes; old run-component-regression.py has a stale source label.
Check resource/mapping state before builds; never erase active caches, unverified source or mapped files.
Optional `/workspace/remote-viewer-setup` remains separate; no verified local-browser bridge/preview.
Do not reset/replace this VM or let optional viewer networking stop component implementation.
