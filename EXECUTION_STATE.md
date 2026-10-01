# Execution state — read first on resume

Updated 2026-10-01 UTC. Follow [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md) and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md). Full v1 is **not complete**.

## Last verified checkpoints

- P02 StageA source: `25b2c3150879761026086a62e480045fc38d32ff`, tree
  `5ba385b6e7730119b6d5cdca67fba2dc4f5c395d`; its companion documentation commit
  publishes this state and evidence. Revalidate the current remote ref.
- P01 source: `5d2f2a026ae6ce0c42cf56bb2f3b01971e4bf0bb`.
  Native thread persistence and optional manual migration are independently selected;
  inline attachment storage is a separately replaceable native subset.
- StageA fixes native search worker/publication lifetime in App Server/TUI and adds
  file references to the independently packaged desktop GUI. **Search is still
  coupled.** Infrastructure, lifecycle repairs and additive UI do not count as its
  extraction. Model/auth remain adapters; loop, context, compaction, native tools,
  policy and auxiliary databases remain coupled.
- Unregistered StageB API/path-codec drafts: external WIP
  `17c366bb94f77b3d3895acd6f061a5cb308bd384`, branch
  `wip/p02-search-lifecycle-20261001`. They are excluded from accepted StageA.
- Older isolated WIP: `0b38d5974159ab2a8776200025dbb602e89b8f8f`, branch
  `wip/recovered-next-components-20260930`. Never merge WIP wholesale.
- Official upstream pin: `d42056091aded7feb1d88ac7e83972108b2aa478`; Apache LICENSE
  and NOTICE retained. Nucleo's separate pinned MPL-2.0 source/notices are retained.
  See [exact StageA lineage](upstream/p02-search-lineage.json).

## Verified behavior and limits

[P02 evidence](verification/2026-10-01/P02_SEARCH_PREREQUISITE_EVIDENCE.md) and its
machine-readable reports retain actual per-run source/log hashes and failures:

- Native search 35/35, vendored matcher/Nucleo 35/35; App Server 395 library cases
  plus 13 public RPC cases across separate 12+1 runs; focused TUI reconnect 11/11.
- Full TUI: 5,604 passed, four cursor-color failures, four ignored. The same
  executable's six unchanged cursor tests passed with NO_COLOR unset. Together
  these establish 5,608 distinct passing executed cases across environments,
  **not one all-green run**. Original failures/snapshots remain preserved.
- SDK packaging 19/19, desktop controller/gateway 7 Node + 14 Python cases.
  Scoped fix, format, final Clippy and CLI build passed; eight vendor style
  warnings remain. Build scope recorded 2,979 unchanged source fingerprints.
- The new engine ran the existing independently built storage 0.2.0 package
  unchanged: real turns, tools, streaming, persistence, cold resume, restoration,
  manual migration dry-run/apply/idempotence and strict child cleanup passed.
  This proves compatibility, **not a new independent storage build**.
- New-engine GUI: 49+38 Chromium/Playwright commands, zero page errors, real file
  search/insertion, approval/native tool completion, streaming, Stop, reload and
  cold recovery. Manager Launch SIGINT during active turns exited zero in about
  0.315/0.316s; all observed processes absent and subreaper drains passed.
- In-app Browser is unavailable. This is real Chromium fallback evidence with a
  deterministic provider, not live-provider, Windows, remote-viewer or full-suite
  proof. The older P01-engine GUI gate remains separately identified.

Frozen StageA runtime: `/workspace/component-checkpoint-candidate-p02a-20261001/`.
CLI SHA256 `181463925d599f0eed023106820cad8fbb228d81e972eefb256a2076b88813c0`;
manager SHA256 `acffaefb470b796b47f36fd9a9d5d4c23d2b5d3622316256b6c8c770ccc8a8f3`.
Original candidate.json records creation-time pending status; later evidence records
passed acceptance. Do not overwrite it or older frozen P01/v2 artifacts. Stripping
preserved all 30 allocated ELF sections; runtime gates used this immutable copy.

## Ordered next actions

1. Re-observe original task environment, repository, actual working files, refs,
   source/index hashes, processes and disk/memory. Read AGENTS and runtime skill.
2. Register neutral `file-search-api` and `component-path-codec` drafts, add missing
   Bazel target, apply reviewed state-codec path re-exports. Run API/path/old-codec
   compatibility tests, scoped lint/format and lock updates. No installed-search
   claim from this supporting-library step.
