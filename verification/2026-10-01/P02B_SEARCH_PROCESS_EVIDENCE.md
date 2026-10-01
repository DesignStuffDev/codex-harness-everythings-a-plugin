# P02B: versioned search service and real-peer process adapter

The new `codex-file-search-component` crate implements a versioned search service, validated value codecs and a `ProcessSearchBackend`. Its tests exercise retained ownership through **real independent protocol-peer processes**. They do **not** run the native matcher through an installed worker: service tests use a controlled backend, and peer tests use a protocol fixture. Independently installed native search remains outstanding.

Source is frozen at WIP **`bb03f3a81a828871e4e35d3e46b9c8528f3b7d00`**, tree `6ad17d58936b6c9f519c06f2932efd1c8ecd4724`. The recorded final fingerprints match **24 search-component, 49 component-host, 3 component-API, 6 path-codec and 10 search-API files** at that commit. Individual test manifests identify their actual source scopes. This is not whole-WIP acceptance; the high-level App Server client shutdown changes were not compiled by these gates.

## Implemented contracts and ownership

`codex-rs/file-search-component/src/` owns wire validation and accepted service work. Its versioned requests carry provider/lease/epoch identities, explicit roots, existing options and budgets. Codecs preserve decimal `u64` identity bits and same-platform non-UTF8 paths. Invalid root/result paths, indices, inconsistent counts and impossible idle frames fail explicitly.

Service reservations cover preparing, live and closing operations. Poll and update lanes stay owned when an observer abandons its wait. Joined close refunds quotas while retaining operation failure; unconfirmed cleanup retains allocations and fences the provider. Bounded retired receipts do not invalidate an older live lease, and oversized encoded snapshots return typed resource exhaustion with joined cleanup instead of silently truncating successful output.

The process adapter orders epoch allocation and bounded transport admission without waiting for an earlier open response. Paired cleanup ownership survives abandoned opens and late startup. It checks roots/options and query size before encoding or identity admission, fences stale/mismatched replies, preserves the original operation error on uncertain release, and closes owned work when the last public session/backend handle drops.

Persistent component launch settings now support an explicitly validated absolute working directory. This changes the child process's startup context, not the host cwd, package identity or state-directory binding; it is not a filesystem sandbox. Existing connection wrappers retain package-directory behavior. Catalog compatibility checks reject an incompatible selected search-package upgrade without changing the active package.

## Search-component verification

The corrected component run passed **22/22 tests, 0 skipped**, test/subreaper exit 0, with **95 unchanged source fingerprints**:

| Scope | Cases | Evidence |
| --- | ---: | --- |
| Service ownership with controlled backend | **6** | Preparing release/late startup, abandoned poll ownership, joined-failure quota refund, uncertain-close fencing, bounded retired receipts, encoded-frame overflow cleanup. |
| Value/wire codecs | **6** | Options/budgets, exact decimal identities, roots/query round trips, non-UTF8 path encoding, invalid paths/indices, impossible completion/count rejection. |
| Real peer-process adapter | **10** | Exact roots/query/explicit cwd, admission ordering, abandoned open/poll/update ownership, final-handle drop, stale/mismatched identities, pre-admission size rejection and uncertain release. |

These are protocol-peer processes, not an actual native search worker. Exact test names, source hashes and subreaper reports appear in [the machine-readable evidence](p02b-search-process-results.json).

The initial test command failed before execution with **five compiler diagnostics**: four ambiguous assertion imports and a move out of a match-guard binding in the new boundary tests. Explicit macro import and borrowing corrected the test fixture. During that failed compilation, only `Cargo.lock` changed as the new dependency was resolved; production/test code fingerprints stayed unchanged. The failed log remains separate from the corrected passing run.

## Separate host cwd/catalog regression

A separate component-host run passed **80/80 executed tests, 1 skipped**, with runner exit 0. Four launch-option cases verified concurrent explicit directories without host/package identity changes, retained default behavior, invalid directories/relative entrypoints rejected before launch, and handshake failure reaped before return. A catalog case verified that incompatible search-contract upgrade leaves the current selection intact.

Existing component lifecycle, logical limits, reserved cleanup, large history and parent-death cases remained enabled. Only `Cargo.lock` changed during this gate from API dependency resolution; all recorded code was unchanged. Its broad source manifest correctly reports `scoped_source_unchanged: false` rather than concealing lockfile drift.

The skipped top-level case is the parent-death subprocess helper, explicitly invoked by its passing parent test. Three adopted children were reaped with status `-9`; the host test command and runner exited 0. Neither the skipped helper nor those signal statuses are silently counted as ordinary passing top-level tests. The 80-case and 22-case results are separate runs, not a single aggregate 102-case acceptance.

## Changes after the passing component run

Scoped `just fix` made two mechanical changes in `service_operations.rs` and `process_session.rs`, then reported two warnings in `process_open.rs`:

- A documented `await_holding_invalid_type` expectation retains the intentional admission mutex. It spans only epoch allocation and bounded transport admission; response and cleanup waits do not hold it.
- The internal identity `expect` was replaced by typed malformed-reply handling. **This is post-test defensive behavior, not merely formatting. The new missing-identity branch was not separately exercised in the 22-test run.**

Final scoped fix compiled with exit 0, unchanged source and no warnings in its log. Global integration formatting then changed the recorded component/host files and unrelated high-level client files. Before/after fingerprints and the final immutable source binding remain separate in the JSON; no new exact-tree runtime run is inferred from lint/format success.

Original artifacts are `/workspace/acceptance/p02b-wire-service-process*`, `p02b-launch-cwd-and-catalog*` and `p02b-integration-format*`. Original failures and later successful gates remain independently fingerprinted.

## Remaining acceptance and custody

Catalog fixtures and a real peer process do not establish independent native search-package construction, installation, selection, removal or replacement. At this snapshot the [bounded native implementation](P02B_NATIVE_BUDGET_EVIDENCE.md) has no actual `NativeSearchBackend` trait adapter. Those pieces still need to be joined and exercised through the real host and existing GUI. No new CLI/App Server executable or browser interaction was tested in these gates.

WIP source is preserved on GitHub. The accompanying incremental archive at `/workspace/recovery-backups/20260930T165936Z/p01-source-20261001T051016Z.tar.gz` is cloud-local, SHA-256 `53c274520e38ce7f573239172af967680d19b00e1a27687cbde9f1d0acac93b3`; it is not independently durable outside the VM. Public reports contain whitelisted paths, outcomes and hashes, without private runtime reports, credentials, bearer URLs or session state.
