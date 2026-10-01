# Upstream maintenance and recovery contract

Status: **required, planned component; no updater is implemented or activated**.
This workflow preserves a maintainable OpenAI Codex fork as native services become
independently installable. It does not count as extraction of an existing engine
subsystem. It must ship as its own replaceable package, with an external recovery
entrypoint that still works if the candidate harness cannot start.

## Recorded starting point

- Upstream: <https://github.com/openai/codex>, exact revision
  `d42056091aded7feb1d88ac7e83972108b2aa478`.
- Root import: `ae720ae9a98bad29ca2cff998e7d5baaf05cec86`; its tree
  `147ac2447134294359c4071b0aeb495922760db7` equals the pinned upstream tree.
- Verified source checkpoint: `4e38e02993df698cefa99d7cc325890d837262de`
  on the target repository's published `main` at recovery publication.
- Separately preserved unverified work:
  `0b38d5974159ab2a8776200025dbb602e89b8f8f` on
  `wip/recovered-next-components-20260930`. Its evidence does not validate WIP code.
- The import is a root snapshot, not an invented continuation of missing upstream
  ancestry. Future comparisons use exact upstream revisions and trees explicitly.

[Provenance](UPSTREAM_PROVENANCE.md), [validation](VALIDATION.md), and
[recovery scope](RECOVERY.md) remain the evidence sources.
[Initial lineage](upstream/lineage.json) inventories all 617 changed paths between
the import and verified checkpoint, including documentation/evidence. A path is
not necessarily an extracted component. The map records exact Git objects,
areas, selected boundary symbols, extraction status and unknown semantics.
`current_blob` means that immutable verified checkpoint, not today's dirty file.

The subsequent [P01 lineage](upstream/p01-migration-lineage.json) maps native manual
rollout migration to its optional storage capability, retained worker lifecycle,
selected CLI composition and shared telemetry. It records exact upstream symbols,
the immutable P01 implementation commit, all changed blobs and the complete diff
from the import. The initial snapshot above remains historical. The subsequent
[P02 StageA lineage](upstream/p02-search-lineage.json) records exact native search,
App Server/TUI ownership, foreign Nucleo patch, SDK and additive GUI boundaries at
`25b2c3150879761026086a62e480045fc38d32ff`. It includes every changed path/blob and
78 current symbol anchors, with foreign provenance kept separate. This is a verified
native prerequisite, **not** installed search or updater execution. Unregistered
StageB drafts are excluded; each accepted boundary needs its own mapped checkpoint.
The later [StageB support map](upstream/p02b-contract-lineage.json) records the
neutral API and path-codec extraction into shared libraries, preserving the
distinction between support refactoring and installed service replacement.
The [matcher correction map](upstream/p02b-matrix-lineage.json) records an
inherited foreign dependency extent bug, its exact patch and safely demonstrated
regression. This does not satisfy P18U's later official Codex integration gate.
The separately preserved [bounded-search WIP map](upstream/p02b-bounded-lineage.json)
records all35 changed source blobs, original native queue/completion ownership,
foreign matcher allocation changes and new process-transport contracts at
`98c540863b9868eb216dba18f963c220c4be5187`. Its focused tests do not establish a
new accepted CLI/GUI runtime or installed search replacement. WIP provenance is
retained independently of acceptance so later updates cannot mistake unfinished
customizations for verified component behavior.

The [P02B integration WIP map](upstream/p02b-integration-lineage.json) binds
**81 changed source/build/SDK paths** and 130 selected symbol anchors to
`bb03f3a81a828871e4e35d3e46b9c8528f3b7d00`, tree
`6ad17d58936b6c9f519c06f2932efd1c8ecd4724`, compared with accepted main checkpoint
`59e5d576681d6cbb4977e9ccfaf5fe22bbeb2f15`. It maps native index budgets and
typed cleanup ownership, the foreign allocation changes, paired transport and
payload limits, immutable child cwd, `file_search1` catalog/service/process
adapters, and the existing in-process client's shutdown repair. Workspace
membership rises from 166 to 167 by adding `codex-file-search-component`;
this is not a count of independently installed components.

The [native budget](verification/2026-10-01/P02B_NATIVE_BUDGET_EVIDENCE.md) and
[process adapter](verification/2026-10-01/P02B_SEARCH_PROCESS_EVIDENCE.md)
gates identify their tested bytes and later source transitions separately.
No installed native search worker, production client selection or new GUI
search runtime is established at this checkpoint. The
[client shutdown evidence](verification/2026-10-01/P02B_CLIENT_SHUTDOWN_EVIDENCE.md)
also keeps the initial ENOSPC build without tests, the 39/40 retry, and the
corrected 40/40 fixture run separate. That fixture correction is outside the
mapped immutable WIP; later successful checks do not retroactively validate
different bytes. None of these gates integrates a later official upstream
revision or satisfies the updater and rollback acceptance requirement.

