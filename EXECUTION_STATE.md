# Codex Harness Compartmentalized — execution state

Read [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md) and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
**Full v1 is incomplete. Resume the ordered queue; plans and crate counts are not extraction proof.**

## Repository and publication

- Original cloud checkout: `/workspace/codex-harness-everythings-a-plugin`.
  Preserve `/workspace/codex-harness-next-components` and both worktrees' changes.
- Origin: `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
- Latest remotely verified main: `3c846fe85a9f1eac0f1fa6ef73e9aee12c8089b3`, tree
  `b0cdae358b7a9bb0b146659003d5d7085579523e`, parent source checkpoint
  `18140083910d19c86e9cadaef0990db9aa652f6e`. All 14 affected source/evidence/doc
  blobs and main were verified after a nonforce update from fresh `192d353d`.
  Receipt `p03-cache-revision-publication.json`, SHA256
  `2a8de9055d4751ef2970764cce4dae8d029204ace60ac07ae82e34339c1eabb8`.
  The preceding runtime checkpoint `192d353d` binds native source `e289b6b5`;
  its full-host proof remains distinct from this newer source.
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

## Current source checkpoint and ordered next actions

Latest remotely verified main: `3c846fe85a9f1eac0f1fa6ef73e9aee12c8089b3`, tree
`b0cdae358b7a9bb0b146659003d5d7085579523e`. All 14 source/evidence/doc blobs matched;
publication receipt `p03-cache-revision-publication.json` SHA256
`2a8de9055d4751ef2970764cce4dae8d029204ace60ac07ae82e34339c1eabb8`.

1. Current immutable native source: `18140083910d19c86e9cadaef0990db9aa652f6e`,
   tree `36e47c268966b945ee967e4c13e73224cc6b85cb`, parent `192d353d`.
   [Cache evidence](verification/2026-10-01/P03_CACHE_REVISION_EVIDENCE.md) records
   385 passes (277 login +108 provider), no skips/retries; preserve the separate
   1-pass/7-failure baseline. Only two production paths changed between runs;
   all eight new tests were unchanged. Lint passed unchanged; three files were
   formatted mechanically. All 8,886 scoped final entries match the source tree.
   The unchanged strict runner returned 0/null; its extra adopted descendant
   exited by SIGKILL (-9), with exact fixture/cause attribution unknown.
   Source archive `p03-cache-revision-verified-source.tar.gz`, SHA256
   `e2bc6e7a5b0467e09f1ab367ed209ff44196a30975d3a8de3b26b0c1cbb33030`.
   Publication and remote verification completed as recorded above. Re-resolve
   remote main when resuming; never infer it from stale local refs.
2. Current-source full CLI and installed runtime acceptance passed; see
   [the additive runtime supplement](verification/2026-10-01/P03_CACHE_REVISION_FULL_HOST_EVIDENCE.md).
   Source `18140083` built in 4m 46s, CLI SHA256
   `876c826f76d574d9fee62011166b1ae3f4b15f69103f361aedb4e44cef08de71`.
   All successful build/storage/migration/GUI maps retain identical 8,886 entries.
   13 storage commands, 10 migration commands (plus a fresh workspace fixture repeat),
   and two GUI cold cycles passed with unchanged external packages. Both active-turn
   first manager SIGINTs exited 0 in 0.269s/0.270s, tracked processes absent, no forced
   cleanup. Strict runners passed; some adopted storage/migration descendants have
   nonzero statuses with executable/cause attribution unknown. Do not claim every
   child exited 0. Preserve GUI ENOSPC and subsequent fixture-path rejection separately;
   neither failure was rewritten or its guard weakened. Chromium/Playwright fixture
   inference is not in-app Browser/live-provider proof. No new attachment roundtrip.
   Archive `p03-cache-revision-full-cli-preserved.tar.zst`, SHA256
   `f941862faef55169ab16fa51a9dc3c177f9bdcaa8982d19fc5e5152bad70611b`,
   retains exact executable bytes; receipt SHA256
   `3840e024cefeba5ba314396628e720aa04ced086463ffe59b57c52e9a445cf33`.
   Root owns checkout/Rust/cache/Git mutations; workers stage only in designated paths.
3. Next reviewed source-owner plan: source/cache/provider publication under one
   owner, then latest-admitted installer and source/cache/policy fencing. Frozen
   plan manifest `bbe999f2ab518fc58803793bf2839aa19ffe4a199c99775cae1c4169a3cdb5c4`;
   archive `p03-native-source-publication-plan-bbe999f2.tar.gz`, SHA256
   `dad402783e9f3e369e47e3b65ef62a34f3759d9332b42f7bac0e5c20a6e3716d`.
   Corrected smaller candidate `/tmp/p03-source-owner-review-resumed` is statically
   reviewed and unadopted/untested: manifest `5badaee0`, six authored tests. Archive
   `p03-source-owner-corrected-review-stage.tar.gz`, SHA256
   `bff82c98f1ceac5e145d5db2580089e96f29ab54e1b368bc92624e3d55e23118`.
   Next apply its tests alone against recorded bases, preserve actual red evidence,
   then apply production and run login/provider regressions, scoped fix and fmt.
   Use `/tmp/adopt-p03-source-owner.py`; root reviewed the exact production/test diff.
   Mechanical co-location is not a complete source fence or installed extraction.
4. Carry authority through installer ordering/equal-byte replacement, actual
   refresh success/failure, shared persistence, gateway retained work, HTTP
   retries/body/decode and native model-cache publication. Preserve legitimate
   account-bound factory revocation. Follow the reviewed native plans and
   [leaf broker design](P03_LEAF_BROKER_DESIGN.md); do not activate catalog before
   its connected lifecycle/cancellation/drain/quarantine gates pass.
5. Prove installed native/custom catalog consumers with a separately built worker,
   unchanged host and actual GUI/headless recovery. Then continue every P04–P19
   inventory obligation, maintaining exact provenance each slice.
6. **P18U is required**: independently installed maintenance component plus external
   bootstrap/recovery, real later upstream integration in isolation, compatible
   custom packages, incompatible-update refusal and failed activation/applicable
   interrupted-migration rollback. No polling or live update is enabled.

## Recent resource preservation

The original source backup and all older evidence remain intact. Additional
cloud-local recovery archives retain the unused 7a63 CLI and all three failed
red-run test executables before their generated paths were retired. Their
receipts are `p03-old-p02b-full-cli-consolidation.json` (SHA256
`b1f0f7f0af4ff95689deb145b1486d971e338b5dc2dbfe823ef33a8f57532a94`) and
`p03-cache-revision-red-elf-preservation.json` (SHA256
`4d4c39f02d3dad5127b68d44e500113a2e8e1fb819d1c2234ce0e46c76309601`).
The failed run remains failed; the login integration executable was compiled but
had zero selected red cases. These archives are not external backups.
The green 385-test executables are separately preserved in
`p03-cache-revision-green-test-executables.tar.zst`, SHA256
`1889c40e3680202cd431f079ab6c3e4c6c28926a1af1fc27690226c9e8510b43`
(92,981,660B), before exact three generated paths were retired. Receipt SHA256
`def6d43e2cd75031b027c18c64d25b0a84d61513a5949d281a985eb7b0d6a24b`.
Unadopted source-owner review draft (21 files) is preserved as
`p03-native-source-owner-review-draft.tar.gz`, SHA256
`0eafdcaa73cc2fa2529f05431b8003d313ab65df4d5d1ac7d47778e9758f8fe8`.

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
Latest observation kept the original config identity, running/connected, revision 1457,
restricted package_managers, policy enforced, allowed_hosts empty and no exposed preview.
The earlier policy metadata reported VPN unconfigured; no networking settings changed.
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

## Latest runtime resource boundary

The original GUI gate failed because `/tmp` filled. Its raw wrapper/log/strict record
and empty private report are preserved. Root copied and verified the completed
migration fixture's 153 files/63 directories into workspace storage, retained the old
path via symlink, and recorded receipt `p03-cache-revision-runtime-relocation.json`
(SHA256 `2f1b43ca2c999e5da05e0eae38982dc326166e0518883b83722420b9f0fa8fc5`).
The unchanged GUI isolation guard rejected that canonical-path change, so root
created a fresh migration fixture directly in workspace and ran a third GUI gate.
Use the workspace fixture/results; preserve both failures. Future test TMPDIR must
have measured space; free `/dev/shm` is not an available cgroup allocation budget.
No live GUI process, native source, secondary worktree or lifecycle assertion
was changed. Current code is externally durable on GitHub; runtime/binary archives
remain cloud-local. Recheck free space before compiling the next slice.

Before the next build, root consolidated the unused historical provider CLI only
after verified archival. `p03-old-provider-full-cli-consolidated.tar.zst` SHA256
`e76f0c34e75df0d02f12ed1a6ca8366363a5cf1fd290186c328bafa94381bf16`
retains executable `70669f0b` and historical metadata; receipt SHA256
`2c77110620a8707499b8ff5d5facab61a1e1efd6c99ce185483cf52a05af786e`.
Current CLI, live GUI and eight historical aliases stayed intact. Overlay free
space was about949MB afterward; `/tmp` remained limited to about94MB free.
