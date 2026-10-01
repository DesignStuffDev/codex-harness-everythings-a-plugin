# P02B: prepaid native matcher storage

This checkpoint adds **bounded index construction machinery inside the vendored matcher**. It does not yet enforce the native session's complete resource budget or provide an independently installed search component.

The post-format source is preserved at WIP **`98c540863b9868eb216dba18f963c220c4be5187`**, tree `d193d69db07c0ca378a9f95dae27d6dcee9d6019`, on `wip/p02b-bounded-search-20261001`. All **40 recorded vendor file fingerprints** match that immutable commit after the documented lint/format sequence. Later working-tree development is outside this evidence.

## Implemented boundary

In `codex-rs/third-party/nucleo/src/bounded.rs`, `IndexAllocationPlan<T>` computes checked target-layout charges before the caller creates its dedicated pool. `BoundedNucleo` requires the actual pool worker count to match that plan. The restricted API exposes one immutable, single-column index generation and omits legacy restart, unchecked injection and mutable owner access.

The fixed arena in `src/boxcar.rs` allocates its full geometric buckets up front, admits entries atomically, and moves already prepared values and columns into reserved slots. Rejection returns a typed `EntryLimit` error without advancing the visible count or losing accepted entries. Legacy unchecked injection is fenced from fixed arenas. Candidate, snapshot, in-flight and per-worker matcher storage is prepaid; internal assertions stop invariant violations before existing operations could grow those buffers.

`matcher/src/lib.rs` and `matcher/src/matrix.rs` expose the target scratch charge and a fallible scratch-allocation path. The earlier [inherited matrix extent correction](P02B_MATCHER_FIX_EVIDENCE.md) remains in place. Ordinary unbounded entry points and matching/scoring rules are retained.

Nucleo remains pinned to `helix-editor/nucleo` revision `4253de9faabb4e5c6d81d946a5e35a90f87347ee` under **MPL-2.0**. Covered source, licenses and intentional modifications remain recorded in `PROVENANCE.md`.

## Test results and source identity

The shared library run passed **166/166 executed tests, with 1 ignored entry**, exit 0 and subreaper exit 0. The source scope contained **105 unchanged fingerprints before and after**.

| Package/scope | Passing cases |
| --- | ---: |
| Nucleo bounded index and arena | **13** |
| Retained Nucleo and Matcher tests | **37** |
| Vendor total: Nucleo 23 + Matcher 27 | **50** |
| Native search | **41** |
| Component host | **75** |
| Shared run total | **166** |

The 13 focused cases cover invalid capacity and allocation-layout rejection; full geometric bucket charges at boundary capacities; concurrent admission/rejection; single ownership/drop of accepted and rejected payloads; moving prepared Unicode columns; legacy/fixed arena separation; exact pool-plan matching; notification panic ownership; and concurrent injection with superseded queries.

The matching-parity case compares bounded and ordinary native scores/results through append, replacement, case-insensitive and no-match queries, while checking that candidate/result capacities do not grow. Scoped pool tests join actual OS threads before asserting exit counts. These are real matcher operations, not a static accounting-only test.

The ignored top-level entry is the Linux parent-death subprocess helper; the passing parent-death regression explicitly invokes it as a child. It remains one ignored entry in the aggregate summary. Three adopted children were reaped with signal status `-9`; the runner and test command exited 0. This is not a claim that every child exited normally.

This is the **same** 166-test run referenced by [transport-limit evidence](P02B_TRANSPORT_LIMITS_EVIDENCE.md), not a second run. Exact case names, commands, source fingerprints, artifacts and commit bindings are in [the JSON report](p02b-bounded-index-results.json).

## Post-test lint and formatting

Scoped `just fix` exited 0 and changed six vendor files through 12 mechanical edits: redundant returns/parentheses, explicit elided lifetimes, `map_or` to equivalent `is_none_or`, and a finite test iterator to `repeat_n`. Read-only review found no scoring-arithmetic change or eager evaluation of an unsafe lazy operation. Two existing documentation-indent warnings remained; no warning-free claim is made.

`just fmt` exited 0 and changed eight component-host files, with no vendor changes. Because the workspace formatter excludes the vendored workspace, a separate `rustfmt` invocation then formatted only the three new vendor modules (`bounded.rs`, `bounded_tests.rs`, `boxcar_bounded_tests.rs`), also exit 0. The reports retain exact before/after fingerprints for each transition. The tested bytes are not relabeled as post-format bytes; no test rerun was performed solely for lint/format changes.

## Limits and remaining integration

Charges describe requested allocation layouts under pinned Rust 1.95 Global, **not RSS**. They account for full buckets, candidate/snapshot/in-flight buffers, matcher values and scratch slabs, pattern-column headers and owned Arc allocations. Path/column payloads, query atoms/clones, traversal/ignore metadata, callback state, thread stacks, Rayon bookkeeping and allocator overhead still require separate budgets.

The accounting depends on pinned allocator capacity behavior, Arc's current layout and Rayon's exact-size parallel extension path. Upstream/toolchain changes must re-audit those assumptions. Stable Arc allocation may still follow Rust's ordinary OOM abort behavior; fallible arena/slab/vector allocation is not universal OOM recovery.

The owner must stop and join producers, release injectors, quiesce the matcher outside its callbacks and join retained pool OS-thread handles. Constructor failure alone does not join caller-owned thread handles. **The complete native session ledger, shared composition limits and installed search backend remain unimplemented in this checkpoint.**

No new CLI/App Server binary, installed search package, GUI runtime, live model or cross-platform runtime gate is claimed. Earlier UI evidence is unchanged. The source archive recorded in the JSON is a cloud-local recovery checkpoint; the published WIP ref preserves its included source externally. Private runtime state, credentials and bearer URLs are not included.