3. Implement bounded native backend, coalesced query admission, honest final
   snapshot/Idle ordering, entry/index/worker budgets and resource errors. See
   [native plan](component-sdk/FILE_SEARCH_NATIVE_BACKEND_PLAN.md).
4. Add file_search1 wire service/process adapter and native/custom workers.
   Extend retained startup cleanup and opt-in logical transport limits before
   activation. Install no fallback after selection.
5. Compose one provider per embedded runtime with independent App Server
   connection/local TUI scopes; independent embeddings remain independent. Repair
   high-level embedded-client shutdown timeout/abort reporting before sharing the
   provider. See [composition plan](component-sdk/FILE_SEARCH_COMPOSITION_PLAN.md).
6. Independently build/install workers outside source; exercise GUI/TUI/one-shot
   clients, replacement, removal, incompatible versions, cancellation, resource
   exhaustion and recovery with unchanged host. Regress storage and GUI each cycle.
   The private rollout lookup remains a fourth consumer pending P03 broker.
7. Continue P03 lifecycle/transport/broker, then the roadmap's ordered phases.
   P02b filesystem/watch/Git mutations wait for authority/config/policy contracts.
   P18U upstream update integration is **required**, sequenced after stable
   contracts. Record lineage now; demonstrate a real later revision, unchanged
   compatible custom packages, breaking rejection and external-bootstrap rollback
   before release. No updater, polling schedule or live update is active.

## Preserve and resume safely

Original primary `/workspace/codex-harness-everythings-a-plugin`; isolated recovered
WIP `/workspace/codex-harness-next-components`. Origin remains
`https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
Original `work` HEAD/index deliberately stay at upstream. Baseline files are often
untracked relative to that index: ordinary diff against main can falsely show them
as deleted. Compare actual file blobs. Never reset, clean or bulk-stage either tree.
Use exact path whitelists and a temporary index based on the accepted ref; verify
remote refs immediately before non-force push and verify GitHub afterward.
Original index SHA256: `2e0abf9135fedee93d85ffedb3045397fb85d35017f4191dc5e7a133af0b6759`.

Full both-worktree/SDK/docs/evidence/Git backup is in
`/workspace/recovery-backups/20260930T165936Z/codex-recovered-workspace.tar.zst`,
SHA256 `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
Later member-verified incremental `p01-source-20261001T040040Z.tar.gz` (historical
prefix) covers 124 changed paths versus main4288; SHA256
`9e581f18c9abc5e3719b6f45c11c2fe7b23e0b9284faf408049462edd6691a05`.
It precedes this final state/lineage edit. Keep original failures and later snapshots.
Cloud-local archives are recovery checkpoints, not proven outside backups. GitHub
protects included published source only. Exclude binaries, caches, runtime homes,
credentials and private bearer URLs from publication.

Environment last observed connected/running, revision45, identity
`ccarenv_b64_Y2NhcmVudl8wYzAyOTNkMzVhZTg4MTkxYjc4YjQyZmVhNWYwMDllYQ`.
Source `/workspace/toolchains/component-verification-env.sh`, jobs1, existing
Rust1.95/debug0/incremental0. One owner serializes Rust; never kill Rust. Use
`just test`, scoped `just fix`, `just fmt`, and required Bazel/schema regeneration.
Use `/tmp/run-p02-check.py` for actual source manifests. Never reuse unchanged
`/tmp/run-component-regression.py`: its source label is obsolete.

Resource limit: 32 GiB overlay, last observed only379 MiB free, /tmp tmpfs1.2 GiB
free. Recheck before linking; do not use tmpfs without memory accounting. Audited
cache maps/restoration live in recovery reports `p02-generated-cache-tmpfs-relocation.json`
and `p02-pre-cli-cache-tmpfs-relocation.json`. Original-path cache symlinks need
these backing files. Cargo's dangerous hardlink to frozen-v2 was safely detached;
**do not restore it**. Completed App Server executable is preserved in the
checksum-verified `p02-completed-public-app-server.tar.zst`; generated new CLI
cache was removed after independent immutable candidate verification. Do not remove
source, evidence, runtime state, frozen binaries or active mappings for space.

Optional viewer lives at `/workspace/remote-viewer-setup`, outside harness.
Current enforced restricted policy still has no additional hostname grant; local
services do not establish user-accessible reachability. Never bypass policy or
reset/replace this VM. This workstream does not block component development.
Routine implementation/testing and non-force checkpoint pushes are authorized.
Continue feasible phases; finite resources or unavailable Browser are limitations,
not grounds to claim the platform finished.
