# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Partial platform; P03 remains incomplete.**
Read this file, [roadmap](IMPLEMENTATION_ROADMAP.md), [inventory](COMPONENT_INVENTORY.md),
root AGENTS.md and [upstream maintenance](UPSTREAM_MAINTENANCE.md) before resuming.
Detailed superseded receipts/actions are preserved in
[checkpoint history](EXECUTION_CHECKPOINT_HISTORY.md) and the
[latest chronological snapshot](verification/2026-10-02/p03-auth-refresh-ledger-history.md).

## Identity, publication and preservation

- Original VM commands execute. No stale executor-access blocker remains.
- Checkout: `/workspace/codex-harness-everythings-a-plugin` (P); origin
  `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
  Preserve sibling `/workspace/codex-harness-next-components`. No new VM/checkout/chat.
- Upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`.
  Apache LICENSE/NOTICE/provenance retained. No Cordis/second-harness replacement.
- Last verified publication: `41bafa170f8a8d943a19705d19146df427885152`, tree
  `7602e2c3676fdc3147857923e57a1a6ccbabdfad`, on
  `wip/p03-process-final-and-mcp-preservation-20261002`.
  All10 selected files/276,318B and both refs read back. Parent96a4339 publishes
  provider108/resource evidence; parent e996f0d preserves login297 and proposals.
  Main remains `781080f7e3c8bfe1953378001d777dff33d74bc3`; no promotion is implied.
- Publication receipt: R/p03-auth-reload-lint-publication-01/PUBLICATION.json,
  SHA256 `4d0d72f65c0a3d222ac30e9159e35b110159e58d56ea15a8a00e3f146f1c3115`.
- R = `/workspace/recovery-backups/20260930T165936Z`;
  A = `/workspace/acceptance`. Full two-worktree/Git/SDK/evidence archive:
  R/codex-recovered-workspace.tar.zst, SHA256
  `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
  Preserve later snapshots/proposals and every complete recovery archive.
  VM-local archives are recovery checkpoints, not proven external backups;
  only actually published GitHub source/evidence has verified external durability.
- Local HEAD/index remain upstream and local main is stale. Do not reset/stage the
  real index. Index SHA256
  `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.

## Current source and exact evidence boundaries

| Cohort | State |
| --- | --- |
| Current refresh42 | Three-path permanent refresh-failure ownership fix adopted after reload acceptance. Source8,949/map `42e3899a688183ae740926d188204ea1222d24f79db1e4b9cb37d51aa046dc59`. Four causal tests added; root6487 is compiling, **no test result yet**. |
| Preceding reload0742 | Source8,948/map `0742356b9ff442a4d38f6704928930602f92df0fdfe89bfb250897fda470729f`. Focused5, full login297, provider108 and scoped login lint03 passed; no new production/UI proof. |
| Last production7a | Source8,947/map `7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e`. Offline build02 and12 runtime slots have passing attempts, with original failures retained. This does not validate either later auth cohort. |

Current candidate: R/p03-auth-refresh-adoption-01/CANDIDATE_FORMATTED.json,
SHA256 `7f6a7aaf7fd772b8b6aa66e172dfeba7af7ad3f3cce6839c0b2dbb90c713d4a9`.
Preimages, exact adopted patch and format transition are preserved there.
`just fmt` root88742 returned0; root reviewed whitespace/line-wrapping-only changes
in the same3 paths. Original index unchanged. Exact
[upstream mapping](upstream/p03-auth-refresh-source-lineage.json) is recorded.
The fix captures one source/cache/provider snapshot before refresh and conditionally
publishes permanent failure; stale waiters still receive their original error.
It does not fence successful writes, request dispatch or provider-spawned tasks.

Reload proof: [focused5](verification/2026-10-02/p03-auth-reload-focused/README.md),
[full login297](verification/2026-10-02/p03-auth-login-full/README.md),
[provider108](verification/2026-10-02/p03-auth-model-provider-full/README.md),
[lint03](verification/2026-10-02/p03-auth-login-lint/README.md).
Tests used scoped `just test`, serialized full-package execution, zero retries/skips.
The focused5 overlap297. Provider strict0/null retained reaped{-9:1,0:1}, without
causal or universal graceful-cleanup attribution. Lint changed no source and passed
in2m29s after two separately preserved precompiler RAM-admission blocks. OOM10/kill5
remained unchanged. No tests were repeated solely for lint/format.

