# Codex Harness Compartmentalized — execution state

Read [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md) and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
**Full v1 is incomplete. Plans, crates and native prerequisites are not extraction proof.**

## Repository and publication

- Original checkout: `/workspace/codex-harness-everythings-a-plugin`.
  Preserve `/workspace/codex-harness-next-components` and both worktrees' changes.
- Origin: `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
- Last completed remote verification before this supplement: `63dded3a250527fae51fa61d1e24b70a4029044b`,
  tree `bd5fe63ddb68ed216c2e5a60f81c0607dd337e8c`; all 11 changed blobs and main verified.
  Recovery receipt `p03-source-owner-runtime-publication.json`, SHA256
  `bfa3f69eea18097c5938f5190cafdcf7c31a74179b43844b4e493e9f615cea91`.
- Current native source commit: `65511842d7051b2a1f5cc52917f3ebb5c03be4f3`,
  tree `ed564f68f3297cd65b65321d1f41f79b4f9c4031`, parent `63dded3a`.
  Created source receipt: `p03-install-order-created-source-commit.json`.
  Verify the subsequent `p03-install-order-publication.json` receipt and remote main before resuming;
  this document cannot contain its own eventual publication commit.
- Official upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`;
  exact-tree import `ae720ae9a98bad29ca2cff998e7d5baaf05cec86`. Retain LICENSE/NOTICE.
- Local HEAD/index remain pinned upstream; local main is stale. Use reviewed temporary indices
  and connector publication. Do not reset/rebase or rewrite the real index.
  Its SHA256 is `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.

## Implemented versus proved

Installed replacement proof covers recorded thread storage/manual migration and bounded native search
across CLI/App Server/TUI/GUI. Native inline attachments have a manager-level subset only. The
separately installed GUI is an additive client, not extracted engine orchestration. Authentication
changes below are compiled native prerequisites; auth/catalog installation and broker activation
remain incomplete. Consult the inventory for every coupled subsystem and kernel exception.

[Installer evidence](verification/2026-10-01/P03_INSTALL_ORDER_EVIDENCE.md) binds source `65511842`,
all 8,889 final scoped entries and the [lineage map](upstream/p03-native-install-order-lineage.json).
Latest admitted installation supersedes older work; clear, policy and cache changes reject stale
publication. Equal credentials retain notification compatibility while source identity advances.
Tests-only baseline: 9 executed, 3 passed, 6 assertion failures, 283 filtered, no retries. Full scoped
green: **400 passed** (292 login, 108 model-provider), no skips/retries; source unchanged during tests.
Scoped lint passed without changes/warnings. Global formatting changed only layout in two files;
its separate transition was reviewed, with no tests repeated solely for formatting. A green-run
adopted PID returned SIGKILL; executable and cause are unknown, not an all-child-exit-zero claim.
No fresh full CLI/GUI was built for this source. Shared persistence fencing and source-aware
load/refresh publication remain open.

[Prior full-host evidence](verification/2026-10-01/P03_SOURCE_OWNER_FULL_HOST_EVIDENCE.md), source
`99e6802f`, is **mixed/failed**: two fresh installed-storage runs each passed 13 behavior checks but
strict lifecycle exited 125 because descendants remained. Migration's 10 commands passed. Two GUI
cold cycles passed with actual manager active-turn Ctrl+C exit 0 in 0.265/0.267 seconds, tracked
process identities absent, no forced cleanup and unchanged packages. One migration adopted PID
returned SIGPIPE, attribution unknown. Chromium/Playwright used deterministic inference; this is
not live-provider, in-app Browser or attachment roundtrip proof. Last older fully passing runtime
checkpoint is `b3530790` (source `181400`, CLI `876c…`), not evidence for newer source.

The [curated-sync diagnosis](verification/2026-10-01/P03_CURATED_SYNC_LIFECYCLE_DIAGNOSIS.md)
records live native Git/HTTPS descendants during strict drain. `/usr/local/bin/git` is a native ELF
hardlink, not a wrapper. Default-enabled curated sync discards its worker handle; blocking transport,
lock, download and host shutdown need explicit cancellation and ownership. Do not disable plugins,
raise the strict drain budget, weaken assertions or explain away live descendants as zombies.

## Ordered next actions

1. Finish installer evidence publication after fresh main-parent check; nonforce update and verify
   every source/evidence blob and remote ref. Persist receipt outside temporary tool state.
2. Adopt curated Git transport Stage A tests-only, run red against the native helper, then adopt
   production with unchanged assertions. Run `just bazel-lock-update` for its dependency change,
   inspect the actual lock delta, run affected core-plugins/PTY suites, scoped fix and global fmt.
   Frozen stage: recovery `p03-curated-git-transport-proposal/MANIFEST.json`, SHA256
   `ad4e063abc4169dd8d46e15cded48c388770c6cfbacaf0cacd3da04d75603714`;
   archived unadopted as `p03-curated-git-transport-unadopted-stage.tar.gz`, SHA256
   `dd00735c3bf3e2209ed2919701f2969d7f26ce51395a2499416597d8109c09aa`.
   Linux bounded process-group transport only: uncompiled as of this document, no host cancellation
   wired yet, other platforms retain the old path. Stage A alone cannot close full-host failure.
3. Complete cancellable policy-aware lock/HTTP/body/extraction/fallback and recoverable activation;
   retain worker/admission and generation-owned global sync lease; track actual queued callback work.
   Add real in-process/external/CLI shutdown guards and report cleanup failure instead of success.
   Forced termination must report uncertainty unless an outer owner also contains child groups.
4. Build a fresh full CLI and rerun unchanged default-feature installed storage, migration and GUI
   cancellation/recovery/manager-shutdown gates with exact source/package hashes and strict runner.
5. Continue native auth source-aware load/refresh publication, conditional persistence and retained
   provider work; then installed auth/catalog contracts, selection and failure handling. Resume
   all remaining P04–P19 phases and final whole-platform acceptance, not merely this prerequisite.
6. Required P00M/P18U maintenance track: maintain upstream symbol/path/revision/customization maps
   now; after stable components implement separately installable updater and external bootstrap.
   Demonstrate real isolated later-upstream integration preserving custom plugins, semantic review,
   coordinated host/plugin/schema versions and failed-update/interrupted-migration rollback.
   Updater remains unimplemented. No periodic polling or live installation changes are authorized
   merely by the roadmap addition. Optional viewer transport remains a separate workstream.

## Preservation and resources

Recovery directory: `/workspace/recovery-backups/20260930T165936Z`.
Full original two-worktree/Git/SDK/evidence archive `codex-recovered-workspace.tar.zst`, SHA256
`3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
Installer source archive `p03-install-order-verified-source.tar.gz`, SHA256
`acd1b14e7a2df8de788ae4bec63ccb8d367ef45f3f6272695d71940a9ec7ed89`.
These filesystem archives are recovery checkpoints, not independently proven external backups.
Published GitHub source establishes external durability only for included paths. Preserve WIP
branches/worktrees and failed reports separately; no reset or destructive cleanup.

