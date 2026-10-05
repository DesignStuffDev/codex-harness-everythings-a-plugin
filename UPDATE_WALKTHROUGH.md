# Required guided upstream update and recovery walkthrough

Required P18U/P19 product contract, grounded in the current source; **not yet
implemented or accepted behavior**. This extends the canonical roadmap rather than
replacing the verified0.3 source-transformation scope or the native capacity gate.
The source audit and reviewed design are preserved under R/p18u-guided-update-design-20261005-01;
its sealed manifest is `d3e511836de2f593da40b3855b625468659a0a64715b36303efff8b5c1e6de60`.

The required [A/B release-generation contract](LIVE_UPDATE_GENERATIONS.md) specifies
resource isolation, state coexistence, session pinning and route/rollback semantics.
It replaces the earlier global-stopped-writer default while keeping consistent
provider barriers and controlled destructive restoration requirements.

## Existing boundaries and the missing prerequisite

The separately installed desktop is `presentation:desktop` v1. Its gateway only
forwards nine thread/turn/history/search App Server methods. `Bridge.close` drains
one App Server with207s graceful/209s total deadlines and preserves uncertain cleanup;
it does not fence all writers or snapshot multiple stores. `plugin.launch` exits
when the App Server reader ends, and `Gateway.close` also closes HTTP. Consequently
this presentation cannot currently remain an update/recovery controller while its
own host stops. Its SSE replay and `/status` are transient, not an update journal.

The maintenance source currently declares bounded `tool:upstream_impact_review`,
`tool:upstream_source_capsule` and0.3 `tool:upstream_candidate_overlay` contracts;
root tracks the separate0.3 runtime/checkpoint outcome. They inspect chosen local
objects, retain exact sparse inputs and prepare
sparse outputs. They do not fetch a chosen revision, assemble/build a complete
candidate, snapshot live stores, activate or restore. Current component installation
upgrades by non-atomic remove/install, retaining old objects/state; it is not a
distribution transaction. Native manual rollout migration is a supervised migration
subcontract, not a full multi-store backup or reversible schema migration service.

Reuse HTTP bearer/Host/Origin/CSP protections, safe text rendering, bounded SSE,
request cancellation/ownership and current provenance primitives. Add a separately
installable maintenance owner, a dedicated typed gateway dispatcher, and a GUI
maintenance mode whose lifetime survives deliberate App Server disconnection. The
same external recovery owner must offer an authenticated minimal recovery view/CLI
when the candidate host or ordinary desktop plugin cannot load. It uses recorded
recovery artifacts, not candidate App Server APIs. Do not broaden `/rpc` to arbitrary
methods, executables, paths or shell strings.

The external bootstrap remains a minimal justified recovery exception: validate
accepted manifests/journals, acquire the installation lock, reconcile/switch a known
deployment and start the retained recovery presentation. Planning, dependency rules,
porting, builds, migration policy and normal GUI remain replaceable components.
Do not embed a second harness in bootstrap. The current bearer authenticates a local
session, not an account or authorization to change any installation on the machine.

## Browser walkthrough and authoritative states

