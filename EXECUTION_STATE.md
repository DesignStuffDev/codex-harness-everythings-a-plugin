# Execution state — read first on resume

Updated 2026-09-30. Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Inventory: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).

## Current checkpoint

- Project: Codex Harness Compartmentalized; full v1 is **not complete**.
- Phase: P00 roadmap/source audit complete; P01 implementation is next. No new
  native extraction is claimed by this documentation checkpoint.
- Last accepted implementation: `4e38e02993df698cefa99d7cc325890d837262de` (`main`).
- Unverified preserved work: `0b38d5974159ab2a8776200025dbb602e89b8f8f`
  (`wip/recovered-next-components-20260930`); do not merge wholesale.
- Upstream pin: `d42056091aded7feb1d88ac7e83972108b2aa478`.
- Repository: `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
- Roadmap commit: this document's containing P00 commit; resolve with
  `git log -1 --format=%H main -- IMPLEMENTATION_ROADMAP.md` after publication.
  Implementation proof still refers to the accepted implementation above.

## Ordered next actions

1. Verify this P00 documentation commit is present on remote main before new code.
   Inventory covers all 164 explicit workspace members, 175 primary Rust packages
   and nine non-Rust manifests. Initial lineage maps 617 baseline changed paths and
   seven source-validated boundaries; updater implementation remains planned.
2. P01: agree manual-migration capability/version and cancellation contract. Audit
   WIP `migration_run.rs` and native implementation, then port only that slice.
   Complete native worker dispatch and real CLI selected-store routing. Explicitly
   reject unsupported selected backends; never migrate a different built-in store.
3. Independently build/install the updated native store package, exercise actual
   migration/resume/cancel/shutdown, and run affected storage/CLI/GUI regressions.
   Publish only when this coherent milestone is verified; update evidence/queue.
4. P02a: extract native file search with joined matcher/walker shutdown, real
   app-server/TUI selection and separately installed native/custom packages.
   Defer P02b mutation services until P03/P04/P10 authority prerequisites pass.
5. P03: integrate session startup cleanup ownership, transport v2 and broker as
   separate reviewable gates. Missing WIP test files/linkage must be completed.
6. Continue the remaining ordered roadmap phases; choose independent leaves only
   with recorded contract/dependency justification. Viewer networking is optional.

## Verified evidence and limits

- Corrected core library: **2,694/2,694 passed**, zero failures/retries, one helper
  excluded; run `30e4ea84-f664-40f8-ba22-fac95cb27b80`. Actual source manifest SHA256
  `38a866549659cca98219fa6c3e28d56de2ebb3104add642a1f7c043900188899`.
- Native thread-store/inline-attachment independent installation and real CLI
  streaming/tools/persistence/resume passed. GUI streaming, approval, Stop, cold
  recovery and actual manager Ctrl+C passed. Inference was deterministic fixture
  code with the real engine. See [VALIDATION.md](VALIDATION.md) for exact scope.
- Accepted component manager SHA256:
  `acffaefb470b796b47f36fd9a9d5d4c23d2b5d3622316256b6c8c770ccc8a8f3`.
- Accepted Codex CLI SHA256:
  `b7a39b3534c51a6115135083ee9ffcf7c1eced81e51eef246e80c7b706a64371`.
- Full workspace suite remains disk-blocked; cross-platform and live-provider
  gates remain open. Staged/WIP source is not validated by old checkpoint evidence.
- New source audit found catalog/broker/replay not active in main, model-v2 missing
  module/test/dependency links, migration worker dispatch/CLI route incomplete,
  native auth/OAuth and all auxiliary StateRuntime domains still coupled.

## Workspace and preservation

Use the original cloud source at `/workspace/codex-harness-everythings-a-plugin`.
Saved isolated WIP is `/workspace/codex-harness-next-components`. Both worktrees
still have upstream HEAD and recovered working changes; **ordinary git diff/status
against HEAD includes the entire recovered baseline**, and untracked baseline files
can appear as deletions in a comparison to main. Do not infer missing files from
that diff, reset/clean either worktree, or bulk-stage everything.

Published commits were constructed with temporary indexes while preserving the
original worktree HEAD/index. Continue explicit-path temporary-index checkpoints
based on current `main`, or use a verified new worktree if resources permit. Compare
actual file bytes to published blobs and record tested-source manifests. Preserve
the source path/shared build target to avoid unnecessary whole-workspace rebuilds.

Full cloud-local backup: `/workspace/recovery-backups/20260930T165936Z/` contains
both complete source trees, SDK/GUI/evidence/docs, Git state/patches, accepted
packages and a verified Git bundle. Archive SHA256:
`3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
This is not an external backup; the two GitHub refs protect their published source.
Preserve ignored/private runtime state; never publish credentials or cache artifacts.

