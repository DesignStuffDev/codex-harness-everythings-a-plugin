# Codex Harness Compartmentalized — execution state

Read [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md) and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
**Full v1 is incomplete. Resume the ordered queue; plans and crate counts are not extraction proof.**

## Repository and publication

- Original cloud checkout: `/workspace/codex-harness-everythings-a-plugin`.
  Preserve `/workspace/codex-harness-next-components` and both worktrees' changes.
- Origin: `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
- Last remotely verified main before this runtime supplement:
  `f9e922c35fc9639e66e775837139d01a90c0c4aa`, tree
  `e64c32309f5ad12f4cf4dcbe1c8c5aefba8bcd35`. Source parent:
  `e289b6b5a2f6608779059963aef73c8cc66148eb`, tree
  `04ca864b074159df73d6914c851da59ba3444424`. All 15 affected final blobs matched.
  Publication receipt `p03-native-acquisition-publication.json`, SHA256
  `96d94a76ef509a443a49c83d3fc59286a5079bde20d9686cbdad80037e44d82a`.
- Official upstream remains `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`.
  Exact-tree import `ae720ae9a98bad29ca2cff998e7d5baaf05cec86`; retain LICENSE/NOTICE.
- Local HEAD/index remain at the original upstream pin; local main is stale.
  Never reset/rebase or manufacture local commits to match connector publication.
  Original index SHA256: `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.
  Use temporary-index exact trees, a fresh remote-ref check, nonforce update and remote verification.

## Verified checkpoint and limits

Native installed replacements cover recorded thread-storage/manual-migration,
inline-attachment and bounded-search **subsets**. GUI is an installed additive client.
Other subsystems remain coupled, adapters or compiled prerequisites.

- [Native reload/acquisition evidence](verification/2026-10-01/P03_NATIVE_RELOAD_POLICY_EVIDENCE.md):
  **271 login tests passed**, no skips/retries. Preserve the original 266-pass candidate
  and separate 2-pass/3-failure/266-filtered baseline that exposed its missing case.
  Final lint changed nothing; formatting changed six files mechanically. No tests
  were repeated solely for formatting. This checks policy at in-memory publication
  and current credential acquisition; it is not installed auth/catalog extraction,
  source/credential lifetime closure, persistent-write or final-dispatch authority.
- [Fresh full-host evidence and screenshots](verification/2026-10-01/P03_NATIVE_AUTH_FULL_HOST_EVIDENCE.md):
  full CLI built in 5m58s. All four build/runtime gates retained the same 8,884-source
  maps. **13 storage commands, 10 migration commands and two GUI cycles passed**.
  Unchanged independently built packages were reused. Both active-turn first SIGINTs
  reached the component manager: exit0 in 0.215s/0.265s, tracked processes absent,
  no forced cleanup. Chromium/Playwright and deterministic inference are explicitly
  not in-app Browser or live-provider proof. Attachment roundtrip was not rerun here.
- Actual tested CLI: `codex-rs/target/debug/codex`, SHA256
  `ed571c6fdde14295bd574e1526b8a4957b2f2825ca8d1e94782cc3c41471b342` (634,836,352B).
  Runtime used an exclusive no-writer window and before/after byte checks. Rename
  freezing stopped before mutation on eight old GUI arg0 symlinks. Both Cargo
  hardlinks and these symlinks remain unchanged; this is a mutable runtime path.
  Exact verified archive `p03-native-auth-full-cli-preserved.tar.gz`, SHA256
  `af19c2889ec5aff904d5a5b07c34af0663092ed8636ea0fc84efbeb981eb3c7a` (171,655,268B).
- [P00M supplement](verification/2026-10-01/P00M_PROVIDER_SCHEMA_EVIDENCE.md):
  22 fixture passes; immutable-object check exit2 with 0 invalid/1,167 unresolved.
  The frozen d04 index has 1,092 paths/28 map shapes/27 edges. Metadata recognition
  is not semantic closure, an implemented updater or a later upstream integration.

## Ordered next actions

1. Verify original environment/filesystem, current remote ref, processes and resources.
   Publish this completed runtime supplement with exact file/ref verification.
   Root owns checkout/Rust/cache/Git mutations; workers stage only under `/tmp`.
