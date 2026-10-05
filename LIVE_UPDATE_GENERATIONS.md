# Required immutable release generations and live update routing

Required P18U/P19 design, **not implemented or runtime-accepted behavior**.
This complements [UPDATE_WALKTHROUGH.md](UPDATE_WALKTHROUGH.md). The
[bounded source audit](upstream/p18u-live-update-source-audit.json) binds the
current interfaces and their missing prerequisites. Source transformation0.3,
remove/install and existing GUI checks do not satisfy this contract.

**Required normal update path:** keep verified deployment A usable while immutable candidate B is prepared, built, migrated in isolation and tested. The walkthrough and recovery entrypoint live outside either replaceable engine. An incompatible, unhealthy, incomplete or resource-blocked B remains pending; A continues working. Existing sessions retain their chosen deployment; this is not hot replacement inside a running session.

A stable launcher/router owns installation identity, deployment generations, immutable composition selection, bounded supervision and the crash-reconcilable route journal. Maintenance planning, staging, domain migrations and normal UI remain separately installed components. The bootstrap can recover the retained launcher/router and prior deployment without loading B or the normal maintenance plugin; updating bootstrap itself needs a separate recoverable procedure. A/B are labels, not permission to overwrite artifacts referenced by active sessions or recovery records.

### Isolation, resources and readiness

- B has immutable host/plugin/configuration artifacts, a private staging workspace, separate process ownership and a private state clone. Validation may not write A's packages, configuration, workspace, databases, attachments, credentials or live external destinations. Test tools/connectors target explicit isolated workspaces/sinks with restricted capabilities; trusted same-user processes alone do not enforce this boundary.
- Reserve the measured whole sequence: running A, full B, retained prior recovery set, local backup and external-transfer/restore staging, build/link overlap, migration temporary files, rollback/rescue capacity and A's ongoing write growth. Account per filesystem plus RAM/tmpfs, CPU, process/FD limits and cleanup deadlines. Tmpfs capacity also consumes RAM. Include uncertainty and a minimum reserve; suspend bounded B work before starving A, retaining checkpoints. No admission if the full sequence cannot fit.
- Share only verified immutable content with reference tracking and read-only enforcement; writable state and build outputs never use hard links to active files. Copy-on-write storage, when supported and proved, needs capacity for divergent writes. GC cannot remove anything referenced by a live deployment/session, recovery journal or retained backup.
- Before cutover, B must pass exact-source/package/plan-bound compatibility, security, custom-plugin, state, headless and GUI health checks in isolation. PID/listener readiness is insufficient. Recheck any gate invalidated by state/schema/configuration changes. Restore the retained A recovery set in isolation and verify required external retrieval; a cloud-local archive or GitHub source push is not a full external recovery set.

### Consistent state with continued A writes

Use provider-supported consistent snapshot APIs and a coordinated domain barrier/watermark for conversations, attachments, indexes, configuration and auxiliary state. A may keep writing while B validates its clone. Preserve a bounded ordered change journal or an equivalent tested catch-up protocol; apply idempotently by identity/revision without lost or duplicate logical writes and verify completeness to an agreed watermark. If a provider cannot supply that contract, show the blocked domain. Copying active SQLite files, independent uncoordinated store snapshots or a content hash alone cannot prove consistency.

A validation clone is disposable test state, never a newer authoritative database. Do not promote it over A's later writes. The normal compatible cutover uses one authoritative provider-owned state set at a coordinated epoch with a proven old/new read-write overlap and fenced per-session/domain ownership. Candidate-only test writes stay isolated. Retained A sessions and admitted B sessions may share compatible services only through that ownership contract; never start independent unrestricted writers over the same database path.

Schema changes during coexistence must be additive/backward-compatible and preserve A's readers/writers. Defer destructive contraction until A sessions, writers and recovery obligations have retired and the relevant gates pass. Establish schema/contract compatibility, catch-up watermarks and writer leases before publishing B's route. A stale process, reconnect, delayed callback or expired lease cannot regain write ownership. New logical writes carry stable IDs/revisions; conflict resolution cannot silently overwrite newer values.

If state cannot support safe overlap, keep B staged and A usable. An explicit planned drain/downtime migration can remain a separately labeled fallback, with its own recovery and authorization requirements; it does not pass the normal live A/B acceptance gate. Global shutdown is not the default workaround for missing snapshot/catch-up/fencing contracts.

### Route transaction and session ownership

