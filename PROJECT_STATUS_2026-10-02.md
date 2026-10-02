# Project status — 2026-10-02

The project is a working partial component platform, with three bounded native functionality families independently installed and exercised, a developer SDK, and a separately packaged GUI using the real Codex engine. Most engine behavior still lives in the host. The finished “everything is a plugin” platform, minimal kernel, complete updater and release acceptance have **not** been achieved.

This checkpoint distinguishes published source, completed tests of the newer working candidate, and proposals awaiting adoption. It does not promote the candidate to main or transfer an older runtime result to newer source.

## Published checkpoints and current work

Repository: [DesignStuffDev/codex-harness-everythings-a-plugin](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin). The imported official baseline is OpenAI Codex `d42056091aded7feb1d88ac7e83972108b2aa478`; upstream provenance, LICENSE and NOTICE are retained.

| Checkpoint | Exact identity and meaning |
| --- | --- |
| Verified main | `781080f7e3c8bfe1953378001d777dff33d74bc3`; native installer implementation remains source `65511842d7051b2a1f5cc52917f3ebb5c03be4f3`. |
| Published development branch | `work/p03-curated-sync-lifecycle` at `ba87799c2e9ab821a326f10715c11cf6ec256896`. Includes the callback source milestone with 953 library tests, lint and reviewed formatting; no fresh full-host result for that source. |
| Latest actual rebuilt-host runtime | Native source `92212516ad4d12bcf60546ee5f879b983ed44682`, runtime evidence publication `2ac44529d0dfa261c6d2a09005a34b3ee22754ed`. This predates the later completion/callback/process-final changes. |
| Current working candidate | 56-path process-final/replacement candidate, preserved as unfinished WIP, tested virtual tree `1da290682fc6ddd00994c2af7eac9018c8143dea`, before subsequent documentation edits. Five completed native suites are recorded below. |
| Verified preservation branch | `wip/p03-process-final-and-mcp-preservation-20261002` at `d989351c0f016a3f8c8bc2ea3248c1c19e915dd5`, from published `ba87799…`. All270 selected blobs were read back. This preserves active candidate source and inert proposals; it does not mark them accepted. |

The original checkout, sibling worktree, failed evidence, proposal history and frozen GUI are preserved. VM-local recovery archives are not external backups. External durability applies only to files actually included in verified published GitHub commits.

## Native functionality actually separated

| Area | What the evidence establishes | What remains outside that claim |
| --- | --- | --- |
| Thread persistence and manual rollout migration | A separately built native `LocalThreadStore` worker is selected by production callers. Installed-package tests cover real session storage/recovery and the P01 manual migration path, including dry-run parity, apply/reapply, cancellation and close. | Auxiliary SQLite repositories, all metadata domains, backfill, repair, reclamation and every storage-maintenance operation are not extracted. |
| Inline attachments | Separately packaged native inline upload/resolve implementation. Manager-level acceptance includes byte-preserving 5 MiB upload, the original `NotFound` resolution behavior, and component uninstall. | Inline storage persists no blobs. This is not attachment deletion, durable remote storage, or comprehensive engine-caller/partial-transfer/cancellation proof. |
| Bounded file search | Separately installed native traversal/matching worker, with actual CLI, App Server, TUI and GUI consumers. Recorded parity, streaming, failure without silent fallback, cancellation, shutdown and package-version compatibility tests. | Private storage lookup, pending-Open shutdown and remaining watch/filesystem/patch/Git/worktree/import behavior remain separate work. |

The acceptance distinction matters: these native packages were built outside the harness, installed after the host was built, and exercised without changing its executable hash. A crate split, adapter or test fixture alone does not meet that standard. The inventory covers 169 registered Rust members and supporting/staged packages across 28 ownership families; it is not a count of completed extractions.

## Infrastructure, adapters and the GUI

The component API, process host and adapters support installation, selection, removal, manifest/contract checks, configuration and component lifecycle. Persistent storage transport is active. Broker declarations, bounds, handles, grants and offer/acknowledgment structures have scoped implementation/test evidence; generic runtime negotiation, quota enforcement and granted dependency-service execution remain inactive.

The developer SDK includes a versioned Python wheel, template generator, zipapp packaging, native Rust source/package tools and examples. Recorded acceptance covers outside-checkout builds, fresh installation, source removal, SDK removal and unchanged host hashes. The wheel is locally built, not a published package-registry release. Linux/Python proof does not establish all supported platform/version combinations. Components are trusted executable code with user privileges, not proven hostile-plugin isolation, and do not install into the official Codex desktop application.

Selected model streaming and authentication resolve/refresh have real adapters. Tool, context and lifecycle contributors add capabilities. They do **not** establish independent packaging of the full native inference, auth, context or tool-execution implementations. Newer model transport, native catalog and context/replay work remain staged or coupled.

