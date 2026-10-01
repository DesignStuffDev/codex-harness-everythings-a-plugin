# Pending-start SDK primitives: API-only checkpoint

The `codex-file-search-api` gate passed **19/19 tests** (11 existing contract tests,
8 new public carrier tests), with retries disabled, no skips and unchanged scoped
source. Scoped lint passed without source changes. Formatting passed and changed
only three mechanical wrapping/trailing-comma hunks in the new test file;
production was unchanged. The formatted test file was not rerun for this record.

This checkpoint adds an externally constructible `PendingSearchStart`, a per-start
`SearchStartControl`, and typed cancellation receipts. Ticket abandonment requests
cancellation before dropping its observer, including before the first poll.
Successful or failed terminal handoff preserves the exact result and disarms only
the ticket's Drop action. A retained control remains usable for the same lease.
The unwind tests exercise the actual carrier boundary without manufacturing
cleanup proof; arbitrary panicking destructors and panic=abort remain excluded.

**Classification: SDK primitives only.** Production backends, `SearchBackend`,
consumers, component wire and plugin package versions are unchanged. No active
Preparing cancellation, backend join/quota proof, new installed-plugin behavior,
or new UI runtime verification is claimed. Actual owners must still implement
bounded cancellation, exactly-once cleanup and immutable retained receipts.
The crate README distinguishes its historical support-only stage from the prior
[installed selected-search evidence](p02b-selected-search-results.json) and
[TUI/GUI evidence](p02b-tui-consumer-results.json), while retaining this gap.

The four source paths are published on `main` at
[`d22cea88aa23e35a619d996fc731122450e51313`](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/commit/d22cea88aa23e35a619d996fc731122450e51313),
with remote verification recorded by the owning agent. The tree is
`fbf28305751239cd376cefaec0ea970048096e08`, parent
`789be2e7b4a78ce15eb026293418dc574598ce36`. The runner's older baseline label is
historical and does not identify this tested change. Exact staged, tested,
formatted and published hashes are bound in the
[source lineage](../../upstream/p02b-startup-sdk-lineage.json) and compact
[gate results](p02b-startup-sdk-results.json).

The test subreaper reported exit 0, null runner error and only return code 0 among
reaped children. Raw logs/source reports remain cloud-local references; an
in-workspace file is not proof of external durability. Continue the
[Preparing cancellation plan](../../component-sdk/FILE_SEARCH_PREPARING_CANCELLATION_PLAN.md)
with real owner/backend integration after this narrow checkpoint.
