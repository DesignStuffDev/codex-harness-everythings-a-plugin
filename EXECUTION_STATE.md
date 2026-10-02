# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Partial platform; P03 remains incomplete.** Read this file,
[roadmap](IMPLEMENTATION_ROADMAP.md), [inventory](COMPONENT_INVENTORY.md), root
AGENTS.md and [upstream maintenance](UPSTREAM_MAINTENANCE.md) before resuming.
Full earlier receipts/actions are preserved in
[pre-runtime ledger](verification/2026-10-02/p03-auth-refresh-pre-runtime-ledger.md),
[prior snapshot](verification/2026-10-02/p03-auth-refresh-ledger-history.md) and
[checkpoint history](EXECUTION_CHECKPOINT_HISTORY.md).

## Identity, publication and preservation

- Original VM commands execute. No stale executor-access blocker remains.
- Checkout: `/workspace/codex-harness-everythings-a-plugin` (P); origin
  `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
  Preserve sibling `/workspace/codex-harness-next-components`. No new VM/checkout/chat.
- Upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`.
  Apache LICENSE/NOTICE/provenance retained. No Cordis/second-harness replacement.
- Last verified publication: `ad2fc04768ff7abc8bbd623de462dee6d9b69c07`, tree
  `eadf6d886f35b8b2af9b27086de3d139c9512c24`, on
  `wip/p03-process-final-and-mcp-preservation-20261002`.
  All24 selected files/608,325B and both refs read back. This checkpoint publishes
  exact source42 production/all12 runtime evidence, screenshots, preservation receipts,
  causal-test drafts and the updated roadmap/ledger. Prior15bd9d1 records scoped lint,
  34c0281 records409 package cases,047e91e preserves actual source42.
  Main remains `781080f7e3c8bfe1953378001d777dff33d74bc3`; no promotion implied.
- Publication receipt R/p03-auth-refresh-runtime-publication-01/PUBLICATION.json,
  SHA256 `e268c61b6d8effffe46d8806f297e9fade57f781afd7ae62d0561e15b00bcbee`.
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

## Current accepted source and runtime

Current production Rust baseline:8,949 files/map
`42e3899a688183ae740926d188204ea1222d24f79db1e4b9cb37d51aa046dc59`.
Candidate R/p03-auth-refresh-adoption-01/CANDIDATE_FORMATTED.json SHA
`7f6a7aaf7fd772b8b6aa66e172dfeba7af7ad3f3cce6839c0b2dbb90c713d4a9`.
Three-path permanent refresh-failure ownership fix: coherent source/cache/provider
capture and conditional failure publication. Successful storage writes, later dispatch
and provider-spawned tasks are not fenced by this slice. Exact upstream mapping:
[refresh lineage](upstream/p03-auth-refresh-source-lineage.json).

Focused4/full login301/provider108 pass; focused4 overlap301, total409 package cases.
Serial full-package execution, zero skips/retries; scoped login lint02 and required
format pass. Lint changed no source. Conditional port-early-return body execution
is not independently attested. Strict nonzero adopted statuses are retained without
causal or universally graceful-cleanup attribution. No full workspace result.

