# Featured task receipt proposal (new runtime evidence only)

This is proposal02: an R-only layout correction to the preserved proposal01. No native source, checkout, old fixture,
runtime receipt, or runplan was changed. Root executed proposal01:57 passed/1 error because the flat layout omitted the unchanged historical-preimage assertion input. That failed run remains failed. Proposal02 has not been executed.
AST parsing and source/hash comparisons are the only local verification so far.

The old frozen held-host fixture consumes stderr without retaining raw lines.
Its normalized receipts cover curated/session/MCP events only. Its stderr report
contains bytes, SHA-256, EOF and errors, not featured events. Consequently this
proposal cannot postcheck old runs retrospectively. Fresh production acceptance
must use the separately reviewed derived fixture and its exact module pins.

## Files and original gates

- `proposed/held_production_host.py`: minimal derived collector adapter. Its original 89
  `require` call ASTs remain identical. Original deadlines, observer, strict
  runner, control flow, receipt checks and result interpretation remain intact.
- `collection-adapter.patch`: complete diff against `adapter-preimage/held_production_host.py`.
- `proposed/observe_descendants.py`: byte-identical all-mode-ack observer sibling, retained
  because the frozen fixture resolves it relative to its own file.
- `proposed/featured_owner_receipts.py`: bounded capture and separate postchecker.
- `proposed/test_featured_owner_receipts.py`: pure synthetic parser/artifact tests. The
  continuation test calls only `Receipts.consume`; it starts no drain or process.
- `INPUTS.json`, `REVIEW.json`, `COMMANDS.json`, `MANIFEST.json`: exact source pins,
  structural checks, commands for root review, and package file hashes.

The adapter adds one capture object beside the existing collector, feeds it each
stderr line and writes `CASE/featured-receipts.json` after existing drain-close
paths. `CASE/case-result.json` binds its hash. The existing matrix binds the case
report and original normalized receipts. The new sidecar binds the case, exact
native binary/source commit/tree, fixture/module identities, and original stderr
SHA-256/bytes/EOF. Root must hash fixture/module before and after dispatch against
this manifest, in addition to the existing native source/build bindings.

There is no raw stderr archive. The new collector stores allowlisted event
fields and fixed error codes only. Bounds: 16 MiB input excluding newline
delimiters, 1 MiB per line, 65,536 records and 128 matching events. An error/cap
stops only new capture, records one fixed failure and returns normally; the old
stderr drain and old receipt collector continue. Separate postcheck fails if
capture is incomplete. JSON artifacts are capped at 4 MiB each.

Ordinary unrelated plain text or malformed JSON-looking CLI text is ignored.
An event-name candidate with malformed target/fields is rejected. JSON with a
featured event or lifecycle target, including Unicode escapes, is checked for
malformation and duplicate keys. Matching fmt candidates require complete
key/value tokens with no malformed/duplicate suffixes. All typed integer and
Boolean checks distinguish bool from int. Every matching event must be clean;
an early invalid/unclean event cannot be hidden by later clean rows.

## New independent acceptance gate

For each of the fixed eight cases, the postchecker requires the original matrix
and case gates to have passed and their artifact hashes to match. It then requires
at least one clean process receipt AND one clean scope receipt, with bounded
ordered event records and exact source/stderr bindings.

Process event: `featured_warmup_process_shutdown_observed`, target
`codex_core_plugins::lifecycle`, version 1. All count fields are exact nonnegative
integers: `task_pending`, `task_joined`, `task_panicked`, `task_join_failed`,
`optional_fetch_failed`. Require pending/panicked/join_failed zero and
`task_ownership_clean` true. Optional fetch failure is allowed; joined zero is
allowed because process counts describe retained-scope snapshots, not historical
jobs. Multiple process snapshots are recorded, never summed as lifetime totals.

Scope event: `featured_warmup_scope_shutdown_observed`, same target, version 1.
Require `task_ownership_clean` true and either `Idle`/`None` or
`Joined`/`Some(Succeeded|Failed|Cancelled)`. Idle is reported separately and does
not prove any warmup started. Actual started/held warmup remains the independent
caller fixture gate. This proposal never establishes HTTP pool shutdown, MCP
custody, whole-host cleanliness, or complete extraction.

Native source anchors: `codex-rs/core-plugins/src/plugin_startup_process_shutdown.rs`
process emitter, `codex-rs/app-server/src/featured_warmup_lifecycle.rs` scope
emitter, `codex-rs/app-server/src/in_process.rs` embedded finish and
`codex-rs/app-server/src/lib.rs` stdio finish. Every fixed held path reaches the
appropriate scope finish; repeated executable wrappers can emit process snapshots.

Root reviews and runs proposed pure tests, rechecks existing fixture tests, and
then integrates the derived path into a NEW runtime dispatch. `COMMANDS.json`
contains templates only. It does not replace the current frozen runplan, approve
a completed runtime gate, or reinterpret previous successes or failures.

## Layout-only correction

All eight proposed Python files—including31 historical and27 new pure test methods—are byte-identical to proposal01 and now live together under `proposed/`. Historical `preimage/` is copied exactly from the all-mode-ack proposal (held fixture281173ec…, observer984dc4b6…). The direct adapter parent ac8ae90e… is separately retained under `adapter-preimage/`, because it is the actual patch basis. No ambiguous shared R/preimage was created. The unchanged test resolves its intended path and its89 requirement ASTs/policy constants match by read-only comparison.

Run the fresh source/strict-wrapped policy02 command in `HELD_OVERLAY02.template.json`; do not overwrite policy01 or claim an executed retry. Its matrix fixture path is `proposed/held_production_host.py`, and the collector sibling path is similarly nested. All native hashes still require the actual reviewed current production build binding.
