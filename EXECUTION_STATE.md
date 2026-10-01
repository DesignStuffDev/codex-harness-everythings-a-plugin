# Execution state — read first on resume

Updated 2026-10-01 UTC. Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Inventory: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).

## Current checkpoint

- Project: Codex Harness Compartmentalized; full v1 is **not complete**.
- P00 roadmap/source audit is complete. P01 manual-migration implementation and
  scoped runtime gates are complete: source commit
  `5d2f2a026ae6ce0c42cf56bb2f3b01971e4bf0bb`, tree
  `1aa7638122282600aa493661a33594285f945d2d`.
- This documentation checkpoint accompanies that source on `main`. Revalidate
  remote refs before resuming or publishing; do not infer them from local HEAD.
- Previous accepted implementation: `4e38e02993df698cefa99d7cc325890d837262de`;
  roadmap checkpoint: `83364aa63526d75840d4149f4598ba1aeed02a06`.
- Separately preserved unverified work: `0b38d5974159ab2a8776200025dbb602e89b8f8f`
  (`wip/recovered-next-components-20260930`). Do not merge it wholesale.
- Upstream pin: `d42056091aded7feb1d88ac7e83972108b2aa478`.
- Repository: `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
- P02a native search ownership work has started separately in the working tree.
  Its new edits and vendored dependency changes are **not verified or included in
  the P01 source commit**. Inspect and preserve them before any next step.

## Ordered next actions

1. Verify the original environment, checkout, remote refs, source/index state and
   active commands. Read the P01 evidence and compare actual blobs to the accepted
   source commit; original HEAD/index deliberately remain at the upstream import.
2. Finish P02a1 native file-search ownership: joined matcher/walker and actual Rayon
   threads; failure-aware synchronous run. Review the small pinned Nucleo change,
   retain MPL-2.0 notices/provenance, verify separate source export and inventory
   any added manifests. Never confuse cancellation requested with joined cleanup.
3. Repair TUI reporter ownership/cycles and stale generations; App Server connection
   ownership, ordered notifications and joined close. Agree these semantics before
   enabling the proposed file_search1 process adapter. Detailed gates are in
   [FILE_SEARCH_COMPONENT_PLAN.md](component-sdk/FILE_SEARCH_COMPONENT_PLAN.md).
4. Independently package native/custom search workers, install without rebuilding
   the frozen host, exercise real clients, then regress existing storage and GUI.
   P02b mutation/watch/Git services wait for P03/P04/P10 authority prerequisites.
5. P03 startup ownership, model transport2 and broker are separate reviewable gates.
   Complete missing WIP test/module links; do not activate everything as one import.
6. Continue the remaining ordered phases. P18U upstream maintenance is required;
   establish lineage at every extraction now, implement the updater after stable
   contracts. Viewer networking is an optional separate workstream.

## P01 verified behavior and exact scope

[Evidence and original failures](verification/2026-10-01/P01_TEST_EVIDENCE.md)
record **625 distinct passing Rust cases across runs**, not one aggregate run:
279 native thread-store, 23 component, 21 native worker, 295 CLI unit, five CLI
migration and two real OTLP metrics cases. The final component/worker rerun passed
44/44 with no retries and the unchanged subreaper. One subprocess helper is ignored.
Nine packaging-version tests passed. Scoped final Clippy had no warnings; formatting
and Bazel lock regeneration passed. No new production core/common/protocol changes.

- Storage package **0.2.0**, storage contract **2**, optional manual-migration **1**:
  exported outside the checkout, 81 local crates/917 resolved packages, no new
  dependency identities, core/CLI/App Server/TUI excluded. Worker built separately,
  export removed before install, host fingerprints unchanged throughout acceptance.
- Actual legacy conversation: native/selected dry-run parity and unchanged storage
  bytes, selected Apply, repeated Apply, cold CLI model-history and App Server UI
  history, cancellation and strict process disappearance passed.
- Actual older native package 0.1.0/storage2 remains usable for ordinary turns,
  tools, persistence and cold resume. Migration dry-run returns Unsupported with
  no fallback, unchanged storage and all observed workers reaped.
- Real Chromium/Playwright GUI: streaming, native command approval, Stop, page reload,
  cold engine/browser restart and continuation passed. Two manager-only Launch
  SIGINTs during active blocked turns exited zero; all tracked manager/gateway/
  App Server/storage/model PIDs absent, with subreaper drains. See safe screenshots
  in the P01 evidence. This was **not** the unavailable in-app Browser.
- Initial GUI approval timeout is preserved. CLI exec resume persisted Never;
  fixture preparation now explicitly restores on-request through public
  thread/resume. The product honored Never; no policy bypass or automatic approval
  was added. Failed Rust fixture attempts are also preserved separately.
- State/rollout regression is focused native integration, including journals,
  compression/archive preservation, SQLite contention and recovery. Complete
  standalone state/rollout/workspace suites were not rerun for unchanged crates.
- Generic process large-history transport and migration report codec size cases
  passed. A dedicated >4 MiB migration-report IPC case was not run.

Frozen P01 binaries (do not overwrite):
`/workspace/verified-component-checkpoint-p01-20261001/`.
Manager SHA256 `acffaefb470b796b47f36fd9a9d5d4c23d2b5d3622316256b6c8c770ccc8a8f3`;
CLI SHA256 `467ee8360d45701bed728dead65568d6178b5b2d8c2c5b6d149ebcfb11a5ad01`.
The original accepted host directory remains immutable too.

## Contract and remaining coverage

The selected backend is resolved before native construction. Migration capabilities
are optional; absent/unknown versions disable only migration. No native fallback
is permitted after selection. Selected Apply has one worker-owned StateRuntime;
DryRun opens no database, and both startup maintenance flags are off in this path.
Native cancellation finishes accepted path publication. Cancel acknowledges the
request; close joins cleanup. Dropped waiters do not abort accepted mutation futures.
There are 32 retained runs, bounded pending/tombstone slots and 128 retired-ID
acknowledgements; this is not durable replay protection. Clients use unique IDs.
First interrupt cooperates; repeated signal or 210-second budget reports durability
uncertainty. Native/selected telemetry records one run without double counting.

This extends actual native thread storage. Inline attachment is a separate native
subset; the GUI is additive. Model/auth remain adapters; loop, live context,
compaction, native tools/policy and auxiliary databases remain coupled. The previous
2,694/2,694 core-library result belongs to its recorded older source, not new P02
work. Model-v1's reproduced 4 MiB limit and staged model2/broker/replay remain open.
Cross-platform, live-provider and full workspace evidence are not established.

## Workspace and preservation

Original primary: `/workspace/codex-harness-everythings-a-plugin`.
Isolated recovered WIP: `/workspace/codex-harness-next-components`.
Both retain upstream HEAD and recovered changes; **ordinary diff/status against
HEAD includes the recovered baseline**. Untracked baseline files may appear deleted
in a comparison against main. Never reset/clean/bulk-stage either checkout.

Publish explicit paths with a temporary index based on a verified commit. Preserve
original HEAD/index and the shared build paths. For each checkpoint record the
actual tested source, independent package, binary hashes and limitations. Never use
`/tmp/run-component-regression.py` unchanged: its source label is obsolete.

Full cloud-local backup `/workspace/recovery-backups/20260930T165936Z/` retains both
source worktrees, SDK/GUI/docs/evidence/Git state and accepted artifacts. Archive
SHA256 `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
P01 incremental source snapshots and original logs remain there. Snapshot
`p01-source-20261001T011110Z.tar.gz` SHA256
`5b4aa470b8e883eacf421cc99a8e4730b2cfd0431139051ee0f3df13e854787b`
was member-hash verified before the final documentation additions.
Cloud-local archives are not externally durable backups. GitHub protects only the
source included in its published refs; preserve newer work before further edits.
Never publish credentials, private bearer URLs, caches, binaries or runtime homes.

