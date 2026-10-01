# P02B runtime Preparing cancellation evidence

The corrected focused gate passed **55/55 tests, zero skipped, zero retries**:
44 runtime tests, five plugin-library tests and six real worker-process integration
cases. This adds owned Preparing control to the existing file-search runtime;
it does not extract another subsystem. Source is preserved on cancellation WIP `272993ff0e6376e7ba52403e3b52d7f16449cd16`; accepted main runtime is unchanged.

`FileSearchScope::begin_open` synchronously reserves a start and returns
`PendingFileSearchStart`, its exact `SearchStartControl`, and a passive `finish`
observer. Existing `open` remains convenience. Startup publishes the backend
control before awaiting its result and forwards latched cancellation outside
locks. Local skipped admission has an explicit origin. Original finish and
backend cancellation remain independently owned; the startup task never awaits
the facade close that must join it. Guards exist before spawn. Known cleanup
refunds only after local startup/callback/update drainage; lost proof retains
quota and immutable Unconfirmed receipts. Exact first cause propagates to
lease, scope and provider ledgers before external cleanup hooks.

| Gate | Actual result |
| --- | --- |
| `p02b-runtime-preparing-tests` | Compilation failed, exit101; no tests executed. Fifteen ambiguous new-test `assert_eq` diagnostics and two unused test-guard assignment warnings. |
| `…-tests-02` | 55 executed: 53 passed, two original cause-race assertions failed, exit100; no retries or skips. |
| `…-tests-03` | 55/55 passed, exit0; no retries or skips. |
| `…-fix` | Clean scoped lint, exit0; source unchanged. |
| `…-format` | Exit0; 19 mechanically formatted paths, including seven consumer fixtures outside this test scope. |

All three test commands ran through the unchanged strict subreaper with no runner
error. Each captured 10,139 unchanged source fingerprints during execution.
The failed runs remain preserved separately. Their command processes were reaped
with101 and100; the final command was reaped with0. No broad claim about every
possible descendant or whole-host lifecycle follows from those exit records.

The first correction added explicit macro imports to two new test modules and
used `drop(guard.0.take())` twice in the controlled pending fixture. The next run
then exposed a real production observer defect in both
`close_before_startup_observer_resumes_preserves_the_cleanup_cause` and
`close_during_pending_startup_preserves_the_cleanup_cause`. For operation=Ok and
Unconfirmed cleanup, the observer returned synthetic ClosedLease instead of the
TransportLost cleanup cause when no original startup error existed.

Only `pending.rs` changed between53/55 and55/55. The corrected fallback chooses a
retained operation error first, then an original startup error, then an
Unconfirmed cleanup cause; only proven cleanup with no error synthesizes
ClosedLease. The canonical close receipt retains its operation=Ok. No assertion
or test source was changed for this fix, and the original cause-race source hash
is identical in both reports. The earlier independent static review had missed
this defect; it is preserved as review counterevidence.

All eight new runtime tests passed. Six use the real native backend: abandoned
unpolled ticket and finish, ready-before-public-observation cancellation with
sibling and replacement matching, actual snapshot-budget exhaustion, actual
runtime destruction before native admission, and cancellation before control
publication. The publication-race case allows NotAdmitted or Confirmed according
to actual admission timing; it does not prove native worker creation in every
schedule. Two controlled error-boundary cases preserve a spontaneous ClosedLease
through a distinct delayed cleanup error and a known resource failure through a
panicking cancellation hook in all ledgers. They do not prove OS worker cleanup.

The six separate real-worker cases passed: native scoring/ignore/highlight parity,
index exhaustion with joined cleanup, sibling isolation and quota reuse, exact
relative roots under separate provider working directories, selected-config
failure without fallback, and removal affecting later selection while an existing
provider stays owned. These reuse the integration build/test fixture; they are
not a new outside-harness package build against an unchanged frozen full host.

Source identities come from exact report maps, not the runner's historical
`f2cc6c2…` annotation. The nine authored runtime files and five coordinated runtime
fixture files have separate adopted, failing-run, passing-run and final-format
hashes. All21 archived source members, including seven other consumer fixtures,
were verified against adoption hashes. Archive SHA256:
`7d1de79cee23f4de26a4939da5f00ba961cc8ab6c05d56ff325ab90e2d15c424`.
This is a cloud-filesystem checkpoint; the final source is also on the WIP commit above. The archive and intermediate preimages remain cloud-local.

The seven AS/client/TUI fixture files are not compiled or tested by these gates.
Consumer production integration, fresh CLI/App Server/TUI hosts, GUI behavior and
in-app Browser checks remain separate. Scope/provider completion-guard generic
lost-error fallback and an exact loss-after-local-skip-before-close-poll schedule
remain outside the lease tests. No upstream updater or whole-harness completion
is claimed. Tests were not rerun solely for formatting.
