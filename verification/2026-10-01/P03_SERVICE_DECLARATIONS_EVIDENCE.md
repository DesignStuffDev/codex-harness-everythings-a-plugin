# P03 service declaration validation evidence

The five affected packages passed **177 tests, with 1 skipped and zero test retries**, on the adopted declaration source. All seven new tests passed: six strict API/compatibility cases and the actual catalog load/rejection case. This is custom component infrastructure; it does not extract another native subsystem or activate the reverse-service broker.

The twelve source paths match the frozen declaration stage exactly (`2705a5c51b656e556f7fc488f28e2e0cdbcf5f1318fd88c316927f311a4efc9a`). The generated Cargo.lock delta is exactly one `pretty_assertions` edge in `codex-component-api`, verified against the pre-adoption archive. `just bazel-lock-update` exited 0 and left MODULE.bazel.lock unchanged. The lineage file binds the twelve actual tested hashes plus Cargo.lock; the full source reports retain 8,864 path hashes. Their baseline commit label is historical metadata, not an assertion that the dirty tested tree equals that commit.

| Gate | Observed result |
| --- | --- |
| Unfiltered offline metadata | Exit 101: uncached `android_system_properties v0.1.5`; zero tests. Cargo.lock had already gained the intended edge. |
| Linux-filtered metadata | Valid 1,315-package metadata artifact; root reported successful completion. No separate exit-code receipt was supplied. |
| Required Bazel lock generation | Exit 0, source unchanged. |
| First five-package suite | Linker SIGBUS while building the thread-store test executable; exit 101, zero tests. Root reported concurrent disk exhaustion; the log does not itself prove ENOSPC. |
| Same suite after archive-backed disk recovery | 177 passed, 1 skipped, no retries; all 8,864 before/after source hashes equal, and equal to the failed attempt's source map. |

The passing gate ran `just test --locked --retries 0 --test-threads 2` for component-api, component-host, file-search-component, thread-store-component and attachment-store-component, with one Cargo build job and the unchanged strict subreaper. The wrapper exited 0 with no runner error. Its raw records include four descendant returns of -9 and the command return of 0; these are retained rather than described as every child exiting cleanly. The console records one skipped test without naming it. Per-binary passes were API 6, host library 82, manager binary 6, search component 52, thread storage component 23 and attachment component 8.

The earlier failure and recovery remain separate artifacts. Root preserved completed old TUI generated executables before retiring their mutable aliases; accepted frozen hosts and source were excluded. The verified adoption archive is an in-workspace recovery checkpoint, not proven external durability.

Plain manifests still omit absent requirements. Strict checked declarations reject malformed versions, duplicate/unknown fields, counts and UTF-8 byte limits. Required and optional declarations parse as DTOs but both fail closed at actual catalog loading until coordinated broker activation. The new public Rust struct field requires external struct literals to supply it; the six current workspace literals use `None`.

Scoped lint, formatting, any tested-to-formatted hash changes, a fresh manager install/reject/restoration gate, and publication binding are **pending in this evidence snapshot**. There is no new host-authority, allocation-bound, independently installed broker, native extraction or GUI claim. Exact report/log hashes, failed attempts, source bindings and raw subreaper statuses are in the accompanying results and lineage JSON.
