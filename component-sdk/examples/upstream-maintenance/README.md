# Independently installed upstream impact reviewer

This package exposes `tool:upstream_impact_review`, contract v1, through the existing
component manager. It packages the project's read-only P18U planner and lineage
validator, with exact source digests in `static/TOOL_PROVENANCE.json`. It is an
additive maintenance adapter, not extraction of another native subsystem.

The dedicated `maintenance:upstream` service is not supported by the current host.
The review operation does not fetch revisions, create a checkout, modify source or state,
approve an update, run builds, publish, migrate, activate or roll back anything.
It never schedules polling. Every report has `update_allowed:false`.

## Build and install outside the harness source

Copy this complete project into an independent directory. Use the separately
installed `codex-component-sdk` distribution in the build environment, then run:

```sh
python3 -B build_package.py --output /absolute/new/package-directory
/absolute/codex-component --codex-home /absolute/isolated/home install /absolute/new/package-directory
/absolute/codex-component --codex-home /absolute/isolated/home list
```

The build refuses an existing output and verifies the exact vendored tools before
packaging. Python 3.10+ and Git are runtime dependencies. The host and SDK use
component API v1; no Rust ABI, host rebuild or host source import is required.
Normal immutable package discovery, version checks, removal and explicit selection
apply. Installation alone follows the existing manager's activation rules.

## Explicit chosen-revision request

Write a JSON request with exactly these required fields and optional receipt pair:

```json
{
  "contract_version": 1,
  "repository": "/absolute/existing/local/object-repository",
  "lineage_index": "/absolute/CHECKPOINT_LINEAGE_INDEX.json",
  "lineage_sha256": "EXACT_64_HEX_SHA256",
  "candidate_revision": "EXACT_40_HEX_COMMIT",
  "composition_revision": "EXACT_40_HEX_COMMIT",
  "publication_receipt": "/absolute/CHECKPOINT_LINEAGE_PUBLICATION_RECEIPT.json",
  "publication_sha256": "EXACT_64_HEX_SHA256"
}
```

Invoke the manager's `call tool upstream_impact_review invoke` with one JSON
argument containing `{"call_id":"review-1","name":"upstream_impact_review","arguments":<request>}`. Use a subprocess argument array
to preserve JSON; do not construct a shell command from request data. The manager
prints the final tool result. Its `text` is the versioned review JSON and `success`
means a report was computed, never approval to update.

Only local immutable object IDs are accepted. Missing objects, stale composition,
unknown/multiple owners, incomplete semantics, security/migration/contract review
and omitted details remain blocking findings. A hash identifies bytes; it does
not authenticate an upstream publisher or establish later-revision ancestry.
The normalized lineage index still describes its historical checkpoint; this
component does not silently advance it to the latest custom source.

The request, report and packaged-tool digests bind a deterministic `plan_id`.
The review operation has no mutable job state, deployment journal or resumable
update. Its inputs belong to the caller; it owns its request process and read-only
Git subprocesses. Review reports are returned, not written into a live home.

## Standalone bootstrap and lifecycle

The built package includes a host-independent inspection entrypoint:

```sh
python3 -B /absolute/package-directory/bootstrap.py /absolute/request.json
python3 -B /absolute/package-directory/bootstrap.py /absolute/request.json --full-report
```

It imports the packaged worker from `plugin.pyz`, so neither a working Codex host
nor the harness source directory is required. Exit 1 means invalid input; exit 2
means review remains required or cancellation. It cannot restore a deployment or
reverse a state migration. Those external recovery operations remain future P18U
work. The normal tool result is report text bounded to 8 KiB; the bootstrap can expose the
planner's wider existing 8 MiB bound. This is a report-text bound; JSON escaping adds outer protocol framing, which remains subject to the SDK frame cap. A native model caller may impose a smaller response budget. Omitted details always block approval.

The existing SDK supplies startup, request/result framing and one-shot shutdown.
Package 0.1.1 polls active Git calls for SDK shutdown cancellation at 100 ms
intervals and stops admitting commands after a 90-second job deadline. Each Git
call retains a 30-second execution budget; cancellation, timeout or an interrupted
read kills and waits up to two seconds to reap the direct child. A failed reap is
reported as `owned_git_cleanup_unconfirmed`, never successful cleanup. These
terminal errors bypass the vendored planner's recoverable object-read failures and
stop further admission on that object owner. Caller cancellation or deadline expiry
returns `status:cancelled`; unconfirmed cleanup returns `status:invalid`. Neither
result reports a completed plan or tool success.

On Linux a fresh Python wrapper arms `PR_SET_PDEATHSIG(SIGKILL)` before replacing
itself with Git, checking the expected parent before and after arming. The wrapper
avoids `preexec_fn` in the multithreaded plugin and works from the installed zipapp.
This protects the direct Git process if its plugin parent dies abruptly, including
manager-only termination. It does not isolate arbitrary descendants, and non-Linux
platforms do not gain parent-death protection. Children remain in the host's existing
process group. The 120-second one-shot host timeout and process-group cleanup remain
outer limits; there is no instantaneous or whole-tree graceful-cleanup guarantee.
Git capture limits remain post-capture, not streaming-memory bounds. The package
is trusted executable code, not an OS sandbox or new broker permission grant.

