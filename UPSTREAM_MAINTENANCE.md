# Upstream maintenance and recovery contract

Status: **required component; offline planning support exists, installed updater and activation do not**.
This workflow preserves a maintainable OpenAI Codex fork as native services become
independently installable. It does not count as extraction of an existing engine
subsystem. It must ship as its own replaceable package, with an external recovery
entrypoint that still works if the candidate harness cannot start.

The initial [offline impact planner](upstream/UPSTREAM_IMPACT_README.md) compares exact local
revision trees and joins changed paths to the existing historical lineage index. It retains
missing mappings, ambiguous owners and semantic/security/migration uncertainty. Seventeen
planner fixtures and 22 existing lineage fixtures passed; a real local-object call at the same
upstream revision returned the expected unresolved exit 2. [Evidence](verification/2026-10-02/P18U_OFFLINE_IMPACT_EVIDENCE.json).
This support tool performs no fetch, candidate preparation, install, migration or activation.
It does not satisfy later-upstream integration, independently installed maintenance or rollback
acceptance. Current normalized provenance remains incomplete; no periodic polling was enabled.

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
remain unfinished. A normalized cross-map ownership/completeness checker remains required current
P00M provenance work; automated contract/security/migration impact decisions remain
P18U implementation work.
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

## Current provenance closure

Current source mapping is an immediate P00M obligation; the separately installable
updater and its integration/recovery demonstrations remain P18U implementation.
The [checkpoint index and read-only validator](upstream/CHECKPOINT_LINEAGE_README.md)
now provide concrete metadata coverage for published `d04d5a7`: 1,092 changed paths,
28 historical maps and 27 relationships. The original 13-fixture evidence is preserved;
the [provider-schema supplement](verification/2026-10-01/P00M_PROVIDER_SCHEMA_EVIDENCE.md)
passes 22 fixtures and recognizes all 28 map shapes. It reports zero invalid metadata
and 1,167 unresolved findings: semantic closure, historical anchors/evidence, unavailable
parent/tested-source bindings and updater/release gates. Recognizing a schema does not
verify those claims.
This implements the inventory/checking foundation; it does not complete the semantic
closure work listed below or certify compatibility for an upstream update.
The original upstream pin is still `d42056091aded7feb1d88ac7e83972108b2aa478`.
Later project checkpoints, vendored dependency updates and installed worker
compatibility tests do not advance that official Codex revision.

The existing records establish these bounded starting points:

| Current responsibility | Original native anchors and retained mapping | Interpretation |
| --- | --- | --- |
| C02 native thread storage | `thread-store/src/store.rs::ThreadStore`, `thread-store/src/local/mod.rs::LocalThreadStore`, `thread-store/src/live_thread.rs::LiveThread` in [initial lineage](upstream/lineage.json) | Native implementation externalized for the recorded operations; auxiliary state and remaining maintenance stay separate. |
| C02 manual rollout migration | `cli/src/migrate_rollouts.rs::run`, `thread-store/src/local/rollout_migration.rs::migrate_rollouts_with_progress` in [P01 lineage](upstream/p01-migration-lineage.json) | Existing native algorithm plus custom selected dispatch, retained lifecycle and capability versioning. |
| C04 inline attachments | `attachment-store/src/lib.rs::{AttachmentStore,InlineAttachmentStore}` in [initial lineage](upstream/lineage.json) | Native byte-preserving upload and `NotFound` resolution only; manager-level evidence does not establish every production caller or remote blob storage. |
| C18 native search | `file-search/src/lib.rs::{FileSearchSession,create_session,walker_worker,matcher_worker}` in [selected-search lineage](upstream/p02b-selected-search-lineage.json), extended by [App Server](upstream/p02b-app-server-lineage.json), [TUI](upstream/p02b-tui-consumer-lineage.json) and Preparing maps | Actual traversal/matcher replacement with consumer-specific evidence; private rollout lookup and the rest of C18 remain separate. |
| C00 component and broker support | [Shared wire](upstream/p03-wire-slice1-lineage.json), [declarations](upstream/p03-service-declarations-lineage.json), [limits](upstream/p03-broker-limits-lineage.json), [grants](upstream/p03-broker-grants-lineage.json), [offer/ack](upstream/p03-broker-offer-lineage.json) | Intentional project contracts/refactors; these files have no invented original native implementation and do not activate a broker. |
| C05/C06 inference/auth seams and C19 GUI | Boundary entries in [initial lineage](upstream/lineage.json) | Adapters or additive GUI, not extraction of native provider/authentication implementations or official desktop source. |