The subsequent [native backend map](upstream/p02b-native-backend-lineage.json)
records the **14-path delta** from that WIP to
`cb977e7d664bc752c28854d468619039bd3ad167`, tree
`40207f354efb3bee85265d8f3d83312489b56709`: 13 native source paths and the
corrected client test fixture. It maps the real `NativeSearchBackend`, aggregate
reservations, retained lease/query/poll ownership, explicit input policy and
native snapshot allocation preflight to original and previous symbols.
The [141-case native gate](verification/2026-10-01/P02B_NATIVE_BACKEND_EVIDENCE.md)
and subsequent reviewed mechanical lint/format changes remain separate; the
40-case client gate preceded native backend application. This advances actual
native behavior behind the neutral interface, while installed worker, production
consumer and GUI integration remain unverified at this checkpoint. Its workspace
still has 167 members; the newer worker source is outside this immutable map.

The [selected-search map](upstream/p02b-selected-search-lineage.json) binds the
57-path source delta through WIP `651b0b87a281934e5cd7c639021049fad8d393ac` to the
original native traversal/matcher, explicit runtime ownership, worker package,
standalone CLI selection and SDK export/build changes. The working composition
has 169 members. Its [evidence](verification/2026-10-01/P02B_SELECTED_SEARCH_EVIDENCE.md)
includes a fresh external worker build and actual installation, result parity,
cancellation, failure and removal through unchanged host binaries. This is one
native caller's replacement proof; App Server, TUI and private storage search
remain unfinished. A normalized cross-map ownership/completeness checker and
automated contract/security/migration impact analysis are still future P18U work.
No later official Codex revision or updater rollback is tested by this checkpoint.

The subsequent [App Server consumer map](upstream/p02b-app-server-lineage.json)
binds all 62 changed files and 44 original/current symbol anchors to source
`30c2674600cc84da161562c407515b61511b7c3a`, tree
`86c0bce7a5b399c2337c7c3878ed12ee76da4918`. All 8,739 frozen build fingerprints
match that tree. Its [evidence](verification/2026-10-01/P02B_APP_SERVER_CONSUMER_EVIDENCE.md)
adds real App Server selection of the independently built native search worker.
The map distinguishes extracted search from compiled App Server/client adapters,
protocol compatibility and the then-unverified TUI/GUI work.

The subsequent [TUI/full CLI consumer map](upstream/p02b-tui-consumer-lineage.json)
binds the TUI adoption, acceptance tooling and explicit post-build source transitions
to main `a469cf4be85fda40c64e563ece3e1639e4abdbac`, tree
`3216dfb131f72939c986902053665cbad85ce610`. Its
[evidence](verification/2026-10-01/P02B_TUI_CONSUMER_EVIDENCE.md) includes real installed
TUI, storage/migration and GUI behavior. The GUI preflight fixes were exercised
separately after the host build; two final Rust call-format changes are mechanical.
These lineage records support future maintenance; neither the updater nor a later
official upstream revision integration has run.

The [pending-start SDK map](upstream/p02b-startup-sdk-lineage.json) records four
custom support paths at `d22cea88aa23e35a619d996fc731122450e51313`, including
public ticket/control/receipt symbols and their exact tested/formatted bytes.
This is additive contract infrastructure; the required native/process cancellation
integration and installed updated-worker acceptance remain separate work.

The later Preparing maps separately retain the [native owner](upstream/p02b-preparing-native-owner-lineage.json),
[native backend](upstream/p02b-preparing-native-backend-lineage.json),
[process control](upstream/p02b-preparing-process-lineage.json),
[required startup/service API](upstream/p02b-required-start-service-lineage.json),
[runtime](upstream/p02b-runtime-preparing-lineage.json) and
[App Server consumer](upstream/p02b-preparing-app-server-lineage.json) changes.
These are versioned customizations on the same upstream pin, preserved on the
cancellation WIP branch with their actual tested/formatted source transitions.
Those individual focused gates alone did not establish a full-host checkpoint.
The later aggregate was accepted and promoted to main
`c28a1c33a856a987316f4b97b4c488d68055fffc`, with exact runtime/UI limits below. The
[worker04 build map](upstream/p02b-search-worker04-lineage.json) and
[old-host compatibility map](upstream/p02b-worker04-old-host-compat-lineage.json)
distinguish source API revision 2 from unchanged wire contract 1 and package
version 0.2.0. Installed interoperability with the accepted host passed; newer
host Preparing and GUI gates remain separate. An updater must preserve these
distinctions instead of treating a package version or successful merge as proof
that every consumer has upgraded safely.