2. Reclaim sufficient build space through reviewed reversible artifact consolidation.
   Last observation: about 194 MB overlay /176 MB tmp, insufficient for another large
   link. An unused older P02b CLI can potentially be preserved in a verified archive;
   no action is accepted solely from this plan. Preserve live verified-v2 GUI, current
   d04/new CLI, all source and old arg0 links. Repeat identity/reference/hash checks.
3. Next reviewed **untested** candidate:
   `/tmp/p03-native-cache-revision-stage/FINAL_MANIFEST.json`, SHA256
   `226a211d21ade16bb22cc754bb0548b5f51d9ca9610b1eebb1cd723e0ca426b6`.
   Six paths/eight authored tests; private retained credential revision at real
   load/commit, plus held-resolver retained-endpoint HTTP cases. Adopt its separate
   tests-only patch first, preserve the failing baseline, then reviewed production
   and complete login/provider regression, lint/format and real-host gates.
   Staged archive `p03-native-cache-revision-stage-226a211d.tar.gz`, SHA256
   `da6605a945dc5902b6291aa02264695789739f607c1d678024f416f8c8fa699c`, covers28 files
   but is cloud-local only. Do not count this candidate as tested or published.
4. Carry authority through actual source-owner replacement, persistence, gateway
   retained work, HTTP retries/body/decode and native model-cache publication.
   Preserve legitimate account-bound factory revocation in tests. Follow the
   [native prerequisite audit](component-sdk/design/native-catalog/NATIVE_CATALOG_NEXT_CHECKPOINT.md)
   and reviewed plan archive `p03-catalog-async-authority-plan-02.reviewed.tar.gz`
   (SHA256 `dc2b0cd61267daf1f75c54d35cce2d1c476a425c6155276d86de924fb7a4fbe4`).
5. Implement [leaf broker](P03_LEAF_BROKER_DESIGN.md) admission, retained lifecycle,
   cancellation/drain/quarantine and actual native catalog composition. Keep catalog
   rejection fail-closed until coordinated activation. Prove installed consumers
   and native/custom catalog replacement with unchanged host and GUI/headless recovery.
6. Continue every P04–P19 inventory obligation and maintain provenance each slice.
   **P18U is required**: independently installed maintenance component plus external
   bootstrap/recovery, real later upstream integration in isolation, compatible
   custom packages, incompatible-update refusal and failed activation/applicable
   interrupted-migration rollback. No polling or live update is enabled.

## Operating and recovery instructions

Read AGENTS. Source `/workspace/toolchains/component-verification-env.sh`, then set
`CARGO_BUILD_JOBS=1`. Use `just test --retries 0`, never `cargo test` or killing Rust.
`/tmp/run-p02-check.py` records actual scoped hashes; its baseline field is stale
local main, not tested commit identity. Lifecycle gates use unchanged
`component-sdk/tests/subreaper_runner.py`. Do not weaken process assertions.
Use scoped `just fix` then global `just fmt`; no rerun solely for mechanical format.

The 16 GiB cgroup carries substantial fixed shared memory; free `/dev/shm` capacity
is not a safe allocation budget. Retired caches require exact audits/receipts;
completed test executables were archived before retirement. Some inactive storage
copies share inodes: copy before write/chmod/utime. No cache mutation during Rust.

Environment ID: `ccarenv_b64_Y2NhcmVudl8wYzAyOTNkMzVhZTg4MTkxYjc4YjQyZmVhNWYwMDllYQ`.
Latest observation kept the original config identity, running/connected, revision89,
restricted package_managers, policy state unknown, no extra hostname/VPN/preview.
Root did not reset or replace it. Revalidate. In-app Browser/Context7 remain unavailable;
optional viewer networking does not block extraction. Keep private URLs/logs private.

Recovery root: `/workspace/recovery-backups/20260930T165936Z`.
Full original source/Git/SDK/evidence archive `codex-recovered-workspace.tar.zst`, SHA256
`3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
Detailed prior execution state, all adopted/staged source archives, gate maps/logs,
publication receipts and resource journals are preserved there or in
`/workspace/acceptance`. Archives are cloud-local; GitHub protects included source,
not private state/binaries. Preserve both worktrees and existing WIP branches.
Continue feasible work; record exact next action and limits before any run ends.