Paths in that table are relative to `codex-rs/`. The linked records retain the
original revision/blob identities and destination symbols; the table is an index,
not a new validation of historical Git objects or a complete symbol/call graph.
The provider-owned endpoint capability has separate
[native test evidence](verification/2026-10-01/P03_PROVIDER_ENDPOINT_EVIDENCE.md) and
[original/current symbol mapping](upstream/p03-provider-endpoint-lineage.json).
Its 160 passing tests establish the compiled native ownership prerequisite only;
exposing it is not catalog extraction. The subsequent
[full-host regression](verification/2026-10-01/P03_PROVIDER_FULL_HOST_EVIDENCE.md)
separately binds the compiled host and existing installed storage/search/GUI behavior
to the exact published source; no later official upstream revision was integrated.

The subsequent native workspace-policy change has its own
[original/current boundary map](upstream/p03-native-policy-lineage.json) and
[scoped test evidence](verification/2026-10-01/P03_NATIVE_POLICY_EVIDENCE.md): 256 login
passes and one targeted App Server pass, with 425 App Server tests filtered out.
Its source chain includes the original failed builds, unchanged-source retries,
three argument comments, unchanged lint and mechanical formatting. It remains a
compiled native prerequisite; the earlier d04 full-host proof does not cover it.
The frozen d04 ownership index is not silently advanced by adding this map.

The subsequent [reload/acquisition map](upstream/p03-native-reload-policy-lineage.json)
binds seven Rust paths to the same pinned upstream manager declarations and
project-private reload/acquisition helpers. Its [scoped evidence](verification/2026-10-01/P03_NATIVE_RELOAD_POLICY_EVIDENCE.md)
separates the initial 266 passes, intentional pre-fix 2-pass/3-fail run, and corrected
271 passes with unchanged regression tests. Final scoped lint passed unchanged and formatting was mechanically reviewed.
This advances native policy-checked publication and point-in-time acquisition;
it does not establish independent auth/catalog extraction, later dispatch or
persistence fencing, full-host proof, complete semantic provenance closure or an
upstream revision advance. Historical maps and the frozen d04 index remain unchanged.

The remaining current-provenance work is concrete:

1. Complete semantic ownership/impact coverage in the existing checkpoint-scoped
   index, joining every retained lineage snapshot, accepted boundary and full
   changed-path inventory. Historical
   `current_blob` fields remain bound to their named checkpoints. Add explicit
   supersedes/extends relations instead of relabeling old WIP or failed gates.
2. Record semantic origin separately from the diff status `added`: native code
   moved or copied into a new path still needs its original repository, immutable
   revision, path, blob and symbol/range. Mark project-authored code as such with
   its introducing checkpoint and rationale; record foreign repository/revision,
   patch and license for vendored code. A null same-path upstream blob alone cannot
   distinguish these cases. Label each relation as source derivation, adaptation,
   intentional custom code or interface/dependency use. In particular, App Server
   `run_main` is a GUI dependency and `ExtensionRegistryBuilder` an infrastructure
   integration anchor; neither makes the new GUI or component runtime copied
   upstream code. Lines are locators within a hash-bound file.
3. For each maintained boundary link original and destination symbols, its API,
   adapters and selected callers, persistent state/dependency/lifecycle owners,
   contract/capability versions, intentional behavior changes and acceptance scope.
   Preserve semantic unknowns and exclusions rather than inferring equivalence
   from a matching name, directory move or successful build.
4. Extend the existing read-only schema, identity, anchor and path-coverage checker
   to unverified historical fields/anchors, links/licenses and the complete
   source-to-test-to-export-to-package-to-host evidence chain. Contradictory evidence
   is invalid; missing or unsupported entries remain unresolved. Local-only
   artifacts need retained digest-bound summaries and an explicit durability
   limitation. Recognized metadata alone does not validate the evidence chain.
