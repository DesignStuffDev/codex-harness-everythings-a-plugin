# Codex Harness Compartmentalized — execution state

Read [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md) and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
**Full v1 is incomplete. Resume the ordered queue; plans and crate counts are not extraction proof.**

## Repository and publication

- Original cloud checkout: `/workspace/codex-harness-everythings-a-plugin`.
  Preserve `/workspace/codex-harness-next-components` and both worktrees' changes.
- Origin: `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
- Last completed remote verification before this evidence supplement:
  `7ad8b08c32530efcb7f57f9db6682e55a4808528`, tree
  `5eff6f1953ce75c7c7c3f40bbaf87bcca72123c2`. All 12 source/evidence/doc blobs
  and main were verified after a fresh-parent, nonforce update. Receipt
  `p03-source-owner-publication.json`, SHA256
  `580f8212c018689645b62ae02042804c11d2c1fcca488c222bd0bc8919a18555`.
  Native source is `99e6802f`; its scoped tests pass but overall runtime acceptance
  is blocked as detailed below. Last fully passing older runtime remains `b3530790`.
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

1. Native source-owner checkpoint `99e6802fd478e55686559aefdd6ae79177a45397`,
   tree `651cca62fb376fed10b50872165edda8d9b099e3`, parent `b3530790`.
   [Evidence](verification/2026-10-01/P03_SOURCE_OWNER_EVIDENCE.md) preserves a
   2-pass/4-assertion-failure/277-filtered baseline and **391 passing tests**:
   283 login plus 108 provider, no skips/retries. Only three production paths
   changed between runs; all six new tests stayed byte-identical. Initial lint
   passed unchanged with one fixture type-complexity warning. A private type alias
   resolved it; clean lint passed unchanged, and three mechanical formatter changes
   were independently reviewed. No tests were repeated solely for alias/formatting.
   All 8,888 final scoped files match the source tree. The strict runner returned
   0/null with an additional SIGKILL (-9) child reap whose attribution/cause is unknown.
   The source archive SHA256 is
   `28c110b6b7f526c2c11e4b98ba259ba0fc73f9716f4518d8113b9642d4074786`.
2. Repair inherited curated-plugin startup synchronization ownership before accepting
   full-host regression. Fresh CLI build **passed (4m38s)**, exact SHA256
   `c7111d534c600c35812b714c4a1a536348254e7e1d3aea10dd77a411b48ff021`.
   [Runtime evidence](verification/2026-10-01/P03_SOURCE_OWNER_FULL_HOST_EVIDENCE.md):
   both storage attempts passed 13 behavioral commands but strict descendant drain
   failed exit125; tracing observed live Git descendants. Migration passed 10 commands;
   two cold GUI cycles passed, including real manager active-turn first SIGINT exit0
   in 0.265s/0.267s with tracked processes absent and no forced cleanup.
   Overall full-host acceptance **failed**, and no test assertion/drain was weakened.
   All five build/runtime attempts retained the same 8,888 source maps. Preserve
   failures and sampled trace; [repair contract](verification/2026-10-01/P03_CURATED_SYNC_LIFECYCLE_DIAGNOSIS.md).
   The four investigated native lifecycle files match
   the upstream pin; no production lifecycle fix has been applied.
   Use fresh workspace fixture directories/TMPDIR. `/tmp` is nearly full and overlay
   headroom is too small for the next Rust gate until audited recovery of generated
   artifacts. No active Rust process; never kill Rust or remove source to make space.
3. After lifecycle/resource gates, adopt the reviewed installer-ordering stage based
   on final source `99e6802f`: recovery root `p03-install-order-finalbase-stage`,
   manifest SHA256 `63ed01b3beba1933dea700b05e24628720a68f4460a4c59f18c134cc97e93fb9`.
   Nine held-future tests, production +151/-21, zero-fuzz replay verified. It is
   **uncompiled and unadopted**. Its complete 37-file stage is preserved in
   `p03-install-order-finalbase-unadopted-stage.tar.gz`, SHA256
   `2fce7459d7386b9b59e318defcfd6e1bc970dc6cb4f9b96abdacdab959f3f6f5`
   (cloud-local only). First run tests-only red, then production plus full
   login/provider tests, scoped lint and formatting. Its latest-admitted intent,
   source/cache/policy checks and clear(None) invalidation do not close subsequent
   load/refresh authority or shared persistence. Preserve all original stages.
4. Carry authority through actual load/refresh success and failure, shared persistence,
   gateway retained work, HTTP retries/body/decode and native model-cache publication.
   Preserve legitimate account-bound factory revocation. Keep catalog activation
   fail-closed until connected lifecycle/cancellation/drain/quarantine gates pass.
5. Prove installed native/custom catalog consumers using separately built workers,
   unchanged hosts and actual GUI/headless recovery. Then continue every P04–P19
   inventory obligation with exact source/contract/customization provenance.
6. **P18U remains required and unimplemented**: independently installed maintenance
   component plus external bootstrap/recovery, isolated integration of a real later
   upstream revision, unchanged custom packages, incompatible-update refusal and
   failed activation/applicable interrupted-migration rollback. No polling/live
   update is enabled. [Maintenance contract](UPSTREAM_MAINTENANCE.md).

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
Latest observation kept the original config identity, running/connected, revision 1458,
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
was changed. Published source checkpoints are externally durable on GitHub; runtime/binary archives
remain cloud-local. Recheck free space before compiling the next slice.

Before the next build, root consolidated the unused historical provider CLI only
after verified archival. `p03-old-provider-full-cli-consolidated.tar.zst` SHA256
`e76f0c34e75df0d02f12ed1a6ca8366363a5cf1fd290186c328bafa94381bf16`
retains executable `70669f0b` and historical metadata; receipt SHA256
`2c77110620a8707499b8ff5d5facab61a1e1efd6c99ce185483cf52a05af786e`.
Current CLI, live GUI and eight historical aliases stayed intact. Overlay free
space was about949MB afterward; `/tmp` remained limited to about94MB free.

The 391-test executables were verified in
`p03-source-owner-green-test-executables.tar.zst`, SHA256
`39f803144a69103442e89fd2d29f234b0242a9546aa6edcd933a5caaf7077743`,
before retiring only the three generated paths; receipt SHA256
`f81a9934f3e84af42fa354ffb30a591a9085f4ba3ebdb54ac56ce47a8f35c099`.
Their bytes bind the pre-alias tested source. The later final-format full CLI build
started separately as `p03-source-owner-full-cli-build`; inspect its terminal report.
The unadopted installer draft is separately archived as
`p03-install-order-unadopted-stage.tar.gz`, SHA256
`45d7ae8cecda7a015b1bab6da1e56c8bfa3756d72f2685e138b621c6131a0dc6`.

## Current exact binary recovery

Owner CLI archive `p03-source-owner-cli-archive-preserved.tar.zst` is verified
member-by-member with zstd checksum and metadata, SHA256
`68102a992d8d7ebe3105933d5377c9ec215ed5d51c8f814891555f81fb098222`
(138,823,583B). Receipt SHA256
`2e3fe79f270d4645be9c7d5545bf683be2fdd238aba0622445b23051a6565ff1`.
Both Cargo hardlinks and eight historical GUI aliases were unchanged. Current CLI
is still a mutable build path; no Rust/source writers ran during acceptance, and
binary SHA was checked before/after. This is VM-local recovery, not an external
binary backup. The GUI screenshots are public evidence; private runtime reports
and authentication state must not be published.