| Screen / operation | Data and action shown | Required transition condition |
|---|---|---|
| Updates / choose revision | Installed composition and exact upstream base; explicit repository/ref entry resolves once to a commit/tree; fetch size and network result | Fetch remains isolated; unresolved target is not a plan; no polling |
| Changes and provenance | Before/after versions, affected packages/contracts/features, original→extracted path/symbol mapping, exact custom changes, conflict and mapping gaps | Every change has ownership or a visible unresolved blocker; clean merge is not compatibility |
| Choose changes | Include/exclude components, mapped features or coherent groups; remembered exclusions, dependency reasons and required atomic groups | Invalid/unknown/cyclic/incompatible selection blocked; dependencies are proposed, never silently included |
| Allowed changes | Per-plugin implementation/config/contracts/dependencies/schema scopes; show exact scope expansion, if any | Explicit reviewed plan revision; no worker can widen its own policy |
| Storage and recovery | A+B, build/link/migration workspace, recovery/rollback and ongoing-write peaks across disk/RAM; local and external restore verification | Measured reserves and capped background work; insufficient capacity leaves A usable |
| Stage candidate | Immutable pinned input, chosen plan, transformed diff, unresolved conflicts, final scope audit | A remains responsive; B and test side effects are isolated; no live writes or stale snapshot promotion |
| Build and verify | Separate build and test actions; required gates with exact source/binary/package identities; passed/failed/skipped/unavailable/stale states | Only matching current identities satisfy mandatory gates; skipped/failed/unknown is never green |
| Prepare activation | Live A sessions and pins; provider compatibility, catch-up and ownership; healthy B and verified recovery | Lossless coexistence/handoff proved before commitment; otherwise B stays pending and A usable |
| Activate and check | Durable new-session route transaction and health/state results; A draining status | Healthy B receives new sessions; A sessions remain pinned, responsive and fenced against duplicate ownership |
| Complete / recover | Accepted composition, old recovery point, migration outcomes and conversation availability | Failed/uncertain steps remain visible with recovery action; browser reconnect does not replay mutations |
| Later rollback | Prior software versus snapshot; newer writes, active B sessions and rescue outcomes | Verified rescue first; compatible route rollback preserves newest state and session pins; explicit safe drain for state restoration |

The browser saves no authoritative policy or job state in local storage. Reopening
the page reads the server's durable state revision and resumes observation. Each
mutating button carries an idempotent command ID, exact plan/policy digest and expected
revision; an acknowledgement means durable command admission, not completion. Duplicate
or lost replies reconcile against the journal. Cancelling an accepted stage requests
stop and exposes cleanup-pending until owned work settles. Browser disconnect alone
does not authorize cancellation, activation or restoration.

## Proposed versioned contracts and ownership

These are new contracts, not invented App Server methods. P18U can prototype the
read-only/planning presentation through existing tool-v1 adapters; transactional
mutation must wait for its durable maintenance owner and required host bindings.

| Contract | Essential input / output and owner |
|---|---|
| `maintenance.capabilities` v1 | Installation ID, host/package digests, current composition/schema epoch, supported operations/versions and explicit unavailable functions; discovered maintenance provider |
| `maintenance.selection` v1 | Versioned profile of stable component/feature/group IDs, explicit inclusions/exclusions, dependency graph digest and user scope rules → validated closure or exact unsatisfied reasons; maintenance policy owner |
| `maintenance.plan` v1 | Exact upstream/base/custom revisions, composition digest, mapping index digest and selection revision → immutable plan ID, per-change provenance/conflicts, version/migration plan, budgets and mandatory gates |
| `maintenance.job` v1 | Idempotent command ID, job/plan/policy IDs, expected journal revision, one stage action → durable admission, job state, typed failure/cancellation and monotonic event cursor; single job owner |
| `maintenance.scope_audit` v1 | Final source/build/config/schema/package diff and policy digest → every change classified and allowed or denied; independent trusted validation before building and again before activation |
| `recovery.snapshot` v1 | Coordinated installation/state snapshot epoch, catch-up cursors and complete provider inventory → immutable recovery manifest, artifacts, consistency/integrity/authenticity status and independent restore-test receipt |
| `deployment.transaction` v1 | Candidate and prior composition, exact gate set, recovery ID, state-handoff/ownership tokens and explicit new-session routing command → crash-reconcilable journal, health result or failed-activation recovery |
| `recovery.rollback` v1 | Target recovery point, current writer/state epoch and desired software/state action → rescue snapshot, data compatibility/reverse-migration/export plan and per-data-set outcome; external owner executes without candidate host |

