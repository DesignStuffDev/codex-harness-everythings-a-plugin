# Codex Harness Compartmentalized — execution state

Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Coverage: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required updates: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
**Full v1 is incomplete. Resume this queue; plans and crate counts are not extraction proof.**

## Current checkpoint

- Repository: `DesignStuffDev/codex-harness-everythings-a-plugin`.
  Original checkout `/workspace/codex-harness-everythings-a-plugin`; preserve
  `/workspace/codex-harness-next-components` too. Never substitute/reset this VM.
- Publication base, last remote observed before this checkpoint: `8db2c4e10feab061c7dec542a2125cdac6b2290f`, tree
  `27c78ca6aeb7ac45dbe078276ed66f5cde42542d`, parent `d04d5a7`.
  That earlier checkpoint published provenance tooling, roadmap/state, full-host runtime
  evidence and screenshots. This checkpoint adds the native policy prerequisite,
  expanded metadata checker and required migration-recovery acceptance. Resolve current
  main through GitHub before the next publication; do not infer its ref from this file.
  Resulting commit/tree receipt: `p03-native-policy-publication.json` in recovery after push.
- Native provider prerequisite: `d04d5a77d4dcaf51759666fd042ab65a3597d900`.
  This is compiled endpoint ownership, not an installed catalog/provider component.
- Official upstream remains `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`.
  Exact-tree import `ae720ae9a98bad29ca2cff998e7d5baaf05cec86`, tree
  `147ac2447134294359c4071b0aeb495922760db7`. Retain LICENSE/NOTICE and lineage.
