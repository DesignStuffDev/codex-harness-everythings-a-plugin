# Production build01 failure and executor recovery — 2026-10-02

This note records terminal observations captured before execution transport failed and subsequent
read-only status/connection results. It is not a new filesystem audit or runtime pass. It was
published through the GitHub connector while shell access was unavailable; synchronize it into
the original working tree only after preserving and comparing local changes.

## Exact failed build

- Original checkout: `/workspace/codex-harness-everythings-a-plugin`; preserve sibling
  `/workspace/codex-harness-next-components` and all original runtime/worktree state.
- Prefix: `/workspace/acceptance/p03-process-final-full-cli-build-01`.
- Started2026-10-02T03:30:25Z; finished2026-10-02T03:36:30Z; terminal exit101.
- Command: `cargo build --locked -p codex-cli --bin codex -p codex-component-host --bin codex-component`.
  One Rust job, existing source-bound wrapper and unique evidence prefix.
- Normal production `codex-tui` library compilation failed:
  `couldn't create a temp dir: No space left on device (os error28)` at
  `codex-rs/target/debug/deps/rustcxgiRNV`.
- Final wrapper JSON parsed successfully and recorded `scoped_source_unchanged: true`.
  Overlay available space was0. No newly rebuilt-host storage, migration, GUI or shutdown tests ran.
- This differs from `p03-process-final-tui-tests-01`, whose test-target compiler was killed by
  SIGKILL after2,156.43s. No conclusive OOM attribution exists for that earlier failure.
- Root did not signal Rust. The production command was already terminal before recovery work.
- Resource baseline: R/`p03-process-final-full-build01-resource-baseline.json`, where
  R=`/workspace/recovery-backups/20260930T165936Z`. About1.215GB overlay margin proved insufficient;
  cgroup memory limit16GiB. Preserve newly compiled production dependencies before any retry.

The adopted candidate is externally preserved at1f808b9fcac8fc2f548b73bc0e486459bc9c59ce, based
on the56-path manifest SHA256 `0a46a9a3f50670a4cc10180012dd3adb09ee2226c75efb8db8c86fbb093d4065`.
Its previously verified8,920-file source map is
`31f82854bdfeffe7ec99a05ef46a9a2eaba2cacd1b2587a568e8f2be81836a73`.
Do not invent a fresh post-reconnection map or transfer older runtime proof to this candidate.

## Temporary artifact relocation: verify before restoring

To obtain disk headroom after ENOSPC, root verified and moved only the inactive generated-test-cache
archive from R/`p03-test-curated-caches-preserved-01.tar.zst` to
`/tmp/p03-test-curated-caches-preserved-01.tar.zst`.

- Size239,382,700 bytes; SHA256
  `abb99adaa480fa2dbd3feb2ddb0b7833803f5ea555b121b84b30697c3bb198d8`.
- Receipt: `/tmp/p03-fixture-archive-temporary-relocation.json`, status
  `temporarily_relocated_restore_pending`; records original mode0600 and nanosecond times.
- Copy was fsynced, mode/times preserved and both contents hashed before exact original removal.
  Last observed overlay availability after relocation was153,407,488 bytes.
- The archive contains97 inactive generated test-fixture clone caches, not unique project source.
  Their original97 nested cache directories had previously been archived, verified and retired;
  parent homes/configuration were retained. The archive is now in RAM-backed temporary storage.
- Reconnection/restart may affect its survival. Check its existence/hash and receipt first. After
  obtaining disk space, copy/fsync/verify it back to R with original mode/times before deleting
  the temporary duplicate. Restored inode/ctime differ; update metadata-based inventories honestly.
- If temporary storage was lost, report this exact generated-fixture preservation gap; do not claim
  either that all source was lost or that every backup remains intact.

The primary full two-worktree/Git/SDK/evidence archive was untouched:
R/`codex-recovered-workspace.tar.zst`,218,540,149 bytes, SHA256
`3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
Its post-interruption accessibility is unverified. VM-local archives are recovery checkpoints,
not external backups. GitHub durability covers only published included source and documentation.

A temporary pre-build source snapshot was also saved at
`/tmp/p03-full-build01-running-source-preserved.json`, SHA256
`ea4930db706554e883950a0490af8f786ffadfda02d57cf115cc964144ea4b18`.
The actual terminal wrapper did finish its final receipt; preserve both if present.

## Prepared, unexecuted headroom work

R/`p03-production-curated-clones-preservation-02/` contains a staged first24 inactive
historical clone-cache preservation bundle. PLAN SHA256
`2ea613be11b09d8735ff53d426c821b7cf223d8278f0ba0e57bb017cc841efd9`.
The candidate selection is840 members/576 unique regular files,589,111,296 allocated bytes;
content bytes are24,466,019. It excludes source, SDK, frozen GUI and remote-viewer state.

The worker statically checked its scripts, but root's attempted review command failed when the
executor disconnected. **No archive, deletion or promotion from this bundle has run.** Review
PLAN, guard and action sources, verify hashes and current references/ownership before action.
Scripts stage an archive in temporary storage, verify it, retire exact members, then promote
and verify the archive into R. Do not execute blindly, reuse a completed older selection, or
assume this first589MB wave alone provides enough production-build margin.

## Connection result and exact resume sequence

The task-bound environment retained its identity. Status moved from offline/running through
offline/starting to running/connected with current observations, observed revision1462. However,
two subsequent read-only `pwd` attempts failed before process creation:

`Noise harness handshake failed before connection became ready`.

The earlier transport failure said exec-server recovery timed out after25s. No explicit reconnect
or resume command is exposed in the available tool catalog. Status metadata alone does not prove
execution readiness. No environment configuration change, reset, replacement VM, new checkout,
local execution or repeated remote-viewer network probe was performed.

1. Restore execution access to this same cloud chat/VM through the supported environment controls;
   if that cannot be done without replacement, stop and report the exact access limitation.
2. Verify original repository identity, both worktrees, changes and real index SHA256
   `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59` without resetting it.
   Compare/reconcile this connector-published ledger with local files; preserve divergences.
3. Check temporary archive/receipt survival, full-source backups, frozen GUI/runtime processes,
   production failure reports and actual source maps. Do not assume a previous PID is still live.
4. Establish fresh exact resource/reference guards; obtain enough overlay margin and restore the
   temporarily relocated archive. Keep a single Rust/build/cache writer. Do not kill Rust.
5. Run a uniquely named production retry02, then the planned installed storage/migration/GUI,
   actual Launch Ctrl+C, held Git/HTTP, slow-store and replacement gates with unchanged strict
   descendant checks. TUI, changed App Server integration and lint remain outstanding.
6. Only after closing the current lifecycle prerequisite resume the dependency-ordered extraction
   queue. Unadopted proposals are not implementation or test proof. Main promotion remains gated.

Remote viewer connectivity remains a separate optional workstream. In-app Browser and Context7
are unavailable; earlier GUI runtime evidence used Chromium/Playwright with deterministic inference.
