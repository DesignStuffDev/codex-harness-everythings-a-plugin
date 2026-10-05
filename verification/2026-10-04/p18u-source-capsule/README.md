# Separately installed upstream source-input preparation

Maintenance package `codex.maintenance.upstream-review` **0.2.0** adds
`tool:upstream_source_capsule` v1 beside the existing review tool v1. A package
built outside the harness, using the installed SDK, now prepares and verifies
actual source inputs from later pinned OpenAI Codex revision
`2e5fea64eefcaa19f48458b2386011b619f69c70` through unchanged host `f054d84a`.
This is progress toward P18U candidate preparation. It does **not** apply that
revision, assemble an adapted candidate, upgrade an installation or restore state.
The harness's imported upstream remains `d4205609`.

## Exact source and inputs

The seven-path implementation is bound to 8,967 scoped files/map
`0b7c68b95eadc215474ac0466c07ae98a1f124ba3631f075babfc63155ad8269`.
It follows published custody checkpoint `1d94c37a6619aa11b9b252d9693ee79bbeb4e466`.
All native Rust and existing SDK code remain unchanged; this adds no native
extracted family and performs no host rebuild. The
[source lineage](../../../upstream/p18u-source-capsule-lineage.json) separates new
project code, retained contracts and unchanged project-owned planner/validator
copies. Their license/notice and exact packaged-tool provenance remain intact.

The real gate uses these independent inputs:

| Role | Exact commit |
| --- | --- |
| Imported upstream base | `d42056091aded7feb1d88ac7e83972108b2aa478` |
| Chosen later upstream, verified direct child of base | `2e5fea64eefcaa19f48458b2386011b619f69c70` |
| Pinned custom committed composition | `914cc59374c1149463e78bc33851d83e3f14d0a4` |

The custom input is intentionally an older published composition, separate from
the current plugin implementation. The operation retained 13 upstream-changed
paths and 25 unique blobs totaling 1,228,513 bytes. For
`codex-rs/core/tests/suite/scenarios.rs`, base, upstream and custom blobs all differ;
all three were retained exactly. [SOURCE_INPUTS.json](SOURCE_INPUTS.json) maps
every path and blob, exact trees, source receipts and the still-incomplete review.
This preserves material for later adaptation; it does not demonstrate preservation
of custom behavior in an integrated updated host.

Root prepared an isolated bare object cache before plugin execution. The initial
fetch failed because an alternate's shallow boundary was missing. That failure and
the isolated-cache correction are retained in [OBJECT_SETUP.json](OBJECT_SETUP.json).
No new project checkout or replacement VM was created. The plugin itself neither
fetches objects nor alters Git configuration or source trees.

## Contract, ownership and failure handling

The request has exactly five fields: contract_version 1,
`purpose:prepare_source_inputs`, a hash-bound review request, its expected plan ID,
and explicit limits. Preparation recomputes the full review, verifies the plan ID,
requires a distinct descendant revision and rejects unavailable objects or unsupported
entry types. Review uncertainties remain in the manifest with
`gate_inventory_complete:false`; preparation never approves compatibility.

Only paths changed between base and chosen upstream are included, with their
committed custom variants. Unchanged paths, custom-only changes outside that set,
dirty work and a runnable source tree are excluded. Inputs support regular blob
modes 100644/100755. Per-job ceilings are 1,024 paths, 4 MiB per blob, 64 MiB total unique
blob bytes and 8 MiB manifest. These are not cumulative retained-state quotas, and
Git capture limits remain post-capture.

The host initializes state_dir through the SDK. Each attempt that passes preflight
exclusively creates a new private
`upstream-capsules/<uuid>` directory, writes an intent, then content-addressed
blobs, manifest and a seal binding intent/manifest hashes. Temporary writes are
renamed and directories fsynced; old jobs are not rewritten or removed. Files
remain owner-writable: immutability describes producer behavior, not filesystem
enforcement or protection against a malicious trusted package.

Producer success requires both `status:sealed` and
`durability:directory_fsync_completed`. A persisted terminal marker makes offline
inspection incomplete even when a seal is visible. Terminal-marker recording is
best effort: a combined seal-fsync and terminal-write failure is not tested and
cannot be reconstructed by a later inspector. Producer failure never grants update
permission. Preparation checks a 90-second deadline. The initial review retains its own
90-second Git-owner deadline; subsequent capsule Git work is capped to the
preparation deadline.
blocking filesystem operations are not subject to a hard wall-clock guarantee.
Late cancellation after a completed seal may leave valid sealed output.

The independent `capsule_bootstrap.py` verifies present hashes, lengths, references
and bounded structure with host and Git absent from PATH. A sealed inspection returns
`durability:not_attested_by_inspection`; it does not authenticate publishers,
revalidate Git ancestry, prove declared tree IDs from a full object database,
recover failure history, restore a deployment or reverse a state migration.
Every result retains `candidate_assembled:false`, `update_allowed:false`,
`activation_allowed:false` and `self_contained_candidate:false`.

## Completed evidence

