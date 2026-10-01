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