5. Publish the normalized index and its actual validation report with a pinned
   checkpoint before claiming complete current provenance. Resolve affected
   mapping gaps before a later-upstream candidate can pass impact review. Automated
   semantic porting, dependency impact decisions and the updater remain separate
   implementation work; source metadata alone cannot decide compatibility.

The initial map explicitly labels all 617 path rows `not_individually_reviewed`
and maps selected boundary anchors. This is an honest historical limitation, not
617 proven defects or a reason to discard its scoped runtime evidence. Later
maps add detail but use different schemas; the selected-search record already
identifies normalized ownership/completeness as missing. Current mapping closure
must therefore have its own result, rather than inheriting a runtime pass.

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
version. Include optional capability versions, state readers/writers, supported
upgrade/downgrade paths and the tested old/new overlap matrix. For example, P01's
storage wire 2, optional manual migration capability 1 and native package 0.2.0
are separate version dimensions; search worker 0.2.0 still uses wire contract 1.
Package semantic versions alone do not express wire or state compatibility.
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
- State migration when persistent schemas change: upgrade preserved realistic
  state and verify sessions, configuration and custom-component selection. Inject
  interruption/failure after migration changes state, then prove a supported
  downgrade or restore of a pre-upgrade snapshot with its compatible host/package/
  schema tuple. Verify recovered behavior; a failure before mutation does not prove
  migration recovery. For unchanged schemas, record the no-migration compatibility
  basis instead of claiming an unexercised migration gate.
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

The [P03 broker-limit lineage](upstream/p03-broker-limits-lineage.json) maps additive checked
configuration and its tests to the current custom API. It changes no upstream native symbol
and implements neither accounting nor the updater. The separate negotiation design remains
unimplemented; future update tooling must preserve these compatibility and authority contracts.

The [grant API lineage](upstream/p03-broker-grants-lineage.json) records checked opaque
handle representations and canonical service/operation descriptions. This is additive custom
contract code, with no native upstream symbol movement or authority issued by construction.

The [offer/acknowledgement lineage](upstream/p03-broker-offer-lineage.json) binds exact
canonical validation and requirement matching to original staged and root-integrated source.
Its twenty API cases do not activate runtime negotiation or satisfy native extraction/updater gates.

## Native auth checkpoint: full-host regression

The source checkpoint `e289b6b5` (published with scoped evidence at `f9e922c3`)
also passed a fresh full CLI build and unchanged installed-package acceptance:
13 storage commands, 10 migration commands and two cold GUI cycles. Both GUI runs
terminated normally after first SIGINT to the actual component manager during an
active turn, with tracked processes absent and no forced cleanup.
[Runtime evidence and reviewed screenshots](verification/2026-10-01/P03_NATIVE_AUTH_FULL_HOST_EVIDENCE.md)
bind the exact source, executable and independently built packages. This uses
Chromium/Playwright and deterministic inference; in-app Browser and live-provider
proof remain unavailable. Existing native extraction coverage is unchanged, and
credential/source lifetime, broker/catalog activation and P18U upstream integration/
rollback remain required work. The subsequent cache-revision checkpoint has its
own scoped evidence below; it is excluded from this earlier runtime checkpoint.

The [credential-cache map](upstream/p03-native-cache-revision-lineage.json) binds
six final source paths at `18140083910d19c86e9cadaef0990db9aa652f6e` to native
manager load/cache adaptation, project-authored revision identity and connected
regression tests. [Evidence](verification/2026-10-01/P03_CACHE_REVISION_EVIDENCE.md)
separates the original failed source, corrected 385-test source and reviewed
formatting transition. No dependency, public contract, persisted schema or
official upstream revision changes. This map does not advance the frozen d04
index, close its semantic findings, implement an updater or satisfy P18U.

## Current cache-revision host and GUI regression