| Gate | Actual proof |
| --- | --- |
| Focused checks | 11 capsule fixtures plus 13 existing review-adapter cases passed on exact current source. The 11 use real local Git fixtures with a mocked planner; they are separate from installed integration proof. |
| Real chosen-revision preparation | 49 controlled commands: independent installed-SDK build, removal of its disposable source copy before install, actual manager review/preparation, all 13 path entries and 25 blob identities compared with Git, stale-plan/budget negatives, standalone inspection, package removal, invocation rejection and retained-state inspection after removal. |
| Cooperative installed lifecycle |After actual admission and real Git/FIFO readiness, SDK shutdown returned unsuccessful `incomplete_or_unavailable` with `cancelled_or_deadline_reached`; plugin exit 0 and tracked absence within 0.116500672s. INTENT and TERMINAL retained, no seal/blob. |
| Abrupt installed lifecycle |After actual admission and real Git/FIFO readiness, SIGKILL targeted only the installed plugin. Plugin-9 and direct Git -9 were reaped; tracked absence within 0.074950169s. Intent retained without terminal/seal/blob; abrupt durability is explicitly not attested. |
| Subsequent operations |Both incomplete jobs were identified offline with host/Git unavailable. A fresh real-manager invocation after each interruption successfully sealed the 13-path capsule, while the earlier incomplete job stayed byte-identical. This is subsequent source preparation, not failed-installation rollback. |

The interruption cases invoke the installed entrypoint through the SDK protocol
directly; manager graceful-shutdown forwarding is not tested. Subsequent successful
preparation uses the real manager. Both lifecycle cases held the FIFO writer
through the saved primary outcome,
used no rescue and passed unchanged identity/reaping assertions. The helper's
admission-to-FIFO scheduling race is explicit: failure to establish the barrier
would fail the case, not be retried or waived. The two cases happened to establish it.
Large tracked lists are accumulated historical observations, not counts of
simultaneously alive processes. Source/strict wrappers bind the exact current
source; both outer real/lifecycle runners exited0 with no runner error.

The installed 0.2.0 `plugin.pyz` is 34,924 bytes, SHA256
`2e5adafa9a231ace5b7687c685447afedc7c6f81b230373ab6b05581259b018f`.
[EVIDENCE.json](EVIDENCE.json), [REAL_RUNTIME.json](REAL_RUNTIME.json),
[LIFECYCLE.json](LIFECYCLE.json) and [FOCUSED_AND_SOURCE.json](FOCUSED_AND_SOURCE.json)
retain commands, package/host identities, raw-receipt hashes and limitations.
Exact [real-input](reproduction/real_capsule_acceptance.py) and
[lifecycle](reproduction/capsule_lifecycle_acceptance.py) helpers are preserved.
The latter imports the hash-bound prior custody helper and unchanged SDK owner/
subreaper utilities; use fresh private output directories and current bound inputs.
Raw multi-megabyte source maps, lifecycle identity histories and private runtime
homes remain in workspace recovery evidence rather than duplicated in this report.

## Regressions and remaining gates

[REGRESSIONS.json](REGRESSIONS.json) separately records the fresh native-storage
check with reused independently built storage package, accepted host/CLI and
deterministic inference: streaming/tools/context/cold resume, three storage-child
terminations and native restoration pass. Storage strict reaping retains seven -9 and two 0 statuses; migration retains
four -9 and one 0. These are not claims of universally graceful descendant cleanup. A fresh 10-command migration regression also passes native/selected dry-run parity,
apply/idempotence, cold CLI and paginated App Server history, turn cancellation
and reaping; its nonzero adopted statuses remain explicit. Two actual Chromium/
Playwright GUI cycles also pass 49 and 38 browser commands: streaming, native tool
approval, Stop, cold history and nine native-search checks per cycle. Real manager
Launch exits 0 on first SIGINT in 0.314480728s and 0.364460721s respectively, with
tracked processes absent and final drains passed. Three root-reviewed screenshots
show [approval](screenshots/approval.png), [recovery](screenshots/recovered-2.png)
and [file-reference insertion](screenshots/search-inserted-2.png); no private
browser URL, credential or runtime home export is published. This cycle does not
invoke the maintenance tool through the GUI, and uses native search rather than
a selected external search component. In-app Browser remains unavailable; these
are actual Chromium fallback checks with deterministic inference, not requested
manual Browser or live-provider validation.

The 0.2.0 replacement/upgrade selection matrix has not been exercised; the earlier
0.1.1 review replacement proof retains its own package/source scope. Native auth/P03
capacity remains closed. Required updater work still includes adapting an actual
later-upstream candidate, building coordinated host/native-package/SDK versions,
compatibility/security/migration gates, preserved custom plugin and UI behavior,
incompatible-update rejection, and external-bootstrap recovery of a failed
installation and its prior state. No scheduled polling or live activation occurs.
Workspace-local source capsules and archives are not externally durable backups;
only included source/evidence actually published and read back has that durability.

## Documentation correction after acceptance

Only the example README changed after these tests, to qualify terminal-marker
persistence under a second I/O failure. Runtime code/tests remain byte-identical;
the tested package and source map above are not relabeled.
[Exact documentation delta](POST_TEST_DOCUMENTATION.json) binds the tested and
published documented maps separately.
