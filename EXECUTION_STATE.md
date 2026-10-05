# Codex Harness Compartmentalized — execution state

Updated 2026-10-04. **Partial platform; P03 remains incomplete.** Read this file,
[roadmap](IMPLEMENTATION_ROADMAP.md), [inventory](COMPONENT_INVENTORY.md), root
AGENTS.md and [upstream maintenance](UPSTREAM_MAINTENANCE.md) before resuming.
Full earlier receipts/actions are preserved in
[pre-runtime ledger](verification/2026-10-02/p03-auth-refresh-pre-runtime-ledger.md),
[prior snapshot](verification/2026-10-02/p03-auth-refresh-ledger-history.md) and
[checkpoint history](EXECUTION_CHECKPOINT_HISTORY.md).

## Current resume checkpoint — 2026-10-04

The same task-bound environment reattached and the original checkout/index survived.
The accepted maintenance0.1.1 source has8,964 paths/map
`285c1464a236cf391d7c7b06c68ab17388e8c99079c5fc4a091f8b7595daa36f`.
The adopted maintenance0.1.1 package closes the direct-Git cancellation gate:
58 focused cases, fresh external package acceptance23commands/19manager,
manager-only SIGINT cleanup and cooperative SDK shutdown all pass. Both cancellation
cases hold real Git on a verified FIFO, require tracked absence before release,
use no rescue, and then invoke successfully through unchanged managerf054.
Cooperative cancellation returns a typed cancelled result and exits0; abrupt manager
SIGINT exits-2 and is not graceful manager forwarding. See
[exact evidence](verification/2026-10-04/p18u-owned-git/README.md).

The two interrupted October2 wrappers finished0, but their detailed reports were
lost with `/tmp`; the new October4 reports/package identities live under A/p18u-resume-
20261004-* and are independent current proof. Original0.1.0 failure evidence remains.
The old object mirror is absent. Its promisor repair had completed (original config
SHA49772cc restored); do not replay it. Chosen revision2e5fea64 and custom914cc593
objects were reacquired into a bare **test object cache**, with no new checkout,
branch reset or working-tree replacement. Source-input staging now has separate acceptance below.

No earlier GUI/viewer/build processes survived reattachment; their disappearance
is not a successful shutdown test. Production CLI8e8a/managerf054 hashes survive.
Temporary memory increased, but missing backing now affects1,875 metadata files,
11 static libraries and1,484 registry archives. Overlay remains roughly362MB;
the prior output/overlap/reserve comparator alone needs2.16GB, before missing-input
recovery and remaining validation. Native admission stays closed; do not infer that
free tmpfs repairs the build. R/p03-reattached-capacity-audit-20261004-01 preserves
exact inventories. Keep auth/MCP proposals and every archive; no Rust build started.

The maintenance0.2.0 source-input capsule is adopted, formatted and runtime-tested,
8,967-path map `0b7c68b95eadc215474ac0466c07ae98a1f124ba3631f075babfc63155ad8269`.
Eleven capsule fixtures and13 reviewer regressions pass. Fresh external build and
49-command installed acceptance retain13 changed paths/25 blobs/1,228,513B from
actual upstream descendant2e5fea64 and committed custom914cc593. The divergent
custom scenarios.rs is preserved as an input, not silently replaced. Package0.2.0
pyz SHA `2e5adafa9a231ace5b7687c685447afedc7c6f81b230373ab6b05581259b018f`.
Cooperative cancellation and abrupt plugin SIGKILL both leave incomplete jobs;
actual Git cleanup is established with the FIFO held and no rescue, then new
manager invocations seal new capsules without changing interrupted jobs. Empty-PATH
inspection works after removal. This inspector verifies artifacts, not prior crash
durability. Terminal failure persistence is best effort under further I/O failure.

Fresh installed storage, one manual migration and two real Chromium GUI cycles
also pass on unchanged CLI8e8a/managerf054 and unchanged native package. The GUI
exercises approvals/native tools/Stop/streaming/search/cold history and actual
Launch Ctrl+C; deterministic inference is not live-provider or in-app Browser proof.
Private runtime reports/state/screenshots were exported out of /tmp to A before
publication. Overlay318,586,880B, tmp9,198,415,872B and hard-unused15,274,414,080B
were observed after the GUI; OOM/kill0 unchanged. No acceptance command remains active.
No Rust build was attempted, and missing cache backing is still unresolved.
One separately observed Git PID2006 remains Z/PPID1 after the first failed isolated
object-cache fetch; its causal owner is not independently attested. It is not
dismissed or counted as successful cleanup. Narrow acceptance reports prove only
their tracked process identities; no whole-host clean-process claim is made.

