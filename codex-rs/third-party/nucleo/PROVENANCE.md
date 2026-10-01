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