## Separately installable maintenance service

Proposed selector: `maintenance:upstream`, contract version 1. This kind is not
currently admitted by the component host. Its package must contain its worker,
manifest, license, documented CLI/bootstrap entrypoint and conformance tests.
Install/remove must use the same immutable package mechanism as other components.
Restart activation is sufficient; no live engine replacement is required.

The host admits an explicit maintenance job and renders progress. The worker owns
its job journal, fetched objects, candidate source directory, analysis, build/test
processes and reports. Git/build/test executors, artifact storage and optional
review submission are explicit dependencies, not implicit access to host secrets.
The first implementation may use a bounded local command adapter with declared
paths. A component process is trusted code, not an OS sandbox.

| Operation | Input | Output / ownership |
| --- | --- | --- |
| `plan` | Repository identity, current checkpoint, chosen upstream SHA, policy, lineage digest | Immutable job ID, change inventory, gates and unresolved decisions; no source application |
| `stage` | Approved plan digest, clean candidate location, expected source identities | Isolated candidate, exact upstream/custom patches and conflict inventory |
| `validate` | Candidate digest, compatibility matrix, fixture/state snapshots | Per-gate evidence with tested tree, commands, outcomes and omissions |
| `status` | Job ID and event cursor | Ordered progress and terminal state; bounded events with artifact references |
| `prepare_checkpoint` | Successful gate manifest and candidate digest | Reviewable source commit, provenance, release/rollback manifest; no automatic activation |
| `cancel` / `close` | Job ID, reason and cleanup budget | Admission stopped, accepted work drained or explicitly uncertain; resumable journal |

These are proposed names, not callable methods. Wire DTOs must use owned values,
explicit contract versions and stable errors: conflict, stale plan, unsupported
version, failed gate, cancelled, cleanup uncertain and artifact unavailable.
Failures must not be inferred from raw command output. Record diagnostics in
redacted artifacts and keep credentials out of patches, journals and reports.

The job state machine is planned → staged → validating → reviewable, with explicit
failed/cancelled states. Publication and activation are separate transitions.
No implicit retry may replay a source mutation, migration, push or activation.
Cancellation preserves accepted artifacts and marks incomplete gates; shutdown
joins owned processes according to their execution contracts. A lost connection
does not mean a build, migration or remote write was rolled back.

## Chosen-revision workflow

1. **Freeze inputs.** Verify target repository identity, source/worktree state,
   remote refs and the chosen immutable upstream SHA. Record tracked, untracked
   and isolated WIP source separately; preserve them before staging. Do not use
   a branch name as a reproducibility identifier or change the target `origin`.
2. **Fetch without applying.** Fetch the chosen revision and needed ancestry to a
   dedicated upstream namespace/object store. Verify commit/tree identity and
   retain LICENSE/NOTICE changes. Report authenticity evidence available; a SHA
   identifies content but alone does not establish its publisher's identity.
3. **Inventory impact.** Diff old versus chosen upstream, then join paths/symbols
   to lineage. Review moves, deletions, generated schemas, dependencies, feature
   flags, configuration defaults, managed policy, licenses and platform changes.
   Include native code not yet extracted. Unknown impact remains a gate.
4. **Create an isolated candidate.** Use an existing verified commit and resolved
   revision before creating a worktree. Keep the active checkout, runtime and
   WIP source intact. Reapply our maintained contracts/adapters against the new
   native behavior; a clean textual merge is not semantic compatibility proof.
5. **Adapt deliberately.** Preserve custom component selection, lifecycle,
   cancellation, state and failure semantics. Update both native defaults and
   external implementations. Record each conflict resolution with source
   references, rationale and the tests that establish the intended behavior.
6. **Validate the exact candidate.** Build new host and affected native packages;
   execute the compatibility/security/native/runtime gates below. Save source
   manifests and patch digests before testing; any later semantic edit invalidates
   affected evidence. Preserve original failed runs separately from reruns.
7. **Prepare a reviewable checkpoint.** Record old/new upstream SHAs, import and
   candidate trees, custom patch series, component/version/state matrix, license
   changes, passed/failed/omitted gates, migrations and rollback artifacts. Commit
   coherent verified work separately from unfinished experiments.
