# Process pending-start cancellation: focused WIP evidence

`codex-file-search-component` passed **42/42 tests**, including 11 new process
pending-start tests and 31 existing component regressions. Retries were disabled;
no tests were skipped, and the captured source was unchanged. The unchanged
subreaper reported exit 0, no runner error, and only successful reaped return codes.

The process backend now exposes inherent `begin_open` with one retained startup
owner and per-start cancellation control; existing `SearchBackend::open` delegates
to it. Real stdio peer tests cover admission cancellation, sibling/provider
continuity, queued-ready and post-handoff cancellation, observer abandonment,
exactly one paired RELEASE, quota retention/refund, immutable repeated receipts,
and uncertainty after dropping an actually unpolled runtime task. These receipts
preserve operation failure separately from cleanup certainty.

Review corrected a real cause-loss case: remote OPEN `ClosedLease` is now
classified by origin rather than mistaken for caller cancellation, so later
release failure cannot replace its cause. Tests also wait for actual first OPEN,
publish epoch metadata before the release marker, validate full stored lease
identity, and reject new work on a stopped peer lease. This makes the sibling
and exact-lease assertions meaningful without weakening accepted-work drainage.

The first lint run exited0 with one warning for a deliberately held admission
mutex in a test. Root added a narrow, reasoned `#[expect]` to that test; the second
lint run was clean and source-unchanged. Exact reconstruction of this annotation
matches the recorded hash. Formatting then changed four files only (mechanical
wrapping and private-module ordering). No post-annotation/format test rerun is
claimed; the [lineage](../../upstream/p02b-preparing-process-lineage.json) binds
those distinct tested, linted, formatted and published inputs.

Seven source files are published on
`wip/p02b-preparing-cancellation-20261001` at
[`e189a695fafd29273fcbcfadd675cc83d1784606`](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/commit/e189a695fafd29273fcbcfadd675cc83d1784606),
with remote verification recorded by the owning agent. Tree:
`6d4bddb3c0e3b3f592a9f04922bc3e85ab331072`; parent:
`4ce3e7057cf5afbe43491a4293b6368b0da98379`.
See the compact [gate results](p02b-preparing-process-results.json).

**Scope remains process-adapter WIP.** The peer is an actual child process, but
it is not an independently installed native worker. This gate does not establish
native constructor cancellation, worker-service control forwarding, facade/AS/TUI
adoption, a new plugin package version, or full-host/GUI/Browser behavior. No new
subsystem extraction or completion of P02/whole-harness work is claimed.

The original uncompiled proposal and corrected author snapshot are preserved in
a checksum-verified cloud-local archive; their author-time zero-test status remains
separate from the adopted root-owned 42-test result. A second verified archive
matches all seven published files. These in-workspace archives are recovery
checkpoints, not externally durable backups. The remote-verified WIP commit
provides external durability for its included source. Continue the coordinated
[Preparing cancellation plan](../../component-sdk/FILE_SEARCH_PREPARING_CANCELLATION_PLAN.md).