## Required acceptance before claiming this milestone

Build the package from an independent project copy with the SDK distribution;
install, list and invoke it through the unchanged actual component manager. Check
exact chosen revisions/tool/input digests, expected review blockers and host hash
stability. Run a missing-object request and a digest-mismatch request; neither may
fetch or mutate the repository. Invoke the bootstrap with the host intentionally
unavailable. Remove the package and require discovery/invocation to fail. Reinstall
and exercise explicit replacement selection with a second package identity.

For package 0.1.0, the 17 planner, 22 lineage and 11 adapter checks passed, followed by 23 acceptance commands,
including 19 through the unchanged real manager. See the repository's
`verification/2026-10-02/p18u-installed-impact/README.md` for exact source identities,
package hashes and limits. Real later-upstream integration, current semantic
closure, custom-plugin/UI preservation, coordinated versions, migration and
failed-update recovery are still required for complete P18U acceptance.

Package 0.1.1 additionally requires real installed-package cancellation and manager-only
termination checks while actual Git is held in a read, tracked child cleanup, and
a successful subsequent invocation. Proposal source and unit tests alone do not
establish that acceptance.

## Source-input capsules (package 0.2.0)

`tool:upstream_source_capsule` v1 adds bounded preparation in a fresh private job
under the component's own state directory. Its tool-v1 arguments contain
`contract_version:1`, `purpose:"prepare_source_inputs"`, the exact `review_request`,
its `expected_plan_id`, and `limits` (`max_changed_paths` up to 1024,
`max_unique_blob_bytes` up to 67108864, `max_manifest_bytes` up to 8388608).
The chosen revision must be a distinct verified descendant and all three committed
trees must already exist locally. No implicit fetch, checkout or source application occurs.

The capsule stores exact changed-path base/upstream/custom entries and bounded blob
bytes. It is sparse, not a complete source backup or runnable candidate. Uncommitted
work and unchanged blobs are outside its committed-tree scope. Historical ownership
and compatibility findings remain unresolved; every update/activation flag stays false.
Unsupported entries or exceeded budgets refuse success. Partial jobs are retained.
The producer acknowledges successful directory fsync; standalone inspection checks
current integrity and does not attest historical crash durability. A successfully
persisted terminal marker keeps inspection incomplete even if the seal file became
visible. Recording that marker is best effort: if its write also fails, later
inspection cannot reconstruct the producer's known I/O failure. Producer failure
never approves an update.

Run `python3 -B /package/capsule_bootstrap.py /absolute/job-directory` to inspect a
job without the host or Git. This is source-artifact inspection, not state rollback.
The new capsule producer and tests require their own recorded runtime acceptance;
the earlier review-only evidence does not validate this added capability.

## Candidate source overlays (package 0.3.0)

`tool:upstream_candidate_overlay` v1 transforms a sealed source capsule into a
bounded, reviewable overlay. Arguments are `contract_version:1`,
`purpose:"prepare_candidate_overlay"`, `source_capsule_id`,
`expected_manifest_sha256`, and `limits` (`max_changed_paths` up to 1024,
`max_output_bytes` up to 67108864, `max_manifest_bytes` up to 8388608).
Only component-owned input/output jobs are used; model-supplied filesystem paths
are not accepted. Package 0.3.0 preserves both older tool-v1 contracts and capsule-v1
state. Upgrading the maintenance package does not update the harness engine.

Unmodified custom paths take upstream bytes; already matching changes retain exact
bytes. Divergent regular text uses owned `git merge-file --stdout --diff3`.
Deletion, executable-mode change, add/add divergence, binary divergence, oversized
merge inputs and nonzero merge results explicitly require review. Conflict output
is a diagnostic artifact, never an executable candidate path. A clean textual merge
does not establish behavioral compatibility or authorize update/activation.

Merge inputs are capped at 128 KiB each; larger direct selections retain the capsule's
4 MiB blob limit. Three reusable private scratch files plus at most one partial
replacement use at most 512 KiB. Retained result/diagnostic blobs respect the caller's
output budget. Git stdout limits are enforced after capture, not a streaming memory
bound. The 90-second job and existing 30-second command/two-second cleanup ownership
limits apply; terminal receipt writes remain best effort under filesystem failure.

The result is an overlay on the exact committed custom tree, with untouched paths
inherited by reference. It is not a complete checkout, source backup or runnable
candidate. Uncommitted work is excluded. `prepared` means every changed path has a
source result; coordinated versions, state migrations, security/contracts, a new
candidate build, unchanged custom-plugin behavior, GUI/headless tests and external
rollback remain explicitly pending. All approval and activation flags stay false.

`python3 -B /package/overlay_bootstrap.py /absolute/overlay-job` checks present
artifact integrity and its retained source capsule with Git/host absent from PATH.
Keep both job directories in their original component state layout. Removal retains
these artifacts. Inspection neither replays the merge nor attests prior durability,
semantic compatibility, installation recovery or rollback. This capability requires
its own independent build, real installed-package upgrade and runtime evidence.