The separate GUI owns its HTML/CSS/JavaScript and Python gateway, using the real App Server for saved tasks, streaming, native shell approvals, interruption and recovery. It supports selected file search and headless operation with the GUI absent. It is an additive presentation client, not extraction of App Server or the official desktop UI. Login, voice/realtime, image attachments, widgets, biometrics and worktree-management parity are incomplete.

GUI evidence uses Chromium/Playwright and deterministic inference fixtures. In-app Browser testing, live-provider inference and a user-accessible remote viewer remain unverified. The optional viewer networking limitation does not prevent source development.

## What has actually passed

**Latest full runtime remains source922.** A fresh CLI build passed in 6m 25s with matching source maps and executable fingerprints. Existing independently built storage/search packages were reused unchanged. The recorded gates passed 13 storage commands, 10 migration commands and two cold GUI cycles, each with strict runner exit 0 and no runner error. GUI covered streaming, shell approval, cancellation, recovery, continuation, search insertion and actual manager SIGINT during an active turn. Manager shutdown took about 0.265 seconds, with tracked processes absent and no forced cleanup.

Those fast runs do not prove deliberately held Git/HTTP cleanup or cleanup exceeding the old 45-second watchdog. Source922 lacked the later production stop/join/callback contract. Earlier source99 runs with surviving Git/HTTPS descendants remain failed evidence. Source922 also recorded adopted Git SIGPIPE exits; it is not an all-children-exited-zero result.

The newer **WIP process-final candidate** has these actual completed checks:

| Scope | Passed | Qualification |
| --- | ---: | --- |
| Core plugins, transport, arg0 and process utilities | 703 | 536 + 157 + 7 + 3; zero skips. |
| App Server library | 431 | Includes two slow-store cases using paused virtual time. |
| App Server client | 42 | Unchanged-source retry after disk-exhaustion link failure; seven adopted SIGPIPE statuses remain unattributed. |
| Exec | 73 | 69 library and four executable tests; production-host acceptance still separate. |
| CLI | 299 | One ignored `blocked_probe_fixture` helper is not counted passed. |

All five successful native runs have unchanged before/after source maps, zero retries and strict runner success with no drain error. Fourteen desktop transport-fixture tests also passed on the candidate before its one-character compiler repair; they did not run a real browser/engine. The initial compiler failure and disk-full linker failure ran zero tests and remain separately preserved. Historical suite counts overlap and are not a project-wide coverage total.

The TUI library test build failed before test execution: rustc received SIGKILL after about 36 minutes, with source unchanged. The VM had high memory pressure and about 592 MB free disk; no pre-run OOM counter baseline was captured, so the precise cause is not conclusively established. That failure is retained separately. Resource recovery and a TUI retry, manager checks, changed App Server integration cases, scoped lint/global formatting and rebuilt-host runtime gates remain outstanding.

## Why P03 lifecycle work is still the immediate priority

The surviving-descendant failures exposed real ownership gaps in native curated-plugin synchronization. Published work now provides bounded Git output/cleanup, retained locks/temporary directories/transport obligations, sticky quarantine on uncertain cleanup, shared cancellation, policy-aware bounded HTTP collection, exact worker-handle observation and owned callback scopes. Native HTTP/runtime teardown can still outlast an observer deadline; dropping a request future is not proof that every backend/DNS task has joined.

The current tested candidate adds actual process-final callers on success/error, admission fencing, separate embedded callback lifetimes, same-home replacement delivery, structured curated-ownership outcomes and coordinated shutdown clocks. Its policy is one 200-second graceful deadline, forced-exit initiation at 205 seconds, GUI bounds of 207/209 seconds within the manager's 210 seconds. Forced outcomes preserve cleanup/durability uncertainty. This is a tested candidate contract, not completed installed-host slow-cleanup proof.

The remaining acceptance is concrete:

- Rebuild and bind the real CLI/manager, then hold actual native Git and HTTP startup during exec success/error and App Server EOF/SIGTERM. Require explicit ownership outcomes plus unchanged strict descendant checks.
- Repeat installed storage, migration and GUI streaming/approval/cancel/recovery/Launch shutdown with the new host.
- Exercise real relay-held installed-storage cleanup exceeding 45 seconds and forced uncertainty. The revised relay has four synthetic checks only; those short synthetic holds do not prove the 46-second production case or admission into native storage.
- Prove same-process A→B replacement using a held real worker, with no stale A effects and one refresh for active B. Current routing tests and installed storage replacement do not prove this case. Different-home global-once support remains explicitly limited.
- Close exact MCP prewarm, refresh, service and transport custody. A callback finishing or requesting MCP invalidation is not proof that MCP work joined. `curated_ownership_clean` does not mean the entire host is clean.
- Adopt/test cooperative extraction separately, and implement recoverable repository/SHA publication and host-death fencing. The frozen C2b extraction proposal is unadopted; its cooperative limits are not a total ZIP-constructor memory or syscall-duration bound.

These repairs are prerequisites for safe extraction, not additional independently installed subsystems. Native auth persistence/load/refresh authority and installed auth/catalog activation still follow them.

## Preserved proposals, not accepted code

