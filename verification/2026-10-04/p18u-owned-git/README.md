# Installed maintenance package: direct Git lifecycle

Package `codex.maintenance.upstream-review` 0.1.1 now passes separately built,
installed-package and real active-Git cancellation acceptance through unchanged
manager `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.
This is additive C27/P18U lifecycle support. It extracts no further native subsystem,
advances no official upstream revision, and does not complete the updater.

## Source and change

The six-path change binds 8,964 scoped source files/map
`285c1464a236cf391d7c7b06c68ab17388e8c99079c5fc4a091f8b7595daa36f`.
The [source map](../../../upstream/p18u-owned-git-lineage.json) records the prior
published 914cc593 implementation, 777ee7c ledger, exact changed paths/symbols and
unchanged dependencies. Native Rust and the SDK remain unchanged. The two packaged
planner/validator files still byte-match the project-owned originals and retained
`TOOL_PROVENANCE.json`; they are not extracted upstream Codex implementations.

The package owns its direct Git child across active cancellation, a 30-second
command budget, the existing 90-second request deadline and interrupted reads.
It kills and waits up to two seconds for its child; unconfirmed cleanup remains an
explicit failure and prevents later command admission. A fresh Python exec wrapper
arms Linux parent-death protection before execing Git, with parent checks on both
sides of arming. No multithreaded `preexec_fn` or new process group is introduced.
Terminal ownership errors bypass the vendored planner's recoverable catches.

## Actual checks and limits

| Gate | Evidence |
| --- | --- |
| Focused Python checks | 58 passed on October 2: 17 planner, 22 lineage, 13 adapter and 6 ownership cases. Exact current source; no new focused rerun claimed. Original zero-discovery and missing-SDK attempts remain nonacceptance records. |
| External SDK/package acceptance | Fresh October 4 run: 23 commands, including 19 through the actual manager. Two independent project copies built with installed SDK0.1.0, then removed before install. Includes invocation, hash mismatch/missing-object handling, API incompatibility, explicit marked replacement, removal and standalone inspection with host absent from PATH. |
| Abrupt manager termination | Actual Git4984 held the exact FIFO read descriptor before SIGINT to manager4973 only. Manager status **-2**. Tracked manager/plugin/Git absence was established after **0.001505899 s** while the FIFO writer remained held. No rescue. A subsequent manager invocation succeeded with `review_required`, updates disabled. This is abrupt cleanup, not graceful protocol forwarding. |
| Cooperative installed protocol | The package was installed by the real manager; the installed entrypoint then received real SDK frames directly. Git8557's exact FIFO readiness preceded the shutdown frame. No OS signal was sent. A typed `cancelled_or_deadline_reached` response, plugin exit **0**, and tracked absence were established after **0.072412019 s**, writer still held, without rescue. A subsequent actual manager invocation succeeded. This does not prove the manager Call command forwards SIGINT gracefully. |
| Outer ownership and identity | All three current runs have strict subreaper exit0, no runner error, and only their command reaped at status0. Before/after source maps, host and package identities match. Final drain checks pass. |

The freshly built primary `plugin.pyz` is 30,214 bytes, SHA256
`d7d918a5f27cd82df4d8c89b36b453de235e099c711544a0fd09fa1218e9d1e2`.
[EVIDENCE.json](EVIDENCE.json) contains exact commands, timestamps, source/strict
receipt hashes and both package identities. Controlled runtime projections are
[PACKAGE.json](PACKAGE.json), [ABRUPT.json](ABRUPT.json) and
[PROTOCOL.json](PROTOCOL.json). Actual Git used a deterministic local object-read
barrier. No fake Git process, model fixture, engine conversation or GUI validation
is claimed by these tests. Process identity tracking remains sampled, supplemented
by the unchanged ownership/subreaper checks. A separate root observation found
Git PID2006 in state Z/parent1 after an object-cache fetch failure. Its causal
ownership was not independently established; it is not attributed to these
package tests. Their scoped results do not claim global process cleanliness.

## Failures and recovery provenance

The original 0.1.0 package's [real failure](BASELINE_FAILURE.json) remains a failure:
Git was still sleeping after5.009 seconds when its manager and plugin had ended.
The primary failure was saved before rescue; successful later invocation did not
turn it into a pass. The earlier [executable-selector setup failure](ORIGINAL_SETUP_FAILURE.json)
never established readiness and is separate from that real failure.

October 2 fixed-run outer receipts survived the runtime interruption and record
exit0, but their `/tmp` detailed reports/package artifacts no longer survive.
They are preserved as historical receipts and are not substituted for October 4
runtime proof. The fresh SDK installation was reconstructed offline from retained
wheel `41a13395059a4c315530e9533624e5b2cae1131988760a32acd0bc0614b3e97c`;
its temporary location is regenerable setup. Fresh detailed runtime state/reports
are retained under `/workspace/acceptance/p18u-resume-20261004-*`.
Workspace-local preservation is not externally durable until included bytes are
actually published and read back from GitHub.

Exact unchanged [abrupt](reproduction/active_git_acceptance.py) and
[protocol](reproduction/protocol_git_acceptance.py) helpers are retained for
reproduction. Invoke under the existing SDK subreaper runner using fresh isolated
work directories, the explicitly bound host/package/request, and actual Git's
executable pathname. The protocol helper also requires the abrupt helper through
`--prior-helper`. Do not rerun against historical output directories or weaken
FIFO/identity/reaping assertions. The existing
`component-sdk/tests/upstream_maintenance_acceptance.py` remains the package gate.

## Still open

Guarantees are bounded to a direct ordinary Git child on Linux and cooperative
SDK cancellation. Arbitrary descendants, non-Linux abrupt parent-death behavior,
whole-host graceful shutdown and streaming-memory bounds are not proved. This
checkpoint does not replace existing GUI/headless/native-storage regression
records. Native auth/P03 capacity remains closed. Candidate assembly from a later
pinned upstream revision, custom behavior preservation, coordinated host/plugin
versions, state migration compatibility, incompatible-update rejection and
external-bootstrap failed-update restoration remain required P18U work. Every
review still returns `update_allowed:false`; no polling or live update occurs.
