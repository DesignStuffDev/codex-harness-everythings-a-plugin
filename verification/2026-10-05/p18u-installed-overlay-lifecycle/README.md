# Installed overlay lifecycle acceptance — 2026-10-05

The exact paused gate passed on the unchanged component manager. Five source/strict
stages cover eight normal commands (two setup and six recovery) plus two direct
installed-SDK primaries. All six tracked process/thread identities disappeared
before rescue; neither primary used rescue. This is bounded installed-runtime
proof, not a native build, native capacity pass, complete updater or P03 completion.

## Exact implementation and source identities

The existing externally built package **0.4.0** was installed into isolated test
state; its legacy overlay implementation is **0.3.0**. No host/plugin rebuild,
upgrade or removal occurred. Installed `plugin.pyz` SHA256:
`913e0842bfae1ec72e80c118612b8aed73575625ea13b7bd71d4bf073dd87a29`.
It was built from the prior 8,975-path `d84cf069…` source map. The repository stayed
unchanged at the distinct 8,977-path `f27ab9e9…` map. The manager remained `f054d84a…`.
[Exact bindings and receipts](EVIDENCE.json) retain complete hashes.

Inputs retain upstream base `d42056091aded7feb1d88ac7e83972108b2aa478`, custom source
`914cc59374c1149463e78bc33851d83e3f14d0a4` and later upstream
`2e5fea64eefcaa19f48458b2386011b619f69c70`. Recovery prepares a sparse 13-path overlay,
not a complete later-revision host or an activated update.

## Executed checks

| Stage | Actual result |
| --- | --- |
| Setup | Two normal commands: install exact retained package and list isolated installation; exit 0, exact package/config bindings checked. |
| Cooperative primary | Direct installed SDK receives protocol shutdown with real Git held at exec; plugin exits 0, cancellation is reported and job remains incomplete. No rescue; all three tracked identities absent. Git's own exit code was not observed. |
| Cooperative recovery | Offline inspection with empty PATH returns expected exit 2/incomplete; ordinary manager invocation creates a fresh prepared overlay; offline prepared inspection exits 0. |
| Abrupt primary | SIGKILL targets the installed plugin only; plugin exits -9, inner helper adopts/reaps Git with -9. No rescue or extra self-signals; all three tracked identities absent. Job remains incomplete. |
| Abrupt recovery | The same separate three-command recovery sequence passes, including a new successful normal-manager invocation. |

Each recovery independently compares all 13 output paths/bytes, preserves the
custom import deletion and retains the interrupted job. The manager reports
`directory_fsync_completed`; subsequent offline inspection verifies integrity but
correctly reports `not_attested_by_inspection` for durability. All seven release
gates remain pending; `update_allowed` and `activation_allowed` remain false.

The unchanged outer strict subreaper exited 0 with no runner error in all five
stages and reaped exactly its own command. This does not erase the abrupt primary
helper's explicit inner Git adoption. Root separately confirmed absence of all six
tracked identities; an unrelated or zombie process was not substituted as proof.
The source wrappers passed on the exact unchanged repository map. No timeout fired.

The real Git exec was traced, then all identities were detached before the primary
control action. The SDK shutdown reader is not the Git spawner. This demonstrates
the exec-boundary ownership case; it does **not** establish that Git computation or
merging progressed beyond exec. Direct SDK shutdown also does not test graceful
forwarding from the component manager. The two later manager invocations are
separate successful recovery evidence.

## Bounds, preservation and correction

Primaries retain 5-second admission, 5-second primary and 3-second cleanup budgets.
Outer timeouts were 60 seconds for setup, 20 for each primary and 140 for each
recovery, with TERM followed by KILL after 5 seconds if required; none fired.
Whole-cycle durable allocation was **6,950,912 bytes**, below the 8 MiB cap before
the small final receipt and this separate documentation checkpoint. Root's final
observation was 4,118,978,560 persistent free bytes and 8,825,147,392 hard-unused
memory bytes. These observations are not reservations or a native validation pass.

An initial shutdown metadata export found `RESULT.json` already occupied by the
wrapper receipt. Exclusive creation refused the collision: the wrapper was never
overwritten and the test was not replayed. The primary was correctly saved as
`PRIMARY_RESULT.json`; `EXPORT_CORRECTION.json` preserves the exact correction.
Static helper review corrections and all original execution receipts remain
preserved and separately hashed. No failed run has been relabeled.

Raw evidence lives under the existing VM's recovery root, including
`p18u-overlay-installed-orchestration-20261005-02/ROOT_ACCEPTANCE.json`, SHA256
`a5571914b90468a2060361dcc3910e9b2be6b8f78785de70e708ddb2a54602c0`.
[EVIDENCE.json](EVIDENCE.json) binds the orchestration, primary, wrapper, source,
strict and export-correction receipts without publishing private process/state
contents. VM-local artifacts are not an offsite backup.

No new GUI, native runtime, build, migration, walkthrough, release activation or
rollback test ran in this gate. No full-workspace result is claimed. The next step
is the critical native eight-path auth-install cohort under fresh phased capacity
and source-preimage checks. Collector and broader updater work stay paused.