- Local HEAD/index remain at the original upstream pin; local main is stale.
  Do not reset/rebase or manufacture local commits to match connector publication.
  Original index SHA256: `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.
  Use temporary-index exact trees, supported GitHub connector, fresh remote ref,
  nonforce update and remote verification.

## Last verified behavior

Native installed replacements cover the recorded thread-storage/manual-migration,
inline-attachment and bounded-search subsets. GUI is an installed additive client.
Other native subsystems remain coupled or compiled prerequisites.

- Provider/models-manager: **160 passed, no skips/retries**, including five local-HTTP
  cases. Preserve the original ENOSPC compilation failure (zero tests), lint and
  mechanical-format lineage. [Evidence](verification/2026-10-01/P03_PROVIDER_ENDPOINT_EVIDENCE.md).
- Fresh full CLI build matched all 8,878 published d04 source entries. Frozen binary:
  `/workspace/component-checkpoint-candidate-p03-provider-full-cli-20261001/codex`,
  SHA256 `70669f0b5b81d5ab14e2093c4eb85d7a9275baaa59e0b0aa2134b3dea6ac4750`.
  Reused independent storage passed 13 real commands, migration 10; installed search/GUI
  passed two cold cycles. First SIGINT to manager during an active turn exited 0 in
  0.667s/0.216s; tracked processes were absent.
  [Evidence](verification/2026-10-01/P03_PROVIDER_FULL_HOST_EVIDENCE.md).
  Chromium/Playwright and deterministic inference are not in-app Browser/live-provider proof.
- Earlier search lifecycle: **6095 library passes/4 skipped**, **23 TUI integration
  passes/4 skipped**, installed old/new workers and held-reply cancellation passed.
  Native-constructor cancellation and pending-Open manager SIGINT remain distinct gaps.
  [Evidence](verification/2026-10-01/P02B_PUBLIC_STOP_EVIDENCE.md).
- Published P00M baseline: 13 fixture passes; object check exit2, **0 invalid/1,160 unresolved**.
  Immutable d04 index covers 1,092 paths / 28 maps / 27 edges. Semantic ownership,
  historical anchors, one schema and updater/release closure remain unresolved.
  [Evidence](verification/2026-10-01/P00M_LINEAGE_EVIDENCE.md). No updater or later official
  upstream integration is implemented/proved by metadata checks.

## This checked source checkpoint

The native policy-epoch prerequisite passes **256 login tests, zero skips/retries**,
including seven new cases, and **one focused App Server test, 425 filtered skips**.
Both passing runs used the same 8,880-entry source snapshot as their original failed
compilation attempts. Scoped Clippy passed unchanged; global formatting changed only
four Rust files. Three required argument comments are recorded separately; expressions
and assertions did not change. No tests were repeated solely for formatting.
[Evidence and limitations](verification/2026-10-01/P03_NATIVE_POLICY_EVIDENCE.md),
[original/current lineage](upstream/p03-native-policy-lineage.json).

This adds atomic manager-local policy revisions, snapshots, stamps and checked native
reads. It is **not installed auth extraction or complete asynchronous authorization**.
The earlier d04 full-host/GUI gates do not cover this newer source. Native cached auth,
post-await publication, credential-owner changes, persistence, HTTP and cache/mirror
publication still require their actual fences.

Preserved failures: login compilation ENOSPC (exit101, zero tests); App Server linker
SIGBUS with full disk observed (exit101, zero tests, no precise cause inferred).
The unchanged-source retries passed. Raw command reaps and separate original logs/maps
remain in `/workspace/acceptance/p03-native-policy-*`. The failed partial executable
was archived before retirement (SHA256
`f5a29d7071e0de43855da6d2a1c7122a633f1334c6372aff82b4ca817f4551fa`).
Successful generated test executables were also archived/verified before cache cleanup;
no frozen runtime host was retired.

P00M provider-schema followup passes **22 fixtures**; actual immutable-object check exits
2 with **zero invalid / 1,167 unresolved** over the frozen d04 index. All 28 map shapes
are recognized. Its three source files remained unchanged through global formatting.
[Supplement](verification/2026-10-01/P00M_PROVIDER_SCHEMA_EVIDENCE.md).
Original index/map and 13-fixture evidence remain intact. Neither metadata validation
nor native policy work implements an updater or advances the official upstream pin.

The next reload-policy patch is independently source-reviewed but **uncompiled and
unadopted**. Original stage `/tmp/p03-native-reload-policy-stage/FINAL_MANIFEST.json`,
SHA256 `1308c77f406c1e7d7373625c6f9c075c753ed9ef869ccdd325387bc5383c0412`,
has five paths and ten authored tests. All 28 stage files, including prior unsafe draft
preservation, are in `p03-native-reload-policy-staged-source.tar.gz`, SHA256
`e4fd320e0c8800bfba9e03961689f96257c4e7ae707b79aa7fb2d70bf123b10a`.
It must be explicitly rebased onto the formatted policy base; a newer worker stage
is not automatically authoritative. Native restriction rejection must remain distinct
from true transient/source failures; do not retain disallowed credentials by conflation.

## Ordered next actions

1. Recheck original environment, both worktrees, current GitHub ref, active processes and
   resources. Read the latest publication receipt and exact source tree. Root owns
   checkout/Rust/cache/Git mutations; workers stage patches under `/tmp` only.
2. Finish nonforce publication of this coherent checked checkpoint if its receipt/ref
   is absent. Preserve the original local index. Do not repeat completed gates solely
   for formatting or relabel old binary/runtime proof as testing the newer source.
3. Review the reload patch's explicit formatted-base rebase and source correspondence,
   preserve it, adopt it, and test both real production reload paths: held resolver/ABA,
   native/external policy rejection, legitimate no-auth clearing, real source failure,
   current failure metadata and notification/lock ordering. Run scoped login regression,
   lint and formatting; publish only its verified result. Owner/persistence/dispatch
   guarantees are distinct followups. Follow the
   [native prerequisite audit](component-sdk/design/native-catalog/NATIVE_CATALOG_NEXT_CHECKPOINT.md).
4. Carry checked authority through actual endpoint/HTTP retries and conditional native
   cache/persistence publication. Fresh host/installed-plugin/GUI checks are required
   before new runtime integration claims; preserve unchanged independent package hashes.
5. Implement the production [leaf broker](P03_LEAF_BROKER_DESIGN.md): strict original-byte
   negotiation, bounded ownership/grants, retained decode/serialize/flush/receiver receipts,
   centralized entry guards, cancellation/drain/quarantine. Retain fail-closed catalog
   rejection until coordinated activation; checked DTOs do not enforce a broker.
6. Prove an independently installed diagnostic consumer, then native/custom catalog/cache
   replacements with actual listing/selection, unchanged host, compatibility/removal/
   failure and GUI/headless recovery. Diagnostic infrastructure is not native extraction.
7. Continue every P04–P19/inventory obligation. Maintain P00M as source changes. P18U must
   implement an installable updater plus independent external recovery, integrate a real
   later official revision in isolation, preserve unchanged compatible custom packages,
   refuse incompatibility and recover from failed activation and applicable interrupted
   state migration. No polling or live update is enabled.

## Resume and preservation

Read AGENTS. Source `/workspace/toolchains/component-verification-env.sh`; override
`CARGO_BUILD_JOBS=1`. Use `/tmp/run-p02-check.py` for actual scoped source hashes and
`component-sdk/tests/subreaper_runner.py` for lifecycle gates. Its baseline field is stale
local main, not a clean-commit assertion. Never kill Rust or weaken lifecycle tests.
Do not repeat tests solely for mechanical formatting; `just fmt` also runs Python Ruff.

Resources require explicit checks before each large link. After the last completed
cleanup, 55 receipt-bound orphan temporary library/metadata backings reclaimed
1,043,165,184 bytes of fixed shared memory (zero direct overlay gain). Receipt
`p03-orphaned-relocated-backing-retirement.json` records exact paths and identity checks.
The 16 GiB cgroup still carries substantial fixed shmem; free `/dev/shm` capacity is not
a safe allocation budget. Later bounded relocation needs its own receipt. Prior
cache savings must not be counted again. Some inactive storage copies share inodes:
copy before write/chmod/utime. Preserve all frozen hosts and the running old GUI.

Original cloud status was connected/running, current/enforced revision 47, restricted
package_managers with no extra hostname/VPN/preview capability. Revalidate current status.
In-app Browser/Context7 are unavailable; optional remote viewer is not a core-work blocker.
Keep private GUI URLs/logs private; publish reviewed summaries/screenshots only.

Recovery: `/workspace/recovery-backups/20260930T165936Z`. Full original source/Git/SDK/evidence
archive `codex-recovered-workspace.tar.zst` SHA256
`3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
Latest 15-file archive `p00m-runtime-verified-checkpoint.tar.gz` SHA256
`0226f72c1af8dfb313e9a13c07b3d71a484c3e082c312d5e2066cc2bf3672419`.
Archives remain cloud-local; GitHub protects included source, not private state/binaries.
Preserve both worktrees and existing WIP branches. Detailed history remains in the
[published prior state](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/8db2c4e10feab061c7dec542a2125cdac6b2290f/EXECUTION_STATE.md),
dated verification files and `execution-state-before-policy-checkpoint.md` in recovery.
