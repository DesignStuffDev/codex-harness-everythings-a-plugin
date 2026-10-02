# Current production attachment proposal 02 — not executed

R-only fixture proposal; no native or checkout edits, imports of tested code,
tests, strace probes or production commands have been executed by its authors.
The preceding audit is `../p03-current-attachment-gate-audit-01`.

Sealed proposal01 and its failed production run remain unchanged. That run
stopped at native package installation before any image case: existing catalog
validation rejects two enabled providers without an explicit selection. The
unchanged trace parser and its 13 pure tests passed separately under root; those
exact bytes and source/strict/log receipts are pinned for reuse, not rerun here.
Proposal02 changes only orchestration and adds installation/removal evidence.

`attachment_production_acceptance.py` uses current pinned CLI/manager binaries and
the exact preserved independently built native inline package. Its historical
report and every package file must match before reuse. No fresh independent
native build is claimed. Two small trusted Python fixture plugins are built
separately through the existing SDK in a fresh external directory. Their source
directories are preserved under `.parked` names before installation; packages
carry their own SDK and have no runtime source-path dependency.

The ordered cases are:

1. Build/install the model and counted fixture; run unselected builtin image
   baseline through real `codex exec --image`. Select counted before installing
   the native competitor. Verify original immutable objects survived installation
   and snapshot all installed objects before switching to the native selection.
2. Selected native inline package through that same actual image route, under
   private process/exec/open/exit tracing. Require exactly one successful exec of
   the immutable installed native ELF, a successful read-only staged input open
   by that process or a traced post-exec `CLONE_THREAD` descendant, native leader
   exit zero, model-facing inline bytes equal to baseline and no native upload
   fallback warning. The independent package is not wrapped or modified.
3. A separate counted test component returns a synthetic file ID; require one
   upload and that exact reference reaching the model probe.
4. Cold CLI resume with no new image retains the same internal thread/reference
   without another custom upload.
5. The counted component returns typed backend failure; require its counter,
   a positive upload-fallback warning and model-facing inline bytes equal to the
   prepared staging bytes recorded by that fixture.
6. Remove the now-unselected native provider before resetting the remaining
   counted selection. Require the exact registry delta, preserved counted
   selection, immutable native object and unchanged native state directory/files.
   Fresh image turn returns builtin inline behavior with the custom counter
   unchanged. Removal retains cached objects/state; no garbage collection runs.

These are real CLI/engine/component runs with deterministic model inference, not
a live provider. The synthetic file ID is valid only for this test model. Neither
the native inline contract nor this proposal creates a production resolve caller.
App Server structured thread-attachment metadata remains a separate contract.

Each command uses the existing `OwnedProcess` and unchanged strict subreaper
runner, with the default five-second descendant drain. A 300-second fixture
command timeout is an outer failure/cleanup limit, not a modified host deadline.
Emergency cleanup is explicitly recorded and cannot produce a passing gate.
Independent cases continue after a native trace failure only if command quiescence
has already been confirmed; any cleanup uncertainty aborts the remaining cases.

Root separately verified tracing its own `/bin/true` subprocess (exit zero);
`../p03-current-attachment-trace-capability-01/RESULT.json` is pinned in INPUTS.
This is not actual plugin tracing or production acceptance. The trace gate
fails explicitly if unavailable, changed, malformed, incomplete or
over its bounds. It never converts a missing trace into a native-package pass.
Trace stdout/read/write data and environment contents are not requested. Raw
traces can nevertheless contain synthetic argv and owned paths, so they stay
private beneath the mode-0700 fresh work directory. Public reports retain only
allowlisted counts, booleans, enums and hashes. The pure parser is bounded at
128 files, 16 MiB total and 100,000 lines; ordinary command logs are checked
against an 8-MiB post-run limit. This is not a hard disk quota.

Native `Reply::Error` can still be followed by process exit zero. Accordingly
this proposal explicitly reports **upload-path invocation plus no observed error
fallback**, not inspection of the native result envelope. The typed-error case
is the positive control for the focused private warning filter. No HTTP pool,
MCP, every-descendant, GUI or whole-host cleanliness claim is made.

The helper sources, proposal files, historical report, native package and host
binaries are checked before/after. Fresh output is required; nothing in a prior
fixture/home is reused. Root must retain exact source-to-ELF build evidence and
check this manifest before dispatch. The existing source wrapper should surround
the actual production invocation. `COMMANDS.json` contains unexecuted commands
for root review, not permission to replace earlier evidence. Pure-test reuse is
limited to identical parser/test bytes; it does not validate the new orchestration
or substitute for fresh production02 execution. Native state may contain zero
files; a retained empty directory is not evidence of substantive durable data.

This directory is an in-workspace proposal, not an externally durable backup.

Copied `HISTORICAL_PROPOSAL01_ROOT_*_DISPATCH.json` files preserve original
dispatch records only. They must not be used to dispatch proposal02; its fresh
command is in `COMMANDS.json`. No new root dispatch record is claimed.
