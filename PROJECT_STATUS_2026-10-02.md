# Project status — 2026-10-02

The project is a working partial component platform, with three bounded native functionality families independently installed and exercised, a developer SDK, and a separately packaged GUI using the real Codex engine. Most engine behavior still lives in the host. The finished “everything is a plugin” platform, minimal kernel, complete updater and release acceptance have **not** been achieved.

This checkpoint distinguishes published source, completed tests of the newer working candidate, and proposals awaiting adoption. It does not promote the candidate to main or transfer an older runtime result to newer source.

## Current recovery and development checkpoint

The original VM and source are recovered and verified. Full-source and test-fixture archives
match their saved checksums; the temporarily relocated archive is restored. Carefully bounded
cache retirement recovered enough space for the actual CLI/manager build, which passed on the
current58 source cohort. The earlier ENOSPC, compiler SIGKILL and connection failures are retained
as historical evidence, not current executor blockers. No replacement VM or checkout was used.

Current-host runtime gates now pass for independently installed thread storage, manual migration,
ordinary GUI and independently installed-search GUI, plus deliberately slow normal and forced
launcher shutdown. Normal slow shutdown waited46.469 seconds and recovered the canonical event
in a cold GUI. Forced shutdown exited nonzero with durability explicitly unknown; all tracked
processes were absent. These are real CLI/App Server/Chromium runs with deterministic inference.
They are not live-provider or in-app Browser proof, nor proof of complete host cleanup.

Three SDK acceptance files now support explicit, hash-pinned reuse of a previously independently
built search package with a newer unchanged host. That is test infrastructure, not a new extracted
engine component. The complete tested source map, executable hashes, failed attempts and precise
limits are in [the recovered-host report](verification/2026-10-02/P03_RECOVERED_HOST_EVIDENCE.md).
Current priorities remain the held Git/HTTP cleanup matrix, same-process replacement, exact MCP
custody and recoverable publication before the next native auth/catalog extraction.


**Held-host03 terminal result:** four real Git-held cases passed all native ownership, independent
executable-observation and strict descendant checks (exec success/error; App Server EOF/SIGTERM).
The fifth Git-failure→HTTP-held case passed its local native/strict checks, but the independent
sampler missed the short-lived Git executable. Its parent gate failed; it is not counted passed.
Three remaining cases did not run. No forced fixture cleanup occurred in these five cases.
The matrix remains failed/incomplete, and MCP/session ownership remains separately unverified.
See [sanitized exact receipts](verification/2026-10-02/P03_RECOVERED_HELD_HOST03.json). A bounded
positive-observation handshake is being reviewed; do not bypass the independent observer check.

## Published checkpoints and current work

Repository: [DesignStuffDev/codex-harness-everythings-a-plugin](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin). The imported official baseline is OpenAI Codex `d42056091aded7feb1d88ac7e83972108b2aa478`; upstream provenance, LICENSE and NOTICE are retained.

| Checkpoint | Exact identity and meaning |
| --- | --- |
| Verified main | `781080f7e3c8bfe1953378001d777dff33d74bc3`; native installer implementation remains source `65511842d7051b2a1f5cc52917f3ebb5c03be4f3`. |
| Published development branch | `work/p03-curated-sync-lifecycle` at `ba87799c2e9ab821a326f10715c11cf6ec256896`. Includes the callback source milestone with 953 library tests, lint and reviewed formatting; no fresh full-host result for that source. |
| Latest actual rebuilt-host runtime | Current58 source map `15e9a44f54f414972eff02de9cd4afdfa5f900263d46b69a96119499779c136b`; CLI SHA256 `d716c6ee9c738c2db9197fbb2ab91d3918839d0b6d32f4556e8a7e1fa2717a92`. Exact evidence below. Historical source922 remains separate. |
| Current working candidate | The same56-path native candidate plus three SDK edits (two shared old paths, one new path), yielding58 changed paths and8,921 scoped source files. Historical native suites remain explicitly bound to their original cohort. |
| Verified source preservation | `wip/p03-process-final-and-mcp-preservation-20261002`: active candidate and204 inert proposal files preserved at `d989351c0f016a3f8c8bc2ea3248c1c19e915dd5`; latest source-preservation checkpoint `1f808b9fcac8fc2f548b73bc0e486459bc9c59ce` adds45 inert observation/callback/replacement-acceptance files and manager evidence. All changed blobs were read back. This later documentation update adds no native implementation and does not mark WIP accepted. |