## Runtime and resume rules

Read AGENTS and cloud-runtime skill, then query task-bound status. Observed original
instance was `ccarenv_b64_Y2NhcmVudl8wYzAyOTNkMzVhZTg4MTkxYjc4YjQyZmVhNWYwMDllYQ`,
revision38, connected/running; re-observe rather than trusting this record.
Source `/workspace/toolchains/component-verification-env.sh`, use jobs1 and existing
debug0/incremental0 settings. One owner serializes Rust builds; never kill Rust.
Use just test, scoped just fix and just fmt; regenerate Bazel/schema when applicable.

Overlay is 32 GiB with roughly 1.2 GB free after P01. Check actual resources before
linking. Eviction reports retain exact hashes of removed stale generated executables;
all sources, libraries, metadata symlinks and accepted binaries were preserved.
Do not treat /tmp tmpfs as disk spill or remove its live metadata-symlink targets.
Use isolated runtime homes and unchanged subreaper/lifecycle assertions.

Optional viewer: `/workspace/remote-viewer-setup`, outside harness. Its authenticated
local services do not prove cloud-to-local reachability. Published editor settings
still did not reach this task's observed allowed_hosts list; bridge pairing remains
unverified. Do not bypass policy, reset the VM or let this block component work.

Routine implementation, tests and non-force checkpoint pushes are authorized.
Upstream updates are required by [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md):
exact lineage, isolated candidate, compatibility/security/runtime/UI gates, coordinated
versions/migrations and rollback/bootstrap recovery. No polling or live installation
update has been started. Continue feasible phases; finite resources/run lifetime and
missing Browser do not justify calling the whole platform complete.