8. **Publish/activate under applicable authorization.** Recheck destination refs,
   use a non-force update or review branch, then verify remote object identity.
   Activation requires a quiescent runtime and recoverable state checkpoint.
   It is never an automatic consequence of fetching, staging or test success.

This document starts no polling, scheduler, fetch, live application or push.
Future unattended jobs require an explicit policy with limits and notifications;
the component must remain useful for a single manually requested revision.

## Automation policy and review boundaries

Routine automation may fetch the chosen source, compute inventories, create an
isolated candidate, apply conflict-free mechanical edits, build and test within
declared resource limits, and prepare reviewable commits. Policy must pin allowed
repositories, target refs, operation permissions, budgets and artifact retention.
Compatible changes are eligible only when mapped semantics, version constraints
and required gates all pass; no conflict is insufficient evidence by itself.

Flag unknown semantic impact, missing lineage, unresolved merge, security/policy
changes, protocol removals, event ordering, auth/destination changes, destructive
state migration, license changes, unsupported platforms and unverifiable artifacts.
Ambiguous behavior needs an explicit recorded decision before activation. Routine
choices with a clear recommendation can be resolved autonomously under project
authorization. Purchases, destructive operations or broader publication require
their actual authorization, not a generic updater permission prompt.

## Coordinated compatibility

Record a release tuple: upstream revision + host build/tree + manifest API + each
component contract/version/package digest + SDK version + persistent state/schema
version. Package semantic versions alone do not express wire or state compatibility.
Keep host negotiation, generated protocol/schema artifacts, native packages, custom
SDK templates and GUI/app-server compatibility in the same review.

Compatible independently installed plugins must run against the unchanged host.
An upstream engine update can require building a new host; that is distinct from
the no-host-rebuild plugin installation requirement. Reject incompatible packages
before invocation. Support a documented overlap window or coordinated migration
for breaking contracts; never silently reinterpret old payloads or select another
backend. State upgrade and downgrade support must be explicit per owner.

## Required evidence before a release

- Source impact coverage: every changed upstream path classified, unknowns
  resolved or explicitly release-blocking, lineage and license notices updated.
- Contract/security tests: version mismatch, invalid frames, provider distrust,
  credential redaction, managed restrictions, destination policy, owner changes,
  bounded queues, cancellation, graceful/forced shutdown and no backend fallback.
- Native regressions for affected crates plus applicable broader suites, with
  truthful resource/platform exclusions. Build success is not runtime evidence.
- External package proof: export/build outside source, remove source access,
  install native and custom replacements, verify real behavior, remove/deselect
  and verify restoration; compare host hashes across installation.
- Upstream compatibility proof: retain compatible, already-installed custom
  replacement and additive packages across the candidate host update. Verify
  their package digests are unchanged and exercise their actual behavior without
  rebuilding them. Rebuilding every plugin with the candidate cannot establish
  this compatibility gate.
- Actual CLI/engine and GUI: streaming, tool execution, approvals, interruption,
  persistence, cold resume, and manager Launch Ctrl+C cleanup through gateway,
  app-server and storage. Recheck existing UI features each development cycle.
- Use the requested in-app Browser when accessible; record the exact limitation
  and real fallback browser evidence when unavailable. Deterministic provider
  fixtures do not establish live-model provider access or remote viewer reachability.

## Rollback and bootstrap recovery

Before activation retain the old source commit, upstream object references,
verified host/package/SDK artifacts and hashes, configuration/selections, state
schema manifest, quiescent snapshots and complete migration journal. Preserve
untracked and WIP source separately. Credentials require protected private storage
and must never enter a public source bundle or verification report.

Drain the actual launcher and owning services before a consistent state snapshot.
If shutdown times out, mark durability uncertain and use a backend-supported
backup/recovery procedure; copying open database files is not a proven backup.
For irreversible migrations keep a restorable pre-upgrade snapshot. Do not point
an old binary at a new incompatible schema and call it rollback.

The independently runnable bootstrap command must verify artifact digests, report
active ownership, restore a compatible host/package/configuration/state set,
and run a recovery smoke check without loading the failed candidate harness.
It must not depend on that candidate's component registry or GUI to recover.
Retain failed-candidate source/logs for diagnosis; never reset the user's checkout.

Distinguish cloud-local checkpoints from external backups. The recovered archive
documented in RECOVERY is local to this VM; published source on GitHub protects
only included source. Before updater v1 release, prove retrieval of required
bootstrap/rollback artifacts from outside the candidate environment. Published
source alone does not preserve host binaries, private state or credentials.

