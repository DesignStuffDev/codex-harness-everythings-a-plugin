# P02 StageB contract and path-codec checkpoint

Source `98eeb3e1c93b3a856834c57e6c4d57bfa9f8c782`, tree
`01e5f25c3a40cd90ea4cae430e481a3e40cc9e65`, registers the neutral search API and
shared same-OS path codec. This is a tested **support-library** slice. It does not
activate independently installed search, rebuild the host, or prove backend cleanup.

`codex-file-search-api` has no matcher, Tokio, Core, host or history dependency.
It defines query/frame identities, explicit resource budgets, bounded diagnostics
and startup/close outcomes. A retained operation failure is separate from actual
joined cleanup. Only a valid receipt for the current selected owner can release
capacity; a decoded enum alone is not authority. Real quota and worker lifecycle
checks belong to the forthcoming implementations.

The lossless path implementation moved from `component-state-codec` to
`component-path-codec`; old storage adapter paths re-export it. UnixBytes and
WindowsWide tags, foreign-platform rejection, non-UTF paths and exact absolute
path validation remain covered. This refactors our existing support codec, not
an original Codex service into an installed plugin.

| Check | Result |
| --- | --- |
| Offline check | Passed; only Cargo.lock changed to register two local packages |
| Neutral search API | 11/11 passed |
| Shared path codec | 8/8 passed |
| Existing state codec | 37/37 passed |
| Combined locked test run | **56/56 passed**, zero skipped, unchanged scoped source, subreaper exit0 |
| Scoped `just fix` | Exit0, unchanged source, no warnings |
| `just fmt` | Exit0; formatting changes in two API files |
| Bazel lock update | Exit0; MODULE.bazel.lock unchanged |
| Bazel targets/direct dependencies | Resolved; no Bazel build/test claim |

No external package dependency identity changed. The wire cases ran on Linux;
Windows runtime, installed search replacement and new-host GUI integration remain
future gates. No tests were rerun solely for formatting/lint. README status was
updated after verification. The separate native queue/transport startup work is
excluded from this source commit even though it shares the working tree.

[Machine-readable results](p02b-contract-results.json) record exact commands,
source hashes before/after, log fingerprints and the subreaper result.
[Lineage](../../upstream/p02b-contract-lineage.json) maps exact current/base blobs,
original native value definitions, and intentional changes. The previous
[StageA evidence](P02_SEARCH_PREREQUISITE_EVIDENCE.md) remains the latest completed
new-host/storage/GUI runtime gate; it is not relabeled for these newer libraries.
