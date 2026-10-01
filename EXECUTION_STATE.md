# Execution state — read first on resume

Updated 2026-10-01 UTC. Follow [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md), [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Full v1 is incomplete. This file is a concise queue; exact historical evidence stays linked below.

## Durable checkpoints and current source

- Previous main documentation checkpoint `f2cc6c2ed7dd84f606d43947a3c71f85bfc0daad`, tree
  `8eae4fda3b4ed65de2a4d0bef9b552636940204e`: verified documentation/lineage publication,
  parent `562554acf45a39cb82cd62df65cf3b6627dc9011`. Accepted runtime source was unchanged.
- Latest published source WIP `30c2674600cc84da161562c407515b61511b7c3a`, tree
  `86c0bce7a5b399c2337c7c3878ed12ee76da4918`, on `wip/p02b-bounded-search-20261001`;
  parent selected-CLI WIP `651b0b87a281934e5cd7c639021049fad8d393ac`. Connector publication
  and remote ref verified. All 8,739 frozen AS build fingerprints independently match the exact tree.
- App Server/client has passing library/contract/public RPC and real installed-worker gates below.
  Source is externally preserved; this documentation checkpoint records its evidence and lineage.
  Earlier CLI proof remains; existing GUI/storage proof is older-engine. This is not all consumers.
- Root applied 26 TUI paths after the AS freeze. First selected gate ended exit 101 at linker SIGBUS,
  zero tests executed; only Cargo.lock changed across its scope. Disk-zero was observed, not proven
  ENOSPC. Retry passed 23/23; full TUI regression passed 5,620, four skipped, unchanged scope.
  Five SDK PTY files were applied afterward. Scoped lint passed with one equivalent
  condition collapse; a test-only helper was then restricted to test builds.
  Formatting and Bazel lock refresh passed separately; the full CLI build is running.
  New full-CLI runtime remains pending. Desktop Python14, controller7 and terminal
  observer5 passed in separate scopes; fixtures/observer checks are not real-engine proof.
- Upstream remains OpenAI Codex `d42056091aded7feb1d88ac7e83972108b2aa478`.
  Retain Apache LICENSE/NOTICE and pinned Nucleo MPL notices/provenance.
- Native thread persistence and inline attachment storage are extracted subsets. Model/auth
  remain adapters. Native loop/context/compaction/tools/policy and auxiliary state remain coupled.
  The independently packaged GUI is additive presentation; do not count it as native extraction.

## Most recent verified gates — separate scopes, no invented total

- [App Server consumer evidence](verification/2026-10-01/P02B_APP_SERVER_CONSUMER_EVIDENCE.md):
  `p02b-as-client-tests-04` passed 457/457 (App Server 415 + client 42), zero skipped/retries,
  all 8,739 scoped fingerprints unchanged, child/subreaper exit 0, no runner error.
  Initial production import failure, second test import failure, and 456/457 assertion failure
  remain preserved. The test-only correction requires the full actual `ownership is closing` cause.
- `p02b-as-protocol-runtime-tests`: 351/351 passed, one ignored schema-generation helper;
  SDK contract/API tests 27 passed. Each has unchanged recorded scope and clean subreaper.
  Stable/experimental schema generation and Bazel lock regeneration completed separately.
  First stable schema scope omitted Python SDK output fingerprints; later captures bind resulting bytes.
- `p02b-as-public-search`: 13/13 passed, 1,448 filtered, unchanged scope and clean subreaper.
  Explicit empty-query fixture correction preserves snapshot/identity/completion barriers.
  Scoped fix passed unchanged; format passed with 26 changed Rust/Python paths, no format-only retest.
  Locked App Server build passed with 8,739 unchanged paths. Frozen AS SHA256
  `a4ca81bb835f8b1883f5fb702982055ebf7161fffd49b80e81265a9fb2500a0a`.
- Real `p02b-as-search-independent-01` acceptance passed: reused separate package03, seven exact
  native/installed/restored cases, concurrent sessions/ABA/clear/stop/sibling isolation, real typed
  resource exhaustion, failed config without fallback, removal and EOF shutdown. Six management/
  config commands and four RPC server runs had expected statuses (resource-error shutdown 1).
  All 40 tracked PIDs absent, host/package/tooling unchanged, outer subreaper 0 with no runner error.
  Retain adopted Git helper 524152 SIGPIPE(-13); do not claim every child exit was zero.
- [Selected standalone search](verification/2026-10-01/P02B_SELECTED_SEARCH_EVIDENCE.md):
  combined 222/222; native/worker 178/178; four race cases x50=200 executions, no retries.
  Independent build03 succeeded, then runtime02 passed 24 expected-status CLI/manager commands.
  Exact native/installed parity, real Ctrl+C 130, selected failure, removal/restoration and PID absence.
  Original compile/config/race/startup/lock failures remain historical evidence, not green runs.
- Later TUI (outside AS source): selected retry 23 passed with 5,601 filtered; full regression 5,620
  passed with 4 skipped. Both used zero retries, 10,112 unchanged scope entries and subreaper 0.
  Preserve adopted-child nonzero statuses in raw reports; runner 0 does not mean every child exited 0.
  Reports: `/workspace/acceptance/p02b-tui-{selected-tests-02,regression}.source.json`.
- Passing native T12 profile is recorded by `p02b-interactive-native-profile`: one test/eight rows,
  78 filtered. At 100k/T12 with highlights, fixed charged storage 11,069,744 bytes and 26 workers;
  128 MiB remains the ceiling. This is a small traversal/matching fixture, not full-scale or RSS proof.
- [StageA engine/storage/GUI](verification/2026-10-01/P02_SEARCH_PREREQUISITE_EVIDENCE.md),
  source `25b2c3150879761026086a62e480045fc38d32ff`: last accepted GUI engine checkpoint.
  Existing storage 0.2.0 package reuse/migration and GUI 49+38 Chromium commands passed.
  Actual manager Launch SIGINT during active turns exited 0 and all tracked descendants disappeared.
  These are older-engine results using deterministic inference, not new consumer/Browser/live-model proof.
- Historical detail: [P01](verification/2026-10-01/P01_TEST_EVIDENCE.md),
  [contracts](verification/2026-10-01/P02B_CONTRACT_EVIDENCE.md),
  [matcher fix](verification/2026-10-01/P02B_MATCHER_FIX_EVIDENCE.md),
  [bounded index](verification/2026-10-01/P02B_BOUNDED_INDEX_EVIDENCE.md),
  [native budget](verification/2026-10-01/P02B_NATIVE_BUDGET_EVIDENCE.md),
  [native backend](verification/2026-10-01/P02B_NATIVE_BACKEND_EVIDENCE.md),
  [process adapter](verification/2026-10-01/P02B_SEARCH_PROCESS_EVIDENCE.md),
  [client shutdown](verification/2026-10-01/P02B_CLIENT_SHUTDOWN_EVIDENCE.md).

## Ordered next actions

1. Revalidate original environment, repository/worktrees, refs, source/index, active processes,
   disk and memory. Read AGENTS and cloud-runtime skill. Preserve current source before edits.
2. Verify this App Server evidence/lineage documentation checkpoint on remote main.
   Source `30c267` is already remotely verified; all mapping uses immutable tree/build bytes.
   Recheck remote refs before every non-force publication; preserve the original work index.
3. Finish the full CLI build, preserve post-regression tooling and freeze its actual output.
   Keep original SIGBUS failure/lock transition and passing retry/regression source identities distinct.
4. Complete real installed TUI selection/stop/reconnect acceptance and existing storage/GUI regression
   against the appropriate new host. Recheck manager Launch Ctrl+C. Chromium fallback is useful
   but is not in-app Browser evidence; no GUI/model proof comes from the search-only AS run.
5. Preserve the successful AS runtime/package03 proof and immutable binaries; no repeat test solely
   for formatting. Follow [the Preparing cancellation queue](component-sdk/FILE_SEARCH_PREPARING_CANCELLATION_PLAN.md)
   for observable per-request control/cleanup, a newly built worker and old-package compatibility.
6. Finish remaining P02 consumers/package compatibility, upgrade/rejection/removal and real custom
   replacement checks. Private rollout lookup is a fourth consumer pending P03 broker; keep it visible.
7. Continue P03 ownership/transport/dependency broker, then the roadmap's model/provider/auth,
   turn/agent orchestration, context/history/compaction, tools/execution/security, auxiliary persistence,
   config/events and remaining UI/platform services. Keep every inventory row's proof and gaps explicit.
8. Implement required P18U upstream-update maintenance component after contracts stabilize:
   original-path/symbol mapping, isolated chosen-revision candidate, compatibility/security/regression/
   real-host/UI gates, coordinated host/plugin versions, migration and independent-bootstrap rollback.
   Demonstrate a real upstream integration plus failed/breaking update recovery before final release.
   No periodic polling or live installation update is authorized merely by the roadmap.

## Preserve and resume safely

Primary `/workspace/codex-harness-everythings-a-plugin`; recovered isolated WIP
`/workspace/codex-harness-next-components`. Origin is DesignStuffDev/codex-harness-everythings-a-plugin.
Original work branch remains at official import `d42056091aded7feb1d88ac7e83972108b2aa478`.
Index metadata bytes changed; all 8,834 staged entries still match that HEAD per root's preservation audit.
Current index SHA256 `2cc2f0dc3e0724bb879d15e02927bf798c4884fee70018a77ef17f31fffdd598`
exactly matches `original-work-index-20261001T0728.bin`; historical `2e0abf...` was an earlier byte image.
Never reset/clean/bulk-stage either tree. Use whitelisted files and a temporary publication index.
Git shell authentication currently fails; connector publication succeeded. Local WIP remains `651` and
local main remains `f2cc6c2`; remote `30c267` is not fetched, but exact local tree `86c0bce7` objects exist.
Use verified remote parents plus exact trees for future publication; never invent a same-tree local
commit or reset original work to repair ref drift. Publication reports and fresh remote checks are
the source of truth; pending main documentation publication can leave local refs intentionally stale.

Recovery directory `/workspace/recovery-backups/20260930T165936Z/`:
- Full both-worktree/SDK/docs/evidence/Git archive `codex-recovered-workspace.tar.zst`, SHA256
  `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
- Frozen AS incremental `p01-source-20261001T073424Z.tar.gz`, 185 source members, SHA256
  `aa6d2f9e70aae809b4619d8a71c5a44f505ef80f01a5bda71e9ce083527e5159`.
  All members verified; AS build source_after matches 170 delta paths; published tree matches all 8,739.
  Source `30c267` includes 62 changes over `651`; runtime README received a documented status-only update.
  Keep earlier 072520 and later TUI 074033 (210 paths; SHA fd9061224c5a79c3ecac3b5eb317c29f770f642f946bf983f21dee94261c948e).
- Published standalone source archive `p02b-selected-cli-tested-source.tar.gz`, SHA256
  `b077447af6c7d80e09c371a11f07f95e55e70803e3819d62d0ab09b7ac1bdb36`, bound to WIP 651.
Cloud-local archives are recovery checkpoints; GitHub gives external durability only for published files.
Preserve original failed logs, unused staged drafts, older archives and both saved worktrees.

Frozen AS: `/workspace/component-checkpoint-candidate-p02b-app-server-20261001/` (unchanged copy, nlink 1).
Frozen CLI02/manager: `/workspace/component-checkpoint-candidate-p02b-search-cli-20261001-02/`.
Independent package03: `/workspace/acceptance/p02b-search-independent-03/package` (source parked).
Earlier GUI engine: `/workspace/component-checkpoint-candidate-p02a-20261001/`; keep its immutable hashes.
Source `/workspace/toolchains/component-verification-env.sh`; Rust 1.95/jobs1/debug0/incremental0.
One owner serializes Rust. Use `just test`, scoped `just fix`, `just fmt`, required schemas/Bazel lock.
Use `/tmp/run-p02-check.py`; obsolete `/tmp/run-component-regression.py` has a stale source label.
Never kill Rust or erase active caches; 32 GiB overlay requires current resource/mapping checks before builds.
Generated-cache symlink backing files and frozen executable hardlink detachment must remain intact.

Optional viewer `/workspace/remote-viewer-setup` is isolated from harness work. Prior enforced policy
had no additional hostname grant; user-accessible preview/remote Browser reachability is unverified.
Recheck supported policy before claiming connectivity. Never reset/replace this VM to bypass the blocker.
Routine implementation/testing and verified non-force publication are authorized. Continue feasible work;
finite run/resources and unavailable in-app Browser are limits, not evidence the platform is finished.