The original checkout, sibling worktree, failed evidence, proposal history, archives and frozen GUI were reverified after recovery. VM-local recovery archives are not external backups. External durability applies only to files actually included in verified published GitHub commits.

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

**Current58 has real rebuilt-host proof.** The production build completed in about75 seconds,
with unchanged8,921-file before/after maps and exact executable hashes. Storage passed13 recorded
behavior assertions; each migration gate passed10 commands. Ordinary and installed-search GUI
modes each passed two cold cycles with streaming, shell approval, cancellation, continuation,
search insertion and actual manager SIGINT. Ordinary shutdown took about0.265–0.268 seconds,
with tracked processes absent and no emergency cleanup. All successful checks use the unchanged
strict runner with exit0 and no drain error; adopted statuses are reported individually, not
assumed allzero. Reused packages retain original independent-build evidence.

Eight selector and12 synthetic wire checks preceded the separate real slow normal/forced runs.
Normal shutdown crossed the old45-second limit and recovered the held canonical UI event; forced
termination reported uncertainty. The relay holds before native storage admission, so this is
not a native durability-admission or exact App Server watchdog-onset test. Slow01's raw-input/UI
projection gap remains unfixed; slow03's incorrect wire selector timeout remains failed evidence.
The held Git/HTTP matrix has no pass yet:01/02 failed before complete readiness on a separately
observed auxiliary ChatGPT CONNECT. Corrections retain exact transport/descendant assertions.

Historical source922 runtime evidence and earlier source99 surviving-Git failures remain separate.
Their successful builds/tests do not establish the behavior of later source. Current runtime uses
real Chromium/Playwright with deterministic inference, not live inference or in-app Browser.

The newer **WIP process-final candidate** has these actual completed checks:

| Scope | Passed | Qualification |
| --- | ---: | --- |
| Core plugins, transport, arg0 and process utilities | 703 | 536 + 157 + 7 + 3; zero skips. |
| App Server library | 431 | Includes two slow-store cases using paused virtual time. |
| App Server client | 42 | Unchanged-source retry after disk-exhaustion link failure; seven adopted SIGPIPE statuses remain unattributed. |
| Exec | 73 | 69 library and four executable tests; production-host acceptance still separate. |
| CLI | 299 | One ignored `blocked_probe_fixture` helper is not counted passed. |
| Component manager | 88 | Six launcher lifecycle cases passed; one ignored parent-death subprocess helper is not counted passed. Short injected budgets do not prove the production 210-second grace. |

All six successful native runs have unchanged before/after source maps, zero retries and strict runner success with no drain error. Fourteen desktop transport-fixture tests also passed on the candidate before its one-character compiler repair; they did not run a real browser/engine. The initial compiler failure and disk-full linker failure ran zero tests and remain separately preserved. Historical suite counts overlap and are not a project-wide coverage total.

The TUI library test build failed before test execution: rustc received SIGKILL after about 36 minutes, with source unchanged. The VM had high memory pressure and about 592 MB free disk; no pre-run OOM counter baseline was captured, so the precise cause is not conclusively established. That failure is retained separately. Guarded resource recovery reclaimed roughly 4 GB of RAM-backed cache storage while retaining exact archives of inactive test fixtures. Manager checks then passed. A TUI retry, changed App Server integration cases, scoped lint/global formatting and remaining held-host/replacement runtime gates remain outstanding.

## Why P03 lifecycle work is still the immediate priority

The surviving-descendant failures exposed real ownership gaps in native curated-plugin synchronization. Published work now provides bounded Git output/cleanup, retained locks/temporary directories/transport obligations, sticky quarantine on uncertain cleanup, shared cancellation, policy-aware bounded HTTP collection, exact worker-handle observation and owned callback scopes. Native HTTP/runtime teardown can still outlast an observer deadline; dropping a request future is not proof that every backend/DNS task has joined.

The current tested candidate adds actual process-final callers on success/error, admission fencing, separate embedded callback lifetimes, same-home replacement delivery, structured curated-ownership outcomes and coordinated shutdown clocks. Its policy is one 200-second graceful deadline, forced-exit initiation at 205 seconds, GUI bounds of 207/209 seconds within the manager's 210 seconds. Forced outcomes preserve cleanup/durability uncertainty. Current58 now has bounded installed-host slow-cleanup proof; exact MCP and full-host custody
remain unproved.

The remaining acceptance is concrete:

- Build/source binding is complete. Finish actual held native Git/HTTP startup during exec success/error and App Server EOF/SIGTERM; require exact ownership outcomes and unchanged strict descendant checks.
- Preserve completed current58 storage/migration/GUI/slow normal-and-forced proof. Re-run relevant gates when later material native changes require a new host.
- The real relay-held installed-storage normal/forced cases now pass, including46.469s cleanup. Native-internal admission and the response-only interrupted UI-history gap remain outside that proof.
- Prove same-process A→B replacement using a held real worker, with no stale A effects and one refresh for active B. Current routing tests and installed storage replacement do not prove this case. Different-home global-once support remains explicitly limited.
- Close exact MCP prewarm, refresh, service and transport custody. A callback finishing or requesting MCP invalidation is not proof that MCP work joined. `curated_ownership_clean` does not mean the entire host is clean.
- Adopt/test cooperative extraction separately, and implement recoverable repository/SHA publication and host-death fencing. The frozen C2b extraction proposal is unadopted; its cooperative limits are not a total ZIP-constructor memory or syscall-duration bound.

These repairs are prerequisites for safe extraction, not additional independently installed subsystems. Native auth persistence/load/refresh authority and installed auth/catalog activation still follow them.

## Preserved proposals, not accepted code

MCP upper02 plus retirement03, the 43-file lower workspace proposal and five-file pinned SDK proposal remain frozen, statically reviewed, **unadopted and uncompiled**. They cover session errors/retained handles, exact local process/service/transport custody and observable HTTP worker/SSE completion. Proposed tests have not run. Adoption requires their recorded dependency order, an approved immutable SDK source/pin and consumer integration, then real transport/process tests and a final aggregate/late-admission fence. The SDK patch alone does not change the workspace's HTTP completion classification; ordinary optional remote DELETE failure is separate from local ownership uncertainty.

A reviewed allowlist selects **204 source/preimage/patch/license/documentation files, 7,971,037 bytes**, for inert WIP preservation. It includes the original and corrected slow-store fixtures and original frozen proposal history, excludes private runtime homes/auth data, caches, binaries and generated replay trees, and preserves Codex and pinned upstream license/provenance material. Preservation manifest SHA-256: `42ebfc9ec99ece1a596223ced049695374271c44db5e10b5fc16ba47848760b7`; allowlist SHA-256: `45630ed70152b44237de33af9a50125fdf636a16f8edb65fc196b6511aaad521`. Publication and exact remote readback completed at `d989351c0f016a3f8c8bc2ea3248c1c19e915dd5`. A further45 inert files (923,814 bytes, including metadata) preserve read-only lifecycle observation, callback activity and actual same-process replacement acceptance proposals at `1f808b9fcac8fc2f548b73bc0e486459bc9c59ce`; all48 changed blobs in that checkpoint were read back. These proposals remain unadopted, uncompiled and unrun. Preserving WIP does not adopt it.

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

- [Manager88 exact evidence](verification/2026-10-02/P03_MANAGER_TESTS01.json) and [production-build failure/recovery](verification/2026-10-02/P03_PRODUCTION_BUILD01_AND_EXECUTOR_RECOVERY.md).

- [Current execution ledger](EXECUTION_STATE.md), [canonical roadmap](IMPLEMENTATION_ROADMAP.md), [component inventory](COMPONENT_INVENTORY.md), [upstream-maintenance requirements](UPSTREAM_MAINTENANCE.md).
- [Source922 full-host evidence](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/verification/2026-10-02/P03_SOURCE922_FULL_HOST_EVIDENCE.md), [callback milestone](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/verification/2026-10-02/P03_CURATED_CALLBACK_SCOPE_EVIDENCE.md), [developer SDK](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/component-sdk/README.md).
- [Exact candidate manifest](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/_preserved_wip/2026-10-02/process-final-source-manifest.json), SHA-256 `0a46a9a3f50670a4cc10180012dd3adb09ee2226c75efb8db8c86fbb093d4065`. Its pre-run status is historical; later actual results are separately bound in `/workspace/acceptance/p03-process-final-{native-tests-02,app-server-tests-01,client-tests-02,exec-tests-01,cli-tests-01,manager-tests-01}.{source.json,subreaper.json,log}`.
- [WIP preservation plan](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/_preserved_wip/2026-10-02/PLAN.md) and [exact allowlist](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/d989351c0f016a3f8c8bc2ea3248c1c19e915dd5/_preserved_wip/2026-10-02/ALLOWLIST.json).

Prepared from the current checkout, recorded publication receipts and actual six-suite summary/strict reports; subsequently updated after verified WIP publication. Test execution and source publication are recorded separately.
