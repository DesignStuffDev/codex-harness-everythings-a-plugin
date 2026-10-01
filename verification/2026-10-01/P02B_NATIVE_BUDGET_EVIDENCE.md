# P02B: native index budgets and retained cleanup receipts

The native search implementation now enforces per-session **indexed-entry, charged index-byte and dedicated worker limits**, with typed startup/close outcomes retained by its async owner. This advances the real native implementation beyond the earlier [bounded matcher machinery](P02B_BOUNDED_INDEX_EVIDENCE.md). **An actual `NativeSearchBackend` trait implementation and independently installed native search worker are not present in this snapshot.**

The frozen source is WIP **`bb03f3a81a828871e4e35d3e46b9c8528f3b7d00`**, tree `6ad17d58936b6c9f519c06f2932efd1c8ecd4724`, preserved on `wip/p02b-bounded-search-20261001`. Post-format fingerprints for **20 native, 10 search-API and 41 vendor files** match that immutable commit. This identifies source, not acceptance of every file on the WIP; it also contains high-level App Server client shutdown work that these runs did not compile.

## Implemented native behavior

`codex-rs/file-search/src/native_budget.rs` preflights fixed matcher storage, optional highlighting scratch, supported entry capacity and the dedicated OS-worker allocation **`2*T + 2`**: one supervisor, one outer walker, up to `T` traversal workers and `T` matcher workers. Overflow or an insufficient budget rejects startup without admitting native work.

`native_index.rs` selects the prepaid matcher for bounded sessions. Its ledger atomically reserves each retained entry and its owned path/column bytes before creating those owned payloads. Both counters advance together or neither does. The first exhaustion is retained, further admission stops, and a partial exhausted walk cannot report a successful complete/idle result.

The vendored matcher adds a borrowed `Utf32AllocationPlan`. It preserves existing ASCII, Unicode and grapheme conversion while computing the retained backing-storage charge before allocation; exact-capacity construction avoids geometric-growth or shrink-copy accounting gaps. Nucleo remains pinned to `4253de9faabb4e5c6d81d946a5e35a90f87347ee` under **MPL-2.0**, with its licenses and covered-source provenance retained.

`native_session.rs` and `async_owner.rs` keep operation failure separate from cleanup proof. `create_bounded`, `close_outcome` and `shutdown_outcome` return typed receipts. Joined cleanup releases the owner's session slot even when the search failed. Lost/unconfirmed cleanup quarantines the slot; observer cancellation does not cancel accepted startup, callbacks or joins. A separately retained error cell preserves the first native failure if reporter destruction later panics and loses the close task's receipt. Legacy `anyhow` entry points and `FileSearchStartError`'s conservative cleanup-error behavior remain available.

## Exact verification

The corrected native/API/vendor run passed **122/122 executed tests, 0 skipped**, with test and subreaper exit 0. Its **74 scoped source fingerprints remained unchanged** throughout.

| Package | Passing cases |
| --- | ---: |
| Native file search | **56** |
| Backend-neutral search API | **11** |
| Nucleo | **23** |
| Nucleo Matcher | **32** |
| Total for this run | **122** |

Within these totals, **eight native-budget tests** cover fixed/highlighting preflight, worker ceiling/overflow, unsupported matcher capacities, atomic concurrent reservations, byte-overflow retention, actual entry/byte exhaustion without false complete/idle, and bounded-versus-legacy Unicode matches/highlights.

**Seven owner-receipt tests** exercise real native sessions: a cancelled close observer while a callback blocks; capacity reuse only after joining; joined reporter failure with immutable repeated receipts; actual entry exhaustion retaining `ResourceExhausted`; pre-admission rejection and reuse; abandoned startup while the blocking executor is occupied; and reporter teardown panic after an already observed native failure, retaining that first error while quarantining uncertain capacity. Legacy constructor receipt compatibility is also covered within these seven cases.

**Five matcher allocation tests** check actual representation/charge parity, combining graphemes, ASCII CRLF behavior, non-power-of-two capacity and target-layout overflow. These groups are subsets of 122, not additional tests. Existing search/query, cancellation, pool-join and matching tests remained enabled. Exact case names and source/log fingerprints are in [the JSON evidence](p02b-native-budget-results.json).

## Original failure and post-test changes

The first command failed at compilation with **32 ambiguous `assert_eq` diagnostics** in the new nested receipt tests; no tests ran. Explicit macro import and unused-import cleanup preceded the separately named corrected run. The original exit-101 log and source/subreaper manifests remain preserved.

After the passing run, scoped `just fix` exited 0 without changing source but warned about collecting reservation-worker handles before joining. That collection deliberately starts every worker before any join; replacing it with a lazy spawn/join chain would weaken concurrency coverage. A documented `expect(clippy::needless_collect)` retains the intended test semantics. Final scoped fix then exited 0 with unchanged source and no warnings in its log.

Scoped native/API formatting changed nine files. Explicit vendor formatting changed only `utf32_allocation_tests.rs`; the workspace formatter excludes that vendor workspace. A later global integration format also reformatted `native_budget_tests.rs`. Every before/after fingerprint is recorded separately. The 122-test run is evidence for its recorded pre-format bytes, not an invented second run of the final WIP. No tests were rerun solely for formatting/lint.

Original artifacts are `/workspace/acceptance/p02b-native-budget-owner*`, `p02b-native-budget-vendor-format*` and `p02b-integration-format*`. The JSON retains exact commands, transitions and immutable commit bindings.

## Remaining boundary and preservation

The ledger bounds retained native index allocations and dedicated OS-worker admission. It does **not** bound total RSS, allocator overhead, every traversal temporary, parsed query/output buffers, or the duration of arbitrary filesystem operations. Shared-runtime and aggregate-provider admission require the next backend/composition layer. Existing pinned allocator/Arc/Rayon accounting assumptions still require review when upstream or the toolchain changes.

No actual native `SearchBackend` adapter, independent native search package build/install/removal/replacement, new CLI/App Server executable or GUI runtime gate is claimed here. The separate [process bridge checks](P02B_SEARCH_PROCESS_EVIDENCE.md) use controlled backends/protocol peers and do not fill that gap.

The recorded source archive `/workspace/recovery-backups/20260930T165936Z/p01-source-20261001T051016Z.tar.gz` has SHA-256 `53c274520e38ce7f573239172af967680d19b00e1a27687cbde9f1d0acac93b3`. It is an incremental **cloud-local** recovery checkpoint. The GitHub WIP commit provides external durability for included source only. Public evidence contains whitelisted results and fingerprints, without credentials, bearer URLs or private session contents.