All job replies include contract version, installation/job ID, durable state revision,
source/package/policy identities, diagnostics and cleanup certainty. Events are bounded
and resumable; journal snapshots are authoritative after gaps. Contract/API majors,
unsupported schemas and unknown required capabilities fail closed. Common errors:
stale plan/policy, invalid selection, unresolved mapping/conflict, scope denied,
capacity unavailable, writer still active, snapshot incomplete, incompatible state,
gate missing/stale, recovery unavailable and durability uncertain. Errors never
contain tokens, environment dumps or credential file contents.

## Selection and modification-scope enforcement

Use a versioned provenance/dependency graph. A selectable feature must map to exact
source changes, resulting artifacts and owning components. Still-coupled/shared
changes form explicitly indivisible groups until a sound finer boundary exists.
Excluding an upstream change keeps the installed behavior; it does not uninstall its
component. A partial selection records the exact applied/excluded upstream change
set and must not claim the whole upstream revision was integrated. A persistent
exclusion follows a stable identity only while mappings remain valid; renamed,
ambiguous or newly dependent changes require visible revalidation.

Default new plan scope is the selected component's mapped implementation paths;
contract, dependency, persistent-schema and config-schema changes require explicit
scope entries. Configuration values are separately constrained by key/schema and
secret classification. Scope rules bind component identity, normalized path or
schema/symbol anchor, operation, before-object digest and policy version. Every
plan revision invalidates earlier candidate/gate approval.

The evaluator checks the **actual final diff**, including generated code, lockfiles,
transitive dependencies, mode changes, symlinks/renames, migrations and build outputs.
Shared dependencies/contracts require all impacted ownership groups and compatible
versions; dependency closure never expands permission. Unknown classification blocks.
Workers write only owned staging directories; production permissions/capability
enforcement must prevent out-of-scope writes. Current same-user plugin execution is
not such a sandbox. Independent final-diff checks are necessary but cannot alone
claim protection from an arbitrary unrestricted plugin; P10/P11 permission/sandbox
and P18 installation enforcement are prerequisites for that security guarantee.

## Verified recovery sets, activation and rollback after newer writes

A complete recovery manifest binds exact host, SDK/runtime dependencies needed for
offline recovery, plugin bytes/versions/selections/config, user custom source/patches
and untracked source, upstream/mapping/policy identity, every durable domain/schema,
conversation/rollout metadata, attachments and references, auxiliary DB/WAL state,
migration journals and external bootstrap. Separately preserved WIP is included or
explicitly referenced as independently durable, never silently treated as accepted.
Provider-owned state outside the default home must be inventoried. Unknown providers
or unaccounted durable domains block a claim of full recovery.

The normal update uses immutable A/B release generations. A stays responsive while
B builds/tests against consistent isolated state. Use provider-supported snapshot
barriers, bounded catch-up journals/watermarks and fenced single-owner sessions/jobs.
Never replace authoritative current data with a stale candidate clone. Additive
schema changes must support both generations' readers and writers. Unsupported or
incompatible overlap leaves B pending and A usable; a separately reviewed downtime
fallback does not satisfy live A/B acceptance. Snapshot consistency and destructive
state restoration still require appropriate provider barriers, not arbitrary copies
of active databases or uncoordinated domain backups.

Keep secrets private: authenticated recovery endpoints, restrictive local files,
redacted public manifests and secret values excluded from logs/browser diffs. Record
credential references and restore feasibility. Where secret material must travel,
use authenticated encryption via an approved key provider and independently verify
key availability; absent key/export support is a blocker, never a plaintext fallback
or a claim that a hash protects confidentiality. In-workspace artifacts are labeled
as such; separately verify externally durable recovery where required.

The stable launcher/router owns a durable installation route and session-generation
pins. Domain update policy and normal presentation remain replaceable components;
independent recovery must work without B or its updater. Health-check B before a
journaled compare-and-switch of the new-session route. Keep A sessions on A through
streaming, tools, approvals, cancellation and persistence; drain only their accepted
work, without turning a deadline into a false clean-shutdown result. Reconnect/resume
uses recorded ownership, not merely the current default route.