[The runtime supplement](verification/2026-10-01/P03_CACHE_REVISION_FULL_HOST_EVIDENCE.md)
binds source `18140083910d19c86e9cadaef0990db9aa652f6e` and exact CLI
`876c826f76d574d9fee62011166b1ae3f4b15f69103f361aedb4e44cef08de71`.
The full build, installed storage, migration and two cold GUI cycles passed with
unchanged independently built packages. Both actual manager first-SIGINT shutdowns
exited 0 in about 0.27 seconds with tracked processes absent and no forced cleanup.
The ENOSPC attempt and unchanged fixture-isolation rejection remain failed evidence;
a fresh workspace migration fixture supported the successful GUI run. Assertions
were not weakened. Some adopted storage/migration descendants exited nonzero;
strict drains passed, but their executable/cause attribution is unknown.

This proves current same-upstream runtime regression for the recorded behaviors,
using Chromium/Playwright and deterministic inference. It does not increase native
extraction coverage, exercise a new attachment roundtrip, establish installed auth
or catalog selection, or satisfy P18U's real later-upstream integration and rollback.
In-app Browser and live-provider acceptance remain pending. Upstream stays pinned.

## Native source and credential ownership checkpoint

Source `99e6802fd478e55686559aefdd6ae79177a45397` pairs the active external auth
provider with credentials and failure metadata under one native owner. Failed
credential preparation preserves the previous in-memory pair; clear and provider
retirement observe the published state, with provider destruction outside locks.
The [scoped evidence](verification/2026-10-01/P03_SOURCE_OWNER_EVIDENCE.md) retains
four baseline assertion failures and **391 passing login/provider tests**, exact
post-test type-alias/formatting transitions and an unattributed SIGKILL child reap.
The [lineage](upstream/p03-native-source-owner-lineage.json) maps actual pinned
upstream methods separately from private owned-update and persistence helpers.

This is a compiled native ownership prerequisite, not installed authentication or
catalog extraction. Installer ordering, source ABA, load/refresh ownership and
shared persistence remain open. Its fresh full-host build, migration and GUI checks
now pass, but two storage lifecycle checks fail on surviving Git descendants; see
[the runtime supplement](verification/2026-10-01/P03_SOURCE_OWNER_FULL_HOST_EVIDENCE.md).
This source has no overall full-host acceptance. Extraction counts and the upstream
pin do not change. P18U's integration/recovery gates remain
required, with no active polling or live update.

The [native install-order map](upstream/p03-native-install-order-lineage.json) binds source
`65511842d7051b2a1f5cc52917f3ebb5c03be4f3`, tree
`ed564f68f3297cd65b65321d1f41f79b4f9c4031`, to original upstream auth
installation/clear and the project's private source/cache policy owner. Its
[evidence](verification/2026-10-01/P03_INSTALL_ORDER_EVIDENCE.md) distinguishes the unchanged nine red/green
tests, 400 scoped passes and mechanical formatting. It adds no installed component
or later upstream revision. The curated-sync failure shows why successful builds
and merges cannot replace P18U's lifecycle/security/runtime gates.

## Curated Git transport candidate and ownership gate

Source `7ab5dd77b87c2e6bf7040824e67bf6f22af6a073` lives on the separate
`work/p03-curated-sync-lifecycle` development branch; it is **not promoted to main**.
Main source remains the native installer checkpoint655 while documentation records this
candidate's progress. [Evidence](verification/2026-10-01/P03_CURATED_GIT_TRANSPORT_EVIDENCE.md)
preserves a failed link, three real baseline lifecycle failures and two541-pass scoped
runs, including the final explicit child-ownership repair, clean lint and separate
mechanical formatting. [Lineage](upstream/p03-curated-git-transport-lineage.json) binds
upstream primitives, native adaptations and new support without claiming an installed component.

Linux private groups/pipe bounds/direct-child reap are scoped native improvements. HEAD
lookup also enters the existing timeout path on non-Linux, which is untested. Private
groups escape the parent's process-group guard on abrupt host death. The current String
pipeline can still fall back/release protected state on uncertain cleanup, and manager
admission can reopen. Therefore B1a must couple typed outcomes with actual retained
cleanup resources, worker/admission ownership and manager integration **before** B1b
cancellable locks/Git. [Required next contract](verification/2026-10-01/P03_CURATED_B1_OWNERSHIP_REVIEW.md)
supersedes any queue that separates this ownership transition into unsafe intermediate
production changes. All full-host/UI gates and remaining extraction remain open.
P18U remains required and unimplemented; no polling/live update has started.