## Build assets and resource state — read before any command

- **Production CLI78d is temporarily archive-only.** Root40318 freshly verified
  all34 archive members/current ELF and inactive owners, then removed exactly its
  two hardlinks.16 historical aliases remain temporarily dangling. Separate older
  live GUI and remote-viewer directories are preserved; they are not current proof.
  R/p03-auth-production78d-archive-consolidation-01/RESULT.json SHA256
  `fefdeec0cd2cd53c5125bc385756d5ac4f050bd571d96add9eb5694d48fe2031`.
  Complete production archive SHA256
  `b4c79bcc83cb356d9f2707fb9a17a3deae10c86860e9ac601af25d8b8d8154bf`.
  Restore/rebuild production before its next runtime gate; no current CLI availability
  is claimed. Login test source does not consume this production binary.
- Executed0742 login library/integration are also archive-only for selector reuse.
  Pair01 audit stopped before advice/mutation on a ctime-only control pin; preserved
  failure is not hidden. Immutable pair02 audit53186/preserve51419/retire33970/
  release48205 returned0. All362 members fully restored/hash checked before exact
  two-ELF retirement and release of only the new restore.
  R/p03-auth-current-login297-proof-preservation-02 archive68,334,364B, SHA256
  `13b0eb7a5b1a90b0060240c0ad3347cd67f8280a024906eb0ba6c2dbbcd06ebd`;
  final release receipt SHA256
  `9607d47b338a9c06009a576ed53520eaa468dd9293e3043d383f658ec33e9c02`.
  Source42 is newer than these historical producer receipts.
- Compiled-only CLI21 library is separately archived (482 verified members):
  R/p03-auth-current-clilib21-artifact-preservation-01, archive SHA256
  `d255e568044c3ffc93fe6706cded1e9df0adec6401b75985de34707d16ef0fc4`.
  Oldba3 compiled library, CLI/Exec tests and all earlier proofs remain preserved;
  complete chains are in the historical ledger. Never replay completed actions
  after compiler selector reuse.
- Current manager and provider test ELF remain live/protected. Keep7 symlink-backed
  core libraries and genuine registry archive backing. Keep85 new lint metadata
  outputs (164,130,816 allocated B); no ordinary-cache cleanup has been authorized
  from their metadata delta alone.
- Last actual post-release resources: overlay1,144,639,488B; hard-unused RAM
  2,375,909,376B; OOM10/kill5. Root6487 now runs the focused compile/test gate.
  Focused refresh budget:845,438,976B overlay,512MiB executable SHM,2GiB hard /
  3GiB effective RAM; conditional credit is not free RAM. Fresh preflight must
  recheck these floors before admitting the compiler.
- Read-only audit found old431/542 generated verification copies total729,907,200B,
  fully resident and without observed owners/aliases. Alone they are insufficient.
  Exact retained archive/provider pages supply an additional measured319MB candidate.
  Root69520 completed the reviewed combined archive/member/restore-verified copy
  release plus bounded fsync/advice on five retained files. All archives remain;
  source42/index are unchanged. R/p03-old431-542-verification-copy-release-01/
  RESULT.json SHA256
  `337d2432f4dedba550ccfb378f160757432981b9ea7bb90547e9b0519eb382c1`.
  No compiler admission or new test pass is implied by this resource result.

## Ordered next actions

1. Preserve the completed old431/542 consolidation receipt and its exact two-file
   scope. Never replay this action; all recovery archives/source/current proof remain.
2. Inspect root6487 and A/p03-auth-refresh-focused-01 before any new compiler command.
   R/p03-auth-refresh-focused-plan-01/MANIFEST.json SHA256
   `da0cdf413acf5ffbfbb509724e7547d541ecc998677ac14fe59fff7c864b0ddf`
   binds source42, historical archives and four exact test names. Fresh admission:
   hard2,498,850,816/effective4,328,507,392B, overlay1,144,455,168B; OOM10/kill5.
   Keep unchanged CLI+login feature composition and strict observer/source/ELF guards.
   Distinguish discovery calls, actual executions and retries; no test pass yet.