Persist the ordered transition: `plan_bound → B_staging → recovery_verified → B_health_verified → state_handoff_ready → route_intent → route_committed → A_draining → accepted`. Each stage binds A/B identities, plan/policy digest, state/schema epoch, recovery IDs, gate receipts, expected route revision and resource lease. Prepare B health before route commitment; perform final state/readiness validation before admitting its first user session. Journal and fsync intent/result with platform-specific crash guarantees, then reconcile actual route/leases on restart instead of blindly replaying a mutation.

Under one installation owner, atomically compare-and-switch the durable **new-session** route to healthy B. Existing A sessions keep their A host/plugin composition and state ownership through streaming, tools, approvals, cancellation and persistence. Requests/replies/events include deployment/session generation and cannot cross routes. Resume/reconnect obeys recorded ownership; transferring a cold session requires an explicit compatible handoff, not guessing the current default. A drains only after all its accepted work and writer leases join; a timeout reports uncertainty rather than killing it to make B appear ready.

The router and recovery presentation remain reachable across A/B failures and browser reconnects. Browser commands remain idempotent, revision-bound and journaled. If B fails before commitment, retain A's route. If commitment succeeds but its reply is lost, reconcile the committed generation. Health failure after commitment invokes the recorded compatible rollback policy and shows a visible failure; it is not permission to erase post-switch data.

### Software rollback after B has accepted work

Before later rollback, inventory active B sessions and all writes since cutover, and create/verify a current rescue set. When A supports the newest state, journal a new-session route switch back to retained A while existing B sessions remain pinned or explicitly drain through their supported shutdown/recovery path. Do not force a running B session into A or count a router change as state restoration. Failed B sessions need an explicit fenced cold recovery procedure and compatible reader, not an implicit retry against A.

If newest state is incompatible, retain B artifacts/data and keep a safe available deployment serving supported work while the user reviews tested reverse migration or snapshot restoration plus readable/exportable rescue data. Never reopen old A against an incompatible schema. Snapshot restore requires bounded writer fencing/drain and a verified preservation path for every newer conversation/attachment/domain; an archive with no working reader is insufficient. External bootstrap must perform recovery with the candidate host and maintenance component intentionally unavailable. Retain both local rollback artifacts and externally retrievable recovery material; GC follows explicit reference/retention rules.

## Proposed contract obligations

| Contract | Required inputs, output and authority |
|---|---|
| `deployment.slot` v1 | Immutable A/B composition and state identities, plan/gate/recovery digests and resource lease → staged/healthy/pending/failed status; no authority to modify active A |
| `state.transition` v1 | Complete provider inventory, compatible schema matrix, snapshot barrier, bounded catch-up cursors and owner leases → attested watermarks, fencing/ownership handoff or explicit blocked domains; per-domain state owner enforces writes |
| `deployment.route` v1 | Expected route revision, healthy deployment identity, current state/read-write compatibility and journaled command → atomic new-session route generation; durable session pins and stale-writer rejection remain authoritative |

Extend `recovery.snapshot`, `deployment.transaction` and `recovery.rollback` with these identities, overlap mode, B-session dispositions and newest-write preservation outcomes. Unsupported versions/capabilities, insufficient capacity, incomplete snapshots/catch-up, stale owners and uncertain cleanup fail closed for activation while leaving A available. These are proposed SDK contracts, not existing App Server methods.

## Required screens and implementation order

| Screen | Required visible behavior / exit |
|---|---|
| Storage and recovery | Show A + B + whole build/migration/backup/rollback peaks, ongoing-write allowance, per-filesystem/RAM reserves, local and external recovery verification; insufficient capacity leaves A usable |
| Stage / build / verify | A remains interactive; B and its side effects stay isolated; failed/skipped/stale gates remain visible and never select B |
| Prepare activation | Show live A sessions, pin/drain policy, provider compatibility/catch-up/ownership readiness, exact healthy B and restored recovery set; incompatible B stays pending |
| Activate and check | Journal and atomically route only new sessions to healthy B; existing A sessions remain usable/pinned, with owner/cursor continuity and visible drain state |
| Later rollback | Show post-cutover writes and active B sessions; verify rescue, compatible route rollback or explicit reverse migration/restoration and per-conversation recovery |

Implement in dependency order: (1) selection/scope/resource and immutable composition contracts; (2) durable maintenance owner, stable launcher/router and independent authenticated maintenance/recovery presentation; (3) consistent provider snapshots, bounded catch-up and fenced ownership; (4) full isolated candidate assembly/build plus coordinated compatibility and A/B coexistence health; (5) journaled route cutover, pin/drain and crash reconciliation; (6) newest-write-safe rollback and external recovery; (7) complete browser/runtime acceptance. Keep each component independently packaged where practical; the minimal external recovery owner stays an explicitly audited exception.