Current mutable CLI is still source99 SHA256
`c7111d534c600c35812b714c4a1a536348254e7e1d3aea10dd77a411b48ff021`.
Verified executable archive `p03-source-owner-cli-archive-preserved.tar.zst`, SHA256
`68102a992d8d7ebe3105933d5377c9ec215ed5d51c8f814891555f81fb098222`.
Preserve both current CLI hardlinks, eight prior GUI aliases and original live GUI App Server
PID 371365 (older frozen v2 binary); do not relabel them as current source or terminate them.

Root retired exactly 336 unreachable non-executable build-cache libraries (1,500,889,088 allocated
bytes), guarding source, archives, executables, fingerprints and depfiles. Receipt
`p03-installer-unused-cache-retirement.json`, SHA256
`a9b5280debb7ae25ea03666ed52cebdc514080efc6bd57597abcfd7f39bb0b73`.
Recheck free disk and actual cgroup headroom before commands; `/tmp` is nearly full. Old cleanup
inventories are stale after Rust work. Root is sole repository/Rust/cache/Git mutation owner;
workers stage proposals in recovery directories. No active Rust/runtime check at this checkpoint.

## Resume procedure

Verify identity/status and source/index before edits. Read root AGENTS.md. Source
`/workspace/toolchains/component-verification-env.sh`; set `CARGO_BUILD_JOBS=1` and
`TMPDIR=/workspace/acceptance/p03-source-owner-runtime-temp` (mode 0700). Use `/tmp/run-p02-check.py`
for unique report prefixes and actual before/after source maps, preserving failed runs. Existing
wrapper baseline labels are historical, not actual tested revisions. Use `just test --locked
--retries 0`, scoped `just fix`, global `just fmt`; never kill Rust or weaken lifecycle assertions.
Keep credentials/private acceptance reports out of tool output and publication; publish reviewed
safe projections only. No current API/framework change requires unavailable Context7 docs.

Original task environment ends `…c3db0403`, config revision 1458 observed running/connected/current;
restricted package-managers policy with additional allowed hosts `[]` remains enforced. In-app
Browser/Context7 unavailable; no supported external GUI preview proven. Do not reset/replace the VM
or bypass transport policy. Remote-viewer networking does not block component development.