3. Once focused passes, run the full scoped login gate serially. Reuse only the NEW
   focused library; discover counts dynamically, requiring existing reload5 plus
   new refresh4. Run affected provider regression and scoped lint/required format.
   Do not rerun tests solely for mechanical fix/fmt. Preserve original failures.
4. Rebuild actual production from the accepted combined source and close current
   installed storage/migration/replacement/shutdown/recovery/GUI gates. P03 TUI6
   with2 snapshots, App Server lifecycle/search/storage16 and same-process replacement
   parents remain open. Original TUI compilation OOM ran zero tests; do not repeat
   unchanged blocked builds. App Server rmcp elicitation has additional cache cost.
5. Review/adopt complete Ephemeral storage/caller activation only after prerequisites.
   [Writer audit](AUTH_STORAGE_OWNERSHIP.md) and
   [contract snapshot](verification/2026-10-02/unadopted-proposals/ephemeral-auth-design/README.md)
   cover manager→policy→map, equal/absence ABA, logout/revoke and native RMW writers.
   R/p03-ephemeral-auth-primitives-proposal-01 is unadopted/uncompiled:3 paths,+452/−45,
   8 proposed tests, MANIFEST5be1110f. Complete caller Stage B is being drafted.
   Do not call a primitive or partial writer patch a complete ownership fix/extraction.
   File/Keyring/Auto, shared secrets initialization and cooperative cross-process
   transactions need their own complete activation, migration/recovery and tests.
6. Integrate staged MCP custody in dependency order: Session prewarm/refresh,
   connection retirement, lower transport, pinned SDK completion, late admission;
   resolve marketplace queue transfer before pool close. Activate negotiated broker/
   model2 grants, then extract actual native OpenAiModelsManager discovery/merge/cache
   with separate native/custom build/install/select/remove/cancel/UI proof on unchanged
   host. Continue remaining P04–P19; three bounded families are not the endpoint.
7. Required P18U: installed maintenance plus external recovery; chosen later upstream
   integration preserving custom plugins/UI, coordinated versions/state migrations,
   incompatible-update rejection and rollback. Offline17+22 support fixtures do not
   satisfy those gates. No scheduled polling or unattended live update.
8. Publish reviewed milestones on existing WIP branch with nonforce/current-ref checks;
   retain accepted main until its gates pass. Preserve unfinished source separately
   and update this queue/evidence/provenance every run.

## Runtime baseline and operating rules

Production7a proof covers13 successful commands across12 planned slots: installed
thread storage/manual migration, attachments, ordinary/selected-search GUI, slow
normal/forced Launch shutdown, eight held Git/HTTP cases and owner postcheck.
Real Chromium used deterministic inference; in-app Browser/Context7 are unavailable.
Original search GUI01 cold-cycle60s timeout remains unexplained; retry02 passed.
Normal Launch waited46.4658s; forced second-SIGINT returned1 with durability unknown.
The hold precedes native forwarding; raw-input/response-only-history, native attachment
resolve/envelope/blob durability and universal cleanup remain unproved. CLI3/Exec9
are scoped historical7a passes; corrected Sep30 core2694 is not current workspace proof.

Root owns source/Git/compiler/cache actions. Use `/workspace/toolchains/component-verification-env.sh`,
one compiler job, offline settings and fresh resource/owner checks. Never kill Rust
commands. `just test`, scoped `just fix`, required `just fmt`; no direct cargo test.
Full workspace suite approval is absent. Preserve security/network fixtures and
subreaper assertions. Viewer networking is optional; do not retry its unchanged blocker.
No reset/force push/credential dump/scheduled polling/live unattended deployment.

Coverage: three bounded native replacement families—thread storage/manual migration,
inline attachments and file search. GUI is an additive installable presentation
package. HTTP/auth custody, host/API/SDK and adapters are not additional extracted
families. Most engine services, minimal-host composition and final plugin/UI/updater
clean-install acceptance remain unfinished.