Persist `plan_bound → B_staging → recovery_verified → B_health_verified →
state_handoff_ready → route_intent → route_committed → A_draining → accepted`.
Bind every transition to exact compositions, plan/gates, state/schema epochs,
expected route revision, owner leases and recovery IDs. Reconcile uncertain commits
on restart; do not replay mutations blindly. If B fails before commitment, A stays
selected. Later failure follows the recorded compatible rollback policy while
preserving all post-switch writes. Retain the previous usable release locally plus
an independent recovery entrypoint and externally retrievable recovery material.

For later rollback **first rescue current data**, even if the requested target is
older. Show two distinct operations:

- **Software rollback:** inventory active B sessions and first verify a rescue copy.
  When compatible, route new sessions to retained A while B sessions stay pinned or
  explicitly drain; never force a running session into another generation. Old
  host/plugins use newest state only when verified read/
  write schema compatibility permits it; otherwise reverse-migrate a rescued clone
  with tested adapters, keeping both originals.
- **Snapshot restore:** old compatible software plus old state. Newer conversations,
  attachments and metadata remain in a verified rescue set and, where incompatible,
  a tested neutral export/readable recovery view. The user sees which conversations
  are retained in active state, reverse-migrated, or available for export/read-only
  recovery. Never overwrite or discard the only post-update copy.

If neither compatibility, reverse migration nor reliable preservation/export can be
established, stop the destructive state transition. A rescued archive alone does not
prove conversations remain readable. Prove each supported format with the preserved
reader/exporter independent of the failed candidate host. GC must retain all artifacts
referenced by recovery journals/rescue sets until an explicit retention operation.

## Dependency-ordered implementation and acceptance

1. Checkpoint0.3 sparse transformation honestly. Add selection/scope/provenance and
   resource-plan schemas plus dependency-validation/final-diff fixtures. Preserve
   every original failure and exact source/package/test binding.
2. Add durable maintenance job ownership and scoped staging; package/update SDK
   contracts. Add authenticated read-only and planning UI with persistent exclusions,
   valid/invalid selection, custom-conflict and prohibited-scope walkthroughs.
3. Assemble a complete chosen candidate and coordinated version set; produce actual
   isolated build, installed-custom-plugin, headless and GUI gate receipts. Current
   native capacity remains a blocker for those builds, not an excuse to mark them done.
4. Implement complete provider inventory, writer fencing, consistent backup/restore
   and offline recovery bundle. Test missing domain/key, corruption, insufficient
   capacity and restore to fresh isolated state before enabling activation controls.
5. Decouple desktop maintenance lifetime; implement stable routing/session pins,
   provider snapshots/catch-up, fenced shared-state compatibility and A/B coexistence.
   Health B before crash-recoverable new-session cutover; prove active A continuity.
   Exercise interrupted migration/activation and host-unavailable external recovery.
6. Implement later rollback after successful new conversations/attachments. Verify
   rescue, compatible software rollback, reverse migration where supported and
   old-snapshot/new-data export outcomes without lost data.
7. P19: run the complete interactive browser walkthrough for a real later pinned
   upstream revision, genuine custom conflict, invalid dependency selection, prohibited
   scope, consistent restore, interrupted migration, failed activation, host unavailable
   and manual rollback after new writes. Include A streaming/writes during B staging,
   capacity refusal with A usable, new-session routing, old-session continuity,
   duplicate-job prevention and rollback with active B sessions. Regress streaming/tools/approvals/Stop,
   storage/migration/recovery, plugin install/remove/replacement and actual Launch
   shutdown each cycle. Use in-app Browser when available; record real Chromium
   fallback separately and leave unavailable mandated checks pending.

No polling, unattended live deployment, active-session hot replacement, new VM or
remote viewer prerequisite. A browser walkthrough is required product behavior;
the optional remote desktop relay is only development access.
