# Independently installed upstream impact reviewer

This package exposes `tool:upstream_impact_review`, contract v1, through the existing
component manager. It packages the project's read-only P18U planner and lineage
validator, with exact source digests in `static/TOOL_PROVENANCE.json`. It is an
additive maintenance adapter, not extraction of another native subsystem.

The dedicated `maintenance:upstream` service is not supported by the current host.
This package does not fetch revisions, create a checkout, modify source or state,
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
There is no mutable job state, deployment journal or resumable update in this
milestone. Inputs belong to the caller; the plugin owns only its request process
and read-only Git subprocesses. Reports are returned, not written into a live home.

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