## Runtime/resources and resume checklist

1. Read AGENTS and cloud-runtime skill; query task-bound environment readiness and
   verify checkout/origin/remote refs. Current original environment ID is
   `ccarenv_b64_Y2NhcmVudl8wYzAyOTNkMzVhZTg4MTkxYjc4YjQyZmVhNWYwMDllYQ`;
   re-observe it rather than trusting this historical value.
2. Inspect active commands before starting work. No Rust build was active at P00
   audit. Never kill Rust commands; one build owner serializes tests/links.
3. Source `/workspace/toolchains/component-verification-env.sh`; use
   `CARGO_BUILD_JOBS=1`, existing debug0/incremental0 profiles and shared target.
   Use `just test`, scoped `just fix`, `just fmt`, schema/Bazel updates as applicable.
4. Overlay is 32 GiB and almost full before reclaim; 22 GiB is generated Cargo
   target data. Approved P00 reclaim is only 44 idle generated CLI/exec-server test
   executables (~2.87 GB), retaining libraries, accepted binaries and all sources.
   That eviction completed; observed free space was 3,572,199,424 bytes. Details:
   `/workspace/recovery-backups/20260930T165936Z/resume-extraction-cache-eviction.json`.
   Recheck current free space, reserve at least 1 GiB for linking,
   and run focused batches. Do not assume /tmp tmpfs is free disk or discard its
   preserved metadata-symlink targets. No second full build cache.
5. Recheck accepted binary hashes; use a new isolated home for new acceptance.
   Keep `/workspace/verified-component-checkpoint-v2-20260930` immutable.
6. Run actual GUI regressions with existing SDK acceptance tooling. In-app Browser
   is unavailable in this cloud task; record that limitation and use actual
   Chromium/Playwright checks, without claiming the requested manual Browser ran.
7. Record implementation/test/publishing status separately. Preserve original
   failures and WIP. Update this file and inventory before each milestone push.

## Optional viewer, not an extraction blocker

Isolated viewer setup is `/workspace/remote-viewer-setup`; no harness dependency.
Its saved reports include recovered persisted history and fresh noVNC canvas input
with unchanged host hashes. Services may stop between runs; inspect before using.
Published environment settings did not reach this task's observed restricted HTTP
policy; bridge pairing/local Browser access remains unverified. Do not spend core
implementation time waiting for it, bypass network policy, or replace the original
VM. Native Codex remote-control extraction is a separate P17 requirement.

## Decisions and open gates

- Native process packages with versioned owned contracts; restart activation for v1.
- Manual migration first: real coupled native CLI behavior, reuses accepted storage,
  no broker prerequisite. File search follows as a distinct native subsystem.
- Shared broker/session lifecycle before catalog and replay activation; no blanket
  import of unverified WIP and no claim that staged source is working replacement.
- Keep authority/enforcement/supervision in a documented small kernel; move actual
  engine scheduling/model/context/tool policy into separately installed packages.
- Routine implementation/testing/commits/pushes are authorized. No force pushes,
  unrelated account changes, purchases or environment replacement.
- Upstream update integration is now required (supersedes earlier deferral):
  [maintenance workflow](UPSTREAM_MAINTENANCE.md), [lineage](upstream/lineage.json),
  P18U. Establish mapping now; later independently package updater and prove one
  real upstream integration plus breaking-update rollback. No periodic schedule
  or live-installation update has been requested or started.
- Remaining limits: finite disk/RAM/run lifetime, missing in-app Browser and final
  cross-platform/live-provider evidence. None justifies calling full v1 complete.