MCP upper02 plus retirement03, the 43-file lower workspace proposal and five-file pinned SDK proposal remain frozen, statically reviewed, **unadopted and uncompiled**. They cover session errors/retained handles, exact local process/service/transport custody and observable HTTP worker/SSE completion. Proposed tests have not run. Adoption requires their recorded dependency order, an approved immutable SDK source/pin and consumer integration, then real transport/process tests and a final aggregate/late-admission fence. The SDK patch alone does not change the workspace's HTTP completion classification; ordinary optional remote DELETE failure is separate from local ownership uncertainty.

A reviewed allowlist selects **204 source/preimage/patch/license/documentation files, 7,971,037 bytes**, for inert WIP preservation. It includes the original and corrected slow-store fixtures and original frozen proposal history, excludes private runtime homes/auth data, caches, binaries and generated replay trees, and preserves Codex and pinned upstream license/provenance material. Preservation manifest SHA-256: `42ebfc9ec99ece1a596223ced049695374271c44db5e10b5fc16ba47848760b7`; allowlist SHA-256: `45630ed70152b44237de33af9a50125fdf636a16f8edb65fc196b6511aaad521`. Publication and exact remote readback completed at `d989351c0f016a3f8c8bc2ea3248c1c19e915dd5`; preserving WIP does not adopt it.

## Remaining platform roadmap

| Phase | Required work / current position |
| --- | --- |
| P00 / P00M | Baseline, inventory and execution ledger exist; maintain and close current semantic provenance, not just historical path/blob maps. |
| P01 / P02 | Accepted manual migration and bounded search subsets; finish their remaining persistence/search ownership gates. |
| P03 | Current lifecycle/transport/authority prerequisite; activate and prove broker services, complete the host contract, then installed auth/catalog work. |
| P04–P05 | Configuration, credentials/secrets, full native authentication, catalogs/providers and inference transport. |
| P06–P09 | Remaining state repositories/maintenance/attachments, replay, live history/context/prompt construction and compaction. |
| P10–P12 | Approval policy, OS sandbox/process execution, native tools/routing and code mode. |
| P13–P15 | MCP/connectors/skills/plugins, session/turn/multi-agent orchestration, memory/goals/queues/hooks and background services. |
| P16–P17 | Events/projections/telemetry and remaining presentation, CLI/TUI, cloud/remote/media behavior. |
| P18 / P18U | SDK/contract/platform/release conformance and upgrade/rollback; independently installable upstream maintenance plus external recovery bootstrap. |
| P19 | Minimal-kernel audit and complete unchanged-host custom-component, GUI/headless, lifecycle/recovery and update acceptance. |

**The updater is required and unimplemented.** Current maintenance support is a read-only provenance inventory/checker at a pinned checkpoint, with 22 fixture passes and 1,167 unresolved semantic/evidence/source findings. These are mapping findings, not current runtime test failures. There is no active maintenance component or automatic update service.

P18U must integrate a real later pinned upstream revision in an isolated candidate, classify changes across owned boundaries/schemas, retain compatible custom package digests/configuration/sessions, reject breaking compatibility, and demonstrate failed-activation/migration rollback. An independent bootstrap must recover the accepted distribution when the candidate host/updater cannot start. A clean textual merge does not satisfy that gate. No polling, live update or deployment has been scheduled.

The intended finished host remains a small bootstrap/composition/compatibility/authority/lifecycle supervisor. Native domain behavior—including current core/session implementations—must become independently replaceable without rebuilding it; restart activation is sufficient. Completing that transformation and its end-to-end release proof remains substantial work, with no defensible single completion percentage.

## Evidence and source anchors

- [Current execution ledger](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/EXECUTION_STATE.md), [canonical roadmap](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/IMPLEMENTATION_ROADMAP.md), [component inventory](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/COMPONENT_INVENTORY.md), [upstream-maintenance requirements](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/UPSTREAM_MAINTENANCE.md).
- [Source922 full-host evidence](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/verification/2026-10-02/P03_SOURCE922_FULL_HOST_EVIDENCE.md), [callback milestone](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/verification/2026-10-02/P03_CURATED_CALLBACK_SCOPE_EVIDENCE.md), [developer SDK](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/component-sdk/README.md).
- [Exact candidate manifest](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/_preserved_wip/2026-10-02/process-final-source-manifest.json), SHA-256 `0a46a9a3f50670a4cc10180012dd3adb09ee2226c75efb8db8c86fbb093d4065`. Its pre-run status is historical; later actual results are separately bound in `/workspace/acceptance/p03-process-final-{native-tests-02,app-server-tests-01,client-tests-02,exec-tests-01,cli-tests-01}.{source.json,subreaper.json,log}`.
- [WIP preservation plan](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/_preserved_wip/2026-10-02/PLAN.md) and [exact allowlist](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/_preserved_wip/2026-10-02/ALLOWLIST.json).

Prepared from the current checkout, recorded publication receipts and actual five-suite summary/strict reports; subsequently updated after verified WIP publication. Test execution and source publication are recorded separately.