The [locked-attempt lineage](upstream/p03-curated-locked-attempt-lineage.json) binds
source9a2's private native ownership seam to the exact pinned upstream facade body,
separating unchanged behavior from the added error-path lock assertion and formatter
layout. Its 476-pass scope is not retained-cleanup, installed-component, full-host or
later-upstream proof. The same remaining P18U integration/rollback gates apply.


The [retained native sync owner map](upstream/p03-curated-retained-owner-lineage.json)
binds source `739eb89c537a29f715e79ebf6e6405181fec9744` to upstream curated-sync,
manager admission and inherited PTY custody primitives. It distinguishes original
fallback/policy behavior from custom typed failures, attempt quarantine and retained
worker generations. Its557-test scoped result is not full-host acceptance, a new
installed component, or integration of a later official revision. Preserve this
checkpoint supplement separately from the frozen d04 normalized lineage index;
P00M current-index closure and all P18U integration/rollback gates remain open.

The [shared cancellation map](upstream/p03-curated-shared-control-lineage.json) binds native
source `cb9f7409071e64293801e52b21a2fc1d1a87de6f` to the upstream worker/curated-sync
and PTY call paths and identifies the custom shared control, typed stop and admission rules.
Its572-test scoped result preserves the source739 custody boundary; it does not prove HTTP
teardown, whole-host shutdown, an installed replacement or a later official revision integration.
Keep this supplement separate from the frozen d04 normalized index. P00M semantic/current-index
closure and P18U isolated integration, custom-package compatibility and external rollback gates
remain open.

The [HTTP-await map](upstream/p03-curated-http-await-lineage.json) binds native source
`92212516ad4d12bcf60546ee5f879b983ed44682` to the upstream HTTP fallback/call paths,
existing policy-aware response API and custom stop/budget/teardown behavior. Charset/BOM decoding
is preserved after bounded collection. The original extractor remains unchanged. Its580-test
result is scoped native evidence; ordinary Runtime teardown may exceed the stop deadline.
The pinned [shutdown audit](verification/2026-10-01/P03_CURATED_SHUTDOWN_INTEGRATION_AUDIT.md)
also records process ownership, callbacks and conflicting cleanup budgets for future impact review.
Neither record advances the frozen d04 index or satisfies later-upstream integration, installed
maintenance, migration or rollback acceptance. Keep current-index semantic gaps explicit.

## Current process-final and replacement impact supplement

[The P03 process-final map](upstream/p03-process-final-lineage.json) binds the56-path adopted
candidate at `f25b069357e5f41ee73b4430abadf827d56fc8d6` to exact parent/import objects,
56 literal anchors and two unchanged manager callers. [Static audit evidence](verification/2026-10-02/P03_PROCESS_FINAL_LINEAGE_AUDIT.md)
records direct official upstream verification for31 existing paths and all source-identity checks.
Eight review groups preserve worker/callback custody, embedded versus process-final authority,
shared deadlines, same-home replacement, retained storage obligations and nontransactional
repository/SHA publication as semantic update hazards. Upstream already had dedicated stdio
threads/a local watchdog; the custom change concerns authority and deadlines.

This is an additional source-impact input, not an adapter accepted by the existing normalized
index validator. The frozen d04 index and its unresolved findings remain unchanged. External
Cargo.lock entries are unchanged in this delta, but feature/platform behavior is not certified.
That historical full-host blocker is resolved by the recovered current58 build/runtime checkpoint;
held transport, replacement and MCP gates remain open.
P18U still requires an installed maintenance component, real later-upstream integration, custom
component compatibility, failed-update rollback and independent bootstrap recovery. No polling,
source application, live installation change or deployment is authorized by this metadata itself.

## Recovered-host SDK acceptance delta

[Three custom SDK paths](upstream/p03-recovered-search-reuse-lineage.json) now bind exact f122
preimages and new blobs to explicit search-package reuse checks. No upstream native implementation
changed in this delta. The complete8,921-file tested-source map and executable identities are in
[the current-host evidence](verification/2026-10-02/P03_RECOVERED_HOST_EVIDENCE.md).
The held-write normal/forced GUI checks cover a component transport call before native admission;
retain that limit when adapting event codecs. The separate saved-rollout versus lossless-wire
encoding distinction must survive updates. Response-only interrupted UI projection remains open.
This supplement does not advance the normalized index or implement P18U. Exact later-revision
integration, coordinated versions/migration, custom-plugin/UI preservation and failed-update
recovery remain release requirements.

