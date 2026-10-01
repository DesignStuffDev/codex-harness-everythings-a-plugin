# P02B: real native search backend

`NativeSearchBackend` now implements the replaceable search contract using **actual native traversal and matching**, with retained ownership and aggregate resource admission. Its focused gate passed **141/141 tests, 0 skipped**. This is real in-process backend behavior; **independently installed native search and selection through the host remain unverified**.

Source is preserved at WIP `cb977e7d664bc752c28854d468619039bd3ad167`, tree `40207f354efb3bee85265d8f3d83312489b56709`, parent `bb03f3a81a828871e4e35d3e46b9c8528f3b7d00`, on `wip/p02b-bounded-search-20261001`. All **82 final source bindings** match that immutable commit, including the runner. GitHub's `native_backend.rs` blob is `98a82dd1196735ade842c8cb09345978c68be5cd`. Post-test mechanical changes are recorded separately; this does not claim every WIP file passed a new host/GUI gate.

## Implemented ownership and limits

- `native_backend.rs` owns a retained `FileSearchOwner`, checked aggregate session/entry/index-byte/worker reservations, startup coordination and shutdown. Joined cleanup refunds capacity before publishing the receipt; uncertainty retains the reservation. Whole task guards preserve cancellation bookkeeping before releasing their tracker tokens.
- `native_backend_session.rs` separates public leases from internal task references. Dropping the final public lease requests closure even while the lifecycle actor retains state. Query enqueue and identity admission share a fence; stale callbacks cannot relabel old results. Operation failure remains distinct from joined cleanup.
- `native_backend_poll.rs` retains one pending poll in the existing lifecycle actor. Abandoning the observing future cannot free the slot. A real revision, timeout or closure completes it; no per-poll task or OS observer thread is created.
- `native_backend_policy.rs` requires explicit policy and an absolute base exactly matching the embedding process's stable cwd. Clean invalid inputs reject before admission. Roots, exclusions and query strings are copied into compact fallible storage without changing their lexical OS-string values or process cwd.
- `native_output.rs` preflights one callback snapshot's backing allocations: result vector, query/path/root bytes and positive-atom highlight capacity. Deduplication may leave capacity larger than final index length. Native snapshot charge, later client copies and encoded process frames have separate limits.

The prior per-session index ledger still charges prepaid matcher/index structures, owned path/UTF32 payloads and `2*T+2` dedicated OS workers. The backend reserves requested allowances across sessions. These bounds are **not RSS limits**: traversal metadata, stacks, allocator overhead, parsed patterns, shared-runtime bookkeeping and later snapshot copies are outside index accounting. Pinned allocator/Arc/Rayon assumptions remain relevant. Universal OOM recovery, interruptible filesystem operations and production-scale default budgets are not claimed.

## Actual verification

The corrected `p02b-native-backend-tests-corrected` run completed with unchanged **82 scoped source fingerprints**, test/subreaper exit 0 and no skipped tests:

| Package | Passed |
| --- | ---: |
| Native file search | 75 |
| Neutral search API | 11 |
| Nucleo | 23 |
| Nucleo Matcher | 32 |
| Total | **141** |

The native 75 comprise the prior 56 plus **14 real-backend and five output cases**. Backend tests exercise equal/normalized/A–B–A/no-match query identities and complete snapshots, retained polls after observer cancellation, final public lease/startup waiter drop with actual successful re-admission, sibling isolation, each aggregate resource axis, clean input rejection, true entry/output exhaustion with typed joined receipts, repeated close/shutdown, lexical roots and spare-capacity input semantics.

Output cases verify exhaustion without successful Idle and with released index ownership, rejected query bytes without identity advancement, Unicode/CRLF/multi-atom highlight parity, exact platform path copying and compact capacities. Spare-capacity backend tests prove preserved real query/root/exclusion behavior; separate helper assertions check actual allocation capacities. This was Linux execution, not Windows/WTF8 runtime proof. These groups are subsets of 141.

The original `p02b-native-backend-tests` command failed compilation before any test ran: `TaskTrackerToken` used an unexported import path, and the compiler reported three guard-capture warnings. The correction imported the token through `task_tracker`, moved whole preconstructed guards into their async bodies and retained token-drop ordering. The original exit-101 log/source/subreaper reports remain separate.

After the passing test, scoped fix replaced two unnecessary outcome clones with moves and collapsed a nested condition without changing short-circuit order. An unused test import was then removed. Final fix exited 0 with unchanged source and no warnings; final formatting changed only that collapsed condition's layout. Reversing those mechanical changes in temporary copies recovered the recorded tested hashes. No second test run is invented for these transitions.

[The JSON evidence](p02b-native-backend-results.json) records every case, original artifact hash, before/after fingerprint, exact commit binding and reviewed transition. The runner's earlier baseline label is explicitly a reference, not the identity of the tested working tree.

## Remaining acceptance and preservation

The native worker crate was applied only **after** this checkpoint; it and the staged facade are outside this gate. Its first 33-case run had one configuration-validation failure. After strict object/null validation, the next run eventually passed 33 cases but marked the exhaustion test **flaky**: its first attempt observed `ClosedLease` instead of the retained `ResourceExhausted`. That typed first-error/close race remains unresolved and the later worker gate is not accepted. The historical 141-pass result does not prove every error race safe. Independent package construction/installation, selected native/custom replacement without rebuilding the host, consumer composition, App Server/TUI/one-shot behavior, removal/upgrade and desktop UI acceptance remain outstanding. No new GUI/Browser or manager Launch check was performed. This completes neither P02 nor whole-harness extraction.

The client shutdown fixture is included in this WIP, but its [40/40 gate](P02B_CLIENT_SHUTDOWN_EVIDENCE.md) ran before native integration and does not cover the new backend.

`/workspace/recovery-backups/20260930T165936Z/p02b-native-backend-tested-source.tar.gz` has SHA256 `842dc08868198a68752e7c7fde61143783115540a2b8c8438647b88f40ea11eb`. It is a checksum-verified **cloud-local** source checkpoint. GitHub provides external durability for included source only. Original failures and execution reports remain preserved. Public evidence contains safe source identities and results, without credentials, private bearer URLs or session contents.