Production root37511 passed in4m18s with unchanged source42, OOM10/kill5 unchanged.
[Build proof](verification/2026-10-02/p03-auth-refresh-production-build/README.md).
New CLI SHA `8e8a5dac859825a910403fc85dc7aed4a4ddaf3cc1c7d585f3a0fb27a7d64dc0`;
reused manager `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.
Root88117 double-hash binding: A/p03-auth-refresh-current-production-build-01.binaries.json
SHA `2d8d116c6a10a3d68425389aa9109e90e05d7283a4e99f0dbeef670bcf3b73b0`;
R/p03-auth-refresh-current-production-runplan-01/POSTBUILD_BINDING.json SHA
`6bc100403f78a5588f383aecfba8984fa076bddc91de4c8f16a913c613cc3556`.
Historical78d aliases now resolve the new CLI; old receipts cannot validate it.

All12 current-source runtime recipes completed successfully on first attempts:

| Gate | Actual result |
| --- | --- |
| Installed thread storage |13 commands,3 tracked storage children terminated; reused independently built package, streaming/tools/context/cold resume/native restoration. |
| Four isolated manual migrations |10 commands each; dry-run parity, apply/idempotence, cold CLI/App Server history and cancellation. |
| Ordinary and selected-search GUI |2 Chromium cycles each; approval/native command/Stop/streaming/cold history/search insertion; Launch Ctrl+C and tracked processes absent. |
| Attachments |5 groups/17 commands; exact native upload path, custom cold reference, positive error fallback, unselected removal and builtin restoration. |
| Slow normal Launch |46.452335498s; held canonical UserMessage forwarded/acknowledged, native cleanup and cold recovery confirmed. |
| Forced Launch |Second interrupt: manager exit1 after2.040751248s, durability explicitly unknown, held message not recovered; cold restart/continuation passed. |
| Held Git/HTTP + owner postcheck |All8 cases and all8 featured-task checks pass; actual nested strict/observer/independent Git ACK reports preserved. |

Plan R/p03-auth-refresh-current-runtime-plan-01 MANIFEST
`bb778caee63e286b574886213393514b5a61d7a947a6fba711027f78767ddeca`.
Original reports A/p03-auth-refresh-newhost-* and A/p03-auth-refresh-held-*;
fresh runtime homes in matching `/tmp/*-runtime` directories. All bind unchanged
source and host/package identities; these are reused native packages, not fresh builds.
The [all12 runtime projection](verification/2026-10-02/p03-auth-refresh-runtime/README.md)
is reviewed and sealed: EVIDENCE SHAe67dfdb9927ca667d370e8c5bb361e3ed542402b3fe6ac4e73127fb871ff0cb0.
Publication and selected-file/ref readback are complete; next adoption remains separate.

In-app Browser/Context7 remain unavailable. Real Chromium via Playwright used
deterministic inference; root viewed current recovery and search screenshots.
This is not in-app manual testing or live-provider proof. Preceding7a GUI01 timeout
remains preserved with unresolved cause. Storage hold occurs before native forwarding.
Raw-input/response-only history, native attachment resolve/envelope/blob durability,
active constructor/lower-transport/MCP custody and whole-host graceful cleanup remain
unproved. Nonzero adopted statuses are not dismissed as zombies or universally graceful.

## Latest additive maintenance checkpoint

`codex.maintenance.upstream-review` v0.1.0 is now implemented and accepted as an
independently installed tool-v1 package. It is read-only review support, not a new
native extraction or completed updater. All50 Python cases pass (17 planner,
22 lineage,11 adapter). The end-to-end acceptance executes23 commands,19 through unchanged manager f054:
external SDK builds, installation, invocation, incompatible API rejection, explicit
replacement with observed executable marker, removal and packaged inspection with
Codex absent from PATH. Strict exit0/null; sole reaped command exit0. Every report
keeps `update_allowed:false`. The chosen same-upstream/ad2fc composition request
correctly retains unavailable local composition and historical ownership blockers.

The current additive test source has8,962 files/map
`930af666e1add9736265a8ca7631f7851657731e9f34e5efead6abfc836667b5`.
All original8,949 paths remain byte-identical. Two fresh runtime gates also pass:
manual migration and two-cycle real Chromium GUI regression (approval, native tool,
Stop, streaming, native search, cold recovery and actual Launch Ctrl+C). Host/native
packages are reused, not rebuilt. This GUI cycle does not invoke the maintenance
tool in an engine turn; in-app Browser/live-provider validation remain unavailable.

After tests, only the new SDK README changed to describe accepted evidence. Current
documented source map is
`27c3eda0c294717768fe7d40af7ddad8363e3f59f7c21784875b39addac4d4f6`,
R/p18u-installed-impact-adoption-01/CANDIDATE_DOCUMENTED_FINAL.json SHA
`75a04501ba35ad9fd0e099b6cc0d1ece2e1e5e8f8e23eea8ec8903bd1f25a746`.
No runtime source or tests changed after acceptance; original receipts retain930a.
[Evidence](verification/2026-10-02/p18u-installed-impact/README.md),
[GUI supplement](verification/2026-10-02/p18u-installed-impact/GUI_README.md),
[source mapping](upstream/p18u-installed-impact-lineage.json).
Publication pending on the existing WIP branch; main promotion remains blocked.

The maintenance adapter’s active Git cancellation remains fixture-only. Real later-upstream integration,
coordinated versions/migrations, candidate activation and failed-update/state
restoration are not implemented or proved. The standalone entrypoint inspects;
it cannot recover a failed installation. P18U release gates remain open.

## Resources and preservation

No current acceptance command remains active after root41618; preserved older GUI/viewer services remain. Final observed free
overlay448,634,880B, /tmp391,753,728B, hard-unused memory986,247,168B; OOM10/kill5
unchanged. Two exact ordinary rlib clean-page caches were advised DONTNEED to admit
the GUI; file identities stayed unchanged, no source/archive/cache file was removed.
Historical PIDs293798/293800/293801 are still unreaped Z/parent1; earlier absent
wording was incorrect. Their state is not used to satisfy any lifecycle assertion. Tmpfs and /dev/shm charge the same16GiB cgroup. Another substantial Rust
build is not admitted on unchanged assumptions. No broad cache deletion/process kill.
[Resource evidence](verification/2026-10-02/p03-auth-refresh-production-resources/README.md)
records complete preservation before exact four test-ELF and four ordinary-cache
retirements. Current test ELFs are archive-only; grouped-test cache rebuild debt is
442,593,280 allocated bytes. Current-four archive745dcc56 fully restored563 members;
manager archiveb0e0fcac and full restore remain. Preserve all complete archives,
seven symlink-backed core libraries, genuine registry download backing, existing GUI
and remote-viewer processes. Never replay actions after selector reuse.

## Ordered next actions

1. Publish the accepted additive maintenance reviewer and separate current-source GUI
   supplement on the existing WIP branch; preserve the unadopted auth-install cohort
   source archive with explicit untested status. Then review resource admission for
   the coherent8-path external-token-install ownership slice (432 preformat changed
   lines), without adopting an untestable Rust candidate. R/p03-ephemeral-token-install-
   cohort-proposal-01 remains unadopted/uncompiled; source-only archive35477172 is
   staged under verification/2026-10-02/unadopted-proposals/ephemeral-token-install-cohort.
   Do not count auth custody support or the additive reviewer as native extraction.
2. Remaining P03 test gates: TUI6+2 snapshots, App Server lifecycle/search/storage16
   and same-process replacement parents. Prior TUI OOM ran zero tests; do not retry
   unchanged resource blocks. App Server rmcp elicitation adds build cost. Advance
   independent feasible work when admission remains blocked. The narrower external-token-install ownership slice remains a reviewed R-only
   proposal. The additive maintenance reviewer is now accepted separately above.
3. Review/adopt complete Ephemeral storage/caller activation only after prerequisites.
   [Writer audit](AUTH_STORAGE_OWNERSHIP.md) and
   [contract snapshot](verification/2026-10-02/unadopted-proposals/ephemeral-auth-design/README.md)
   cover manager→policy→map, equal/absence ABA, logout/revoke and native RMW writers.
   R/p03-ephemeral-auth-primitives-proposal-01 is unadopted/uncompiled:3 paths,+452/−45,
   8 proposed tests, MANIFEST5be1110f. Caller proposal B is now sealed under
   R/p03-ephemeral-auth-caller-activation-proposal-01, MANIFEST3d164963;12 afterimages,
   no compilation/tests/adoption. Root source-only export271c91f4 has51 byte-verified
   members and is staged in verification/2026-10-02/unadopted-proposals/ephemeral-auth-callers.
   Five review units are not independently safe activation stages. A compliant safe
   implementation staging plan02 is preserved and remains unadopted. Additional
   cancellation and post-logout native reload causal test proposals are now drafted,
   cross-reviewed and root-read (234 added lines, no production changes), still unrun.
   Source-only exportf0c1e6fa1880f6e90e5a21e8df4174f36ecb312758af5e0ad031f1144568898c
   is staged in verification/2026-10-02/unadopted-proposals/ephemeral-auth-causal-tests.
   Complete candidate adoption and actual tests are required before acceptance.
   Do not call a primitive or partial writer patch a complete ownership fix/extraction.
   File/Keyring/Auto, shared secrets initialization and cooperative cross-process
   transactions need their own complete activation, migration/recovery and tests.
4. Integrate staged MCP custody in dependency order: Session prewarm/refresh,
   connection retirement, lower transport, pinned SDK completion, late admission;
   resolve marketplace queue transfer before pool close. Activate negotiated broker/
   model2 grants, then extract actual native OpenAiModelsManager discovery/merge/cache
   with separate native/custom build/install/select/remove/cancel/UI proof on unchanged
   host. Continue remaining P04–P19; three bounded families are not the endpoint.
5. Required P18U: installed maintenance plus external recovery; chosen later upstream
   integration preserving custom plugins/UI, coordinated versions/state migrations,
   incompatible-update rejection and rollback. Offline17+22 support fixtures do not
   satisfy those gates. No scheduled polling or unattended live update.
6. Publish reviewed milestones on existing WIP branch with nonforce/current-ref checks;
   retain accepted main until its gates pass. Preserve unfinished source separately
   and update this queue/evidence/provenance every run.

## Operating rules and coverage

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