### Native observation stage1 continuation

[Eight-path lineage](upstream/p03-curated-observation-lineage.json) records copy-only worker and
callback snapshots: finished handles remain distinct from joined ownership. Six new tests and
the existing core-plugins library suite passed542/542, with exact source/strict evidence in
[the stage1 report](verification/2026-10-02/P03_OBSERVATION_STAGE1_EVIDENCE.md). This adds no
installed subsystem. Lint/format, actual same-process replacement and a rebuilt-host held matrix
remain pending. Preserve prior current58 runtime proof separately. The remaining replacement
fixture must explicitly isolate its SQLite root before initialization. P18U impact review must
retain these ownership/observation distinctions; no update integration or rollback has run.

## Featured startup ownership checkpoint — 2026-10-02

[Stage B lineage](upstream/p03-featured-warmup-stage-b-lineage.json) maps all21 native wiring/test paths to exact upstream and prior published bytes. [Stage C lineage](upstream/p03-featured-warmup-stage-c-lineage.json) separately maps three caller-test paths, including the later test-only registration in the same native file. Core Plugins553 and App Server432 tests plus actual in-process caller/replacement cases are [recorded separately](verification/2026-10-02/p03-featured-warmup-stage-c/README.md). The subsequent [current production regression](verification/2026-10-02/p03-featured-warmup-current-production/README.md) binds the rebuilt CLI to source `2ce48a6` and passes storage, four migrations, both GUI modes, slow normal/forced Launch shutdown and the held eight-case matrix. It preserves separate package/build identities, original failed cases, exact adopted child statuses and transport/attachment/consumer-suite limitations. These changes preserve custom lifecycle semantics for later update gates; they neither add an extracted family nor satisfy upstream integration or rollback acceptance.

The subsequent [current attachment caller evidence](verification/2026-10-02/p03-current-attachment/README.md) retains both the legitimate install-order failure and corrected five-group/17-command pass. Its exact fixture lineage preserves the host ambiguity guard, native package reuse, custom file-reference cold resume and positive error fallback. CLI focused retry03 adds three source-bound passing checks after a preserved compilation ENOSPC failure; it is not a new upstream revision or constructor implementation. Future update gates must retain these behavior and error-policy assertions. The normalized current index and P18U real integration/rollback obligations remain open.

The isolated [watchdog/runtime regression lineage](verification/2026-10-02/p03-runtime-drop-watchdog/SOURCE_LINEAGE.json) pins the exact published custom preimage, authored proposal and formatted test source. Four utils-process tests and scoped lint passed; production shutdown code, dependencies and HTTP constructors did not change. Preserve this test when porting the existing custom process-final authority, while retaining the separate production hard-exit and component durability gates. It does not change the upstream pin or satisfy P18U acceptance.


## Native HTTP constructor custody continuation

The [HTTP checkpoint](verification/2026-10-02/p03-http-native-constructor-stage-a/README.md) changes nine source paths: three existing upstream paths and six new custom owner/test modules. Preserve its exact source-to-component lineage and separate test-source transitions when integrating upstream updates. The native route-aware builder now retains constructor and abandoned-result disposal handles, recovers positively joined canceled auxiliary generations on a later runtime, fences publication after the last public pool lease, and preserves first-close deadlines. A separate typed I/O error traversal repair retains TLS classification through nested wrappers; it does not broaden retry or certificate trust.

The package passed139 tests, including ten unchanged custom-CA integration cases, and four configured-CA classification checks passed separately. Earlier failed source/control runs remain evidence. The final lexical test-scope correction passes formatting and scoped lint without warnings; the139 tests retain their original source identity. Process-final aggregate Stage B, shared-cohort queue transfer and consumer/full-host/UI gates remain distinct obligations. The new public Rust error variant requires review for external exhaustive matches; component wire/config/state schemas did not change. This is lifecycle support, not installed HTTP extraction or a new upstream revision. P18U real revision integration, coordinated compatibility/migrations and failure rollback remain required.