See [source-input evidence](verification/2026-10-04/p18u-source-capsule/README.md).
R/p18u-capsule-adoption-20261004-01 preserves exact preimages and formatted manifest.
A subsequent README-only correction qualifies best-effort failure-marker persistence;
current documented map `0d2ec165627ff4a0e9236ead57bd6f6e42c13512eacae2c00c84604541e5e515`.
Runtime code/tests are unchanged; original test/package hashes remain authoritative.
See verification/2026-10-04/p18u-source-capsule/POST_TEST_DOCUMENTATION.json.
Next feasible P18U action: produce a bounded sparse transformation from the sealed
inputs, preserving custom changes and rejecting conflicts/unsupported entries;
prove installation, interruption and host-independent inspection. A sparse overlay
must remain explicitly incomplete until coordinated build/version/migration,
real-host/UI compatibility and external failed-update recovery gates pass.
No merged candidate, update activation or rollback is accepted by this checkpoint.

## Identity, publication and preservation

- Original VM commands execute. No stale executor-access blocker remains.
- Checkout: `/workspace/codex-harness-everythings-a-plugin` (P); origin
  `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
  Preserve sibling `/workspace/codex-harness-next-components`. No new VM/checkout/chat.
- Upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`.
  Apache LICENSE/NOTICE/provenance retained. No Cordis/second-harness replacement.
- Latest verified implementation/evidence checkpoint: `1d94c37a6619aa11b9b252d9693ee79bbeb4e466`,
  tree `7278f32d25e43319e5300c47d0f514d44b49a45d`, same WIP branch. All20 selected
  files/319,269B and both refs read back; main unchanged. Receipt R/p18u-owned-git-
  publication-20261004-01/PUBLICATION.json SHA256
  `16bb1891271f009a0ad0bf8b355799dd192ab383a140d962c229a0ffd4558707`.
- Prior documentation checkpoint: `777ee7ce49eb284d1fea80c927207433ca64a9fc`, tree
  `4feff5ab7324d77c2824e6cd829eb6804457faf6`. All5 selected files/109,741B and
  both refs read back; records914cc593 acceptance, exact native capacity gate and
  preserved unadopted work. Receipt R/p18u-resume-ledger-publication-01/PUBLICATION.json
  SHA256 `8557006b06c4b94fbd864dd9037a309b76b75842d4e35addf387ad7449206387`.
- Prior verified implementation publication: `914cc59374c1149463e78bc33851d83e3f14d0a4`, tree
  `0f692771abe238cac759b2a034b5f8c820165236`, on
  `wip/p03-process-final-and-mcp-preservation-20261002`.
  All31 selected files/727,161B and both refs were read back. This publishes the
  additive maintenance package,50 focused cases,23 acceptance commands (19 manager),
  fresh migration/GUI evidence/screenshots, source mapping and unadopted auth archive.
  Previous ad2fc047 publishes source42 production/all12 runtime evidence. Source42
  itself is047e91e;15bd9d1 records scoped lint and34c0281 the409 package cases.
  Main remains `781080f7e3c8bfe1953378001d777dff33d74bc3`; no promotion implied.
- Publication receipt R/p18u-installed-impact-publication-01/PUBLICATION.json,
  SHA256 `1676e842e956b3d32dc5c5b9ae971937a6f74c7109bd6f16c111dbb2c001e31f`.
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

## Historical additive maintenance0.1.0 checkpoint

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
Published and read back at914cc593 on the existing WIP branch; main promotion remains blocked.

The original0.1.0 active-Git cancellation gap is closed by the separately verified0.1.1 checkpoint above. Real later-upstream integration,
coordinated versions/migrations, candidate activation and failed-update/state
restoration are not implemented or proved. The standalone entrypoint inspects;
it cannot recover a failed installation. P18U release gates remain open.

## Historical resource state before reattachment

The October4 observation above supersedes live-process and volatile-file presence claims here.

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

1. Preserve verified914cc593 and resolve native-build capacity before adopting the
   coherent8-path external-token-install ownership slice (432 preformat changed
   lines). [Capacity checkpoint](verification/2026-10-02/p18u-installed-impact/NEXT_NATIVE_CAPACITY.md)
   records the actual missing outputs and exact recoverable duplicate plan. Current
   established validation cannot fit; the duplicate-only option does not solve it.
   Do not repeat the build or retire copies under unchanged assumptions. R/p03-ephemeral-token-install-
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
5. Required P18U: installed review/cancellation and real source-input staging now
   pass, as recorded at the top. Advance sparse candidate transformation next while
   native capacity is blocked. Preserve exact custom base and input digests; reject
   ambiguous content/mode/deletion changes. Then coordinate versions/state migrations,
   real later-upstream integration preserving custom plugins/UI, incompatible-update
   rejection and external-bootstrap rollback. Input integrity or a clean text merge
   cannot satisfy those gates. No scheduled polling or unattended live update.
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
