# Native lifecycle observation stage1

Eight adopted paths add copy-only worker/callback lifecycle snapshots and six meaningful
ownership tests. Existing owners retain exact handles; observation cannot request stop, join,
reopen admission or clear quarantine. Finished-but-unjoined is explicitly distinct from Joined.
An explicit semaphore timeout failure corrects the original staged replay-test weakness.
This is a P03 prerequisite, not extraction of another installed subsystem.

Scoped `just test -p codex-core-plugins --lib` passed542/542, zero skipped and no warnings.
The unchanged strict runner exited0 with no drain error. Its adopted128 and SIGKILL statuses
remain unassigned to a cause; no claim is made that every child exited normally.
[Exact receipts and source delta](P03_OBSERVATION_STAGE1_EVIDENCE.json) bind8,925 files and
map `6f11c7510a48fe24f500abca6fa90364b70fff79c96d1d9c781473a1b353b72d`.
[Lineage](../../upstream/p03-curated-observation-lineage.json) records all eight paths and
semantic update hazards against the f8a5905 checkpoint.

Scoped lint/global formatting remain pending. The next gate is a new App Server library child
plus parent-driven same-process A→B replacement; it has not run. Current58 production binaries
are historical and cannot establish these new APIs' runtime behavior. In-app Browser is unavailable;
no fresh GUI runtime result is claimed for stage1. The prior source-bound GUI result is preserved.

The separately staged outer-observer acknowledgment fix passed15 pure fixture checks with
strict0/null, unchanged proposal bytes. It has not run against a rebuilt production host.
All original held matrix failures remain; the complete eight-case matrix is still open.

Stage1 recompilation reduced overlay headroom to roughly245MB. Reaudit before another compiler;
do not delete source, archives or frozen runtime, or rerun historical cache retirement lists.
Stage2 fixture review also identified ambient SQLite-root leakage; isolate and check the test
root before initializing state. These pending corrections are not adopted by this checkpoint.
Main remains unchanged; P03 extraction/whole-host gates and P18U update/rollback remain open.
