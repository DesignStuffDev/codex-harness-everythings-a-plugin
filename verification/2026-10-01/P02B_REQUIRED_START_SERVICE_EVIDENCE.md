# P02B required Preparing startup contract and service evidence

The focused gate passed **169/169 tests, zero skipped, zero retries**. This
checkpoint makes owned `SearchBackend::begin_open` mandatory and integrates its
per-start control into the worker service. It strengthens the already extracted
file-search boundary; it does not extract another subsystem. Source is preserved on cancellation WIP `272993ff0e6376e7ba52403e3b52d7f16449cd16`; accepted main runtime is unchanged. The machine report preserves exact commands, log/report identities,
source transitions and thirteen file bindings.

The required method returns `PendingSearchStart` synchronously. Native and process
implementations delegate to their real retained owners; `open` remains an observer
convenience. There is no fallback wrapping an uncancellable startup future. The
service reserves Preparing in reader order, forwards a latched release after
control publication outside locks, retains original finish independently from
cancellation observation, and preconstructs startup/close guards before spawn.
Known cleanup preserves NotAdmitted versus Confirmed; lost proof is Unconfirmed
with the reservation retained. Receipt publication is immutable.

| Gate | Actual result |
| --- | --- |
| `p02b-required-start-service-tests` | Compilation failed, exit 101; no tests executed. 34 ambiguous `assert_eq` errors and eight moves from pattern guards in new fixtures. |
| `…-tests-02` | 169 passed: 96 native-library, 52 component, 11 API library and 10 API startup tests. Exit 0; all 10,131 captured source fingerprints unchanged. |
| `…-fix` | Exit 0, source unchanged; one Option-unwrap warning in a new API test. |
| `…-fix-02` | Exit 0, source unchanged, no warnings after replacing the test constant with `NonZeroUsize::MIN`. |
| `…-format` | Exit 0; five mechanically formatted paths. Tests were not rerun solely for lint/format. |

Both test commands ran through the unchanged strict subreaper with no runner
error. The failed compiler command was reaped with 101, the successful command
with 0; this is not a blanket claim that arbitrary descendant lifecycles were
independently observed. Source was unchanged within both test runs. Between them,
only `service_pending_tests.rs` and `service_fixture_start.rs` changed: explicit
macro import, eight clones in match guards preserving equality assertions, and
two explicit `drop(guard.0.take())` calls consuming the test owner guard.

The initial source review also found a real first-cause ordering defect before
compilation. A later failed cancellation receipt could displace a known startup
failure, and a lost startup guard could replace the prior error. The corrected
stage retains the original real cause before external cleanup hooks, identifies
a spontaneous ClosedLease known before closing intent, and fills only a missing
guard failure. It does not normalize closure from a late flag alone. The original
stage is preserved in the archive. No executed failing baseline is claimed.
`known_startup_failure_precedes_later_cancellation_receipt_loss` and
`close_owner_loss_keeps_already_observed_startup_cause` passed with exact separate
operation/cleanup assertions.

The new service gates cover Preparing cancellation with a usable sibling and
replacement, reentrant release before control publication, actual destruction of
unpolled startup/close owners, request-hook panic with an independent functioning
drain route, failed cancellation routes with immutable uncertainty and a late
result, and known-error precedence over later receipt loss. Controlled service
fixtures prove service bookkeeping; they do not prove native OS-thread cleanup.
Their cancellation does not add constructor permits or replace real cleanup
assertions with boolean fixtures.

Exact captured sources, not the runner's historical `f2cc6c2…` annotation,
identify the tested tree. The passing test map and post-lint/format map remain
separate. The thirteen adopted source members were re-read from the preservation
archive and checked against their recorded hashes. Archive SHA256:
`245977dd9c9c38d7ea7a50965439896537a7017cbd74c14cb1075eb52b7a4703`.
It is a checkpoint inside this cloud filesystem, not proven external durability.
The archive also retains the original first-cause stage and review patches.

No whole-workspace compile, installed new worker, runtime-facade cancellation,
App Server/TUI adoption, fresh CLI/GUI build, or in-app Browser test is established
by these commands. Those gates are separate and pending for this slice. Service
initialization/shutdown and update/poll owner-loss guards remain outside this
change. The matching algorithm, index budgets, wire DTOs and established extraction
scope are unchanged by these thirteen paths. Updater completion is not claimed.
