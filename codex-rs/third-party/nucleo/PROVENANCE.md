# Vendored Nucleo

This directory contains Nucleo 0.5.0 and its Nucleo Matcher 0.3.1
dependency from <https://github.com/helix-editor/nucleo>, pinned to commit
`4253de9faabb4e5c6d81d946a5e35a90f87347ee`. The source was copied from the
existing Cargo Git checkout of that exact revision, without its `.git` metadata.

Both crates are licensed under **MPL-2.0**, not the surrounding project's
Apache-2.0 license. The original `LICENSE`, `matcher/LICENSE`, copyright notices,
and README are retained. Local changes to the covered files remain under
MPL-2.0; distribution must retain the license and make the covered source,
including these changes, available as required by that license.
The upstream `matcher/LICENSE` symlink to `../LICENSE` is materialized as an
ordinary file containing that exact license text for portable source packages.

Copied upstream paths are `Cargo.toml`, `LICENSE`, `README.md`, all of `src/`,
and all of `matcher/` (including its tests, fuzz inputs and supporting scripts).
Upstream benchmark and repository automation files are not vendored. The local
crate manifest removes the absent `bench` workspace member; dependency versions
and matcher feature defaults are unchanged. No separate Cargo lockfile was
copied: the enclosing harness controls dependency resolution.

## Local changes

- `src/lib.rs`: adds `Nucleo::new_with_thread_pool`, accepting an owned, dedicated
  Rayon pool so the caller can configure panic handling and retain actual OS
  thread handles. The existing constructor keeps its default thread selection,
  thread names and construction failure behavior, then delegates to this API.
- `src/worker.rs`: initializes the existing native matcher state from the actual
  supplied pool thread count. Matching, scoring, sorting and cancellation
  algorithms are otherwise unchanged. This also handles Rayon's automatic or
  clamped pool sizes without under-allocating per-thread matcher state.
- `src/lib.rs`: adds consuming `Nucleo::shutdown`, which disables matcher
  notifications, requests cancellation and waits without a timeout for the
  worker lock before normal drop. The existing ordinary `Drop` is unchanged.
- `src/shutdown_tests.rs`: exercises real matching on a supplied scoped pool and
  shutdown waiting for a running native worker notification. These are local
  behavioral tests; the upstream matcher and high-level tests are retained.

- `matcher/src/matrix.rs`: corrects an inherited matrix-view extent in
  `MatrixLayout::fieds_from_ptr`. Its raw slice now uses the same
  `(haystack_len + 1 - needle_len) * needle_len` cell count as the reserved
  layout. The previous expression multiplied by `haystack_len`, which could
  cause `MatrixSlab::alloc` to form a mutable slice beyond its owned slab for
  admitted inputs with a shorter needle. Matching scores and selection rules
  are unchanged.
- `matcher/src/matrix_extent_tests.rs`: validates ASCII and Unicode matrix
  extents using raw pointer metadata without creating potentially invalid
  references. This permits safely demonstrating the regression before the fix.

## Ownership and limits

`shutdown` quiesces matcher work and drops the owned pool. It does **not** itself
join OS threads. The component must stop and join producers, drop injectors,
call `shutdown` outside the worker pool and notification callbacks, then join
every retained thread handle or exit the enclosing OS thread scope. Injector
notifications are independent of the matcher cancellation flags.

Rayon `spawn` task panics reach the configured pool panic handler, not ordinarily
the OS-thread join result. The caller must retain that failure and report it
after joining. An exit callback is not an OS-thread join. This patch does not
modify Rayon, use a global pool, add a hard shutdown deadline, or claim recovery
from process-aborting failures inside Rayon.


## Bounded single-column index construction (Stage B work in progress)

The new restricted `BoundedNucleo`/`BoundedInjector` API in `src/bounded.rs`
preserves the original unbounded API and matching/scoring rules. It does not
expose restart, unchecked injection, or mutable access to the legacy owner.
`IndexAllocationPlan<T>` computes checked target-layout charges before a caller
starts and reserves its worker pool. The constructor requires that pool's actual
worker count to match the prepaid plan.

- `src/boxcar.rs` adds fallible, fully prepaid single-column arenas, atomic entry
  admission, and non-panicking moves of prepared values/columns. The bounded
  path does not use eager/lazy bucket growth or the generic filler callback.
- `src/worker.rs`, `src/pattern.rs` and `src/lib.rs` construct prepaid candidate,
  snapshot, in-flight and matcher storage. Assertions fence internal invariant
  violations before an existing reserve/extend/clone could grow those buffers.
  Configured entry exhaustion instead returns the typed `EntryLimit` error.
- `matcher/src/lib.rs` and `matcher/src/matrix.rs` expose the actual target
  scratch charge and a fallible scratch allocation path. Ordinary construction
  retains its allocation-failure behavior.
- `src/boxcar_bounded_tests.rs` and `src/bounded_tests.rs` cover arena boundaries,
  concurrent rejection, payload ownership, matching parity, stable candidate
  capacities, constructor mismatch, notification panic and joined scoped pools.
  Test execution evidence is recorded by the parent harness, not implied here.

Fixed accounting includes full geometric arena buckets, both candidate/result
vectors and the in-flight vector, matcher values/slabs, pattern-column headers,
and owned Arc allocations. Payload paths/columns, query atoms/clones, traversal
and ignore metadata, thread stacks, callback state, Rayon runtime bookkeeping,
and allocator overhead are separate; this is not an RSS limit. The native
session ledger and separately installed backend remain later integration work.
Stable Arc constructors can still follow Rust's ordinary OOM abort path; the
fallible slab/arena/vector paths do not imply universal system-OOM recovery.

Accounting depends on pinned Rust 1.95 Global recording the requested capacity
for a fresh `try_reserve_exact`, and repr(C) ArcInner's two atomic counters plus
data layout. Neither is asserted to be a stable allocator/Arc ABI. It also
relies on Rayon 1.11's exact-size `ParIter.map` to `Vec::par_extend` path writing
into reserved destination capacity without candidate-sized intermediate vectors;
do not replace it with unindexed filtering without re-auditing allocations.
Upstream or toolchain updates must recheck these assumptions and the concrete
fixed layout before accepting the new source.