## Maintenance v1 completion and ongoing lineage

Completion requires the separately built/installed maintenance package to integrate
**at least one real later upstream revision** into an isolated candidate, preserve
native/custom behavior, and produce a reviewable provenance-linked checkpoint.
Also deliberately exercise a breaking/incompatible candidate: block activation
when detected, and test rollback after a controlled failed activation using the
external bootstrap with the candidate host unavailable. Prove recovered persisted
sessions and working CLI/GUI; a fabricated diff or reset-only test is insufficient.

Every future extraction must update lineage with native source paths/symbols,
new API/adapter/implementation locations, state/dependency/lifecycle owners,
contract versions, evidence and remaining gaps. Refresh the full path inventory
at each checkpoint, preserving prior snapshots. Track new upstream revisions,
renames and deletions explicitly; mark semantic equivalence unknown until reviewed
and tested. This initial map is a baseline inventory, not a completed updater or
proof that all Codex functionality is already independently replaceable.

The [TUI Preparing failure map](upstream/p02b-preparing-tui-consumer-lineage.json)
retains both exact failed regression snapshots, the passing controlled lifecycle cases,
and the later public Stop fix as explicitly outside those tested snapshots. Failed
acceptance is preserved for maintenance analysis; it is not an accepted host version.

The [public Stop map](upstream/p02b-public-stop-lineage.json) records exact upstream
paths/symbols, prior adapter ownership, the visibility-only compile correction and
the combined6095-pass library scope. Upstream session-ID serialization, our earlier
connection-scoped queue and this new receive-order cancellation are distinct
customizations. Full host, installed runtime and UI acceptance remain separate gates.

The [new fullCLI/installed evidence](verification/2026-10-01/P02B_PREPARING_NEWHOST_EVIDENCE.md)
binds all18 public Stop/TUI paths to the formatted CLI artifact and four actual installed-host gates.
WIP source checkpoint3b8a889 extends the prior9bd3bc30 tree without changing the upstream pin.
The final aggregate is promoted to main `c28a1c33a856a987316f4b97b4c488d68055fffc`.
Successful plugin interoperability and cancellation here do not constitute later-upstream integration;
P18U's real upstream candidate and failed-update recovery acceptance remain required.

The [TUI integration evidence](verification/2026-10-01/P02B_PREPARING_TUI_INTEGRATION_EVIDENCE.md),
[normal GUI/storage evidence](verification/2026-10-01/P02B_PREPARING_STORAGE_GUI_EVIDENCE.md) and
[held GUI evidence](verification/2026-10-01/P02B_PREPARING_GUI_HELD_EVIDENCE.md) extend that same
source/binary tuple. Exact test-only runner sources are preserved in the reviewed
[source-fixture archive](verification/2026-10-01/fixtures/preparing-runner-sources.tar.gz) with
[member hashes](verification/2026-10-01/p02b-preparing-source-fixtures.json). Historical stage
headers and path-bound checks remain unchanged. This supplies reviewable customization evidence
for future upstream integration; it does not implement or pass the required updater acceptance.

The [P03 shared-wire map](upstream/p03-wire-slice1-lineage.json) records a private
refactor of our custom transport, whose paths are absent at the original upstream
pin. It maps the former symbols to their new modules and preserves tested source
hashes separately from the reviewed formatting transition. This is component
support infrastructure, not another extracted native subsystem or an upstream
revision integration. New executable and installed-package gates remain distinct
from the previous full-host and GUI evidence. The new standalone CLI subsequently
passed [24 installed-package commands](verification/2026-10-01/P03_WIRE_SLICE1_RUNTIME_EVIDENCE.md)
against the preserved worker04 package. Exact executed test-helper sources are
preserved in a [reviewed fixture archive](verification/2026-10-01/p03-wire-cli-source-fixtures.json).
This remains same-upstream compatibility evidence, not P18U acceptance.

The [service-declaration map](upstream/p03-service-declarations-lineage.json) binds twelve
custom API/loader/fixture paths and the generated lockfile edge. The successful 177-test
snapshot and three-file formatting transition are separate from the eight-command manager
installation gate. The [declaration contract](component-sdk/SERVICE_REQUIREMENTS_V1.md)
records the Rust source-literal change and unchanged plain JSON behavior. The broader
[broker design](P03_LEAF_BROKER_DESIGN.md) and its [source snapshot](upstream/p03-broker-design-bindings.json)
are intentionally future-facing: no grant or runtime broker activation is inferred from them.
