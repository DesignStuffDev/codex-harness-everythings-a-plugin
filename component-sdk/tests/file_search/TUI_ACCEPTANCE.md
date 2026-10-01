# Real TUI installed-search acceptance

Status: Python syntax validation and five terminal-observer helper tests passed.
**No Codex binary, manager, worker, PTY interaction, Rust command or server has
been run by this staging worker.** Product behavior below remains an acceptance
candidate until the parent applies the frozen TUI source, builds the final full
CLI, freezes it and executes this runner. It deliberately fails on an unsupported
terminal mutation, unexpected screen, startup rejection, process leak or deadline;
it does not silently skip/retry an interaction.

## What the runner will exercise

1. Start the actual frozen full `codex` executable with `--no-daemon
   --no-alt-screen --cd <fixture-root>`. Use a new private CODEX_HOME and trusted
   fixture roots, file auth storage, no-auth custom model provider at loopback
   port9, no model discovery/analytics/updater. No model turn is submitted and
   there is no server or protocol substitute. Child environment is an explicit
   noncredential allowlist; no current auth or environment dump is read.
2. Wait for the actual StartupThreadStarted event in the opt-in TUI session log
   and visible composer. Type `@p02alpha`, require the current terminal screen
   contains a fixture filename suffix never typed, and reject its gitignored peer.
   Clear only the known ASCII draft with backspaces before any Enter key.
3. Exercise a second query and actual `/cd` to the other trusted root; wait for
   the completion message, then require the second-root filename and absence of
   the first-root-only filename. This is a real within-process cwd transition,
   **not reconnect**, and it does not change the provider's immutable process cwd.
4. `/quit` uses the real TUI command and must exit0. No quit signal is sent to
   the worker or whole process group on a passing run. Require every observed
   descendant PID gone (including zombies), actual session_end and search events, and absence of any recorded outbound
   `AppCommand::UserTurn`. A dead model URL alone is not treated as proof that no
   turn was accidentally submitted.
5. Install/select the actual independent03 package through the real frozen
   component manager, verify installed immutable bytes, park the install-input
   copy and repeat interactions using the exact same frozen host hash.
6. Require exactly one installed-worker PID/start identity before the first query,
   through both queries and `/cd`. Briefly SIGSTOP **only that selected worker**,
   type a fresh unseen beta query, require no beta result during a0.6-second
   observation interval, SIGCONT in `finally`, then require its real result using
   the same worker. This causal check is stronger than a coincident listening
   process: the actual picker waits for that selected worker. It does not inspect
   in-memory Arc identity or constitute a proof about every future schedule.
7. Supply an unsupported option to that real native worker in only the fixture
   catalog config. Require explicit nonzero startup exit and its real bounded
   rejection cause, without reaching a started interactive session. No selected
   provider failure may silently choose the native fallback. Restore the catalog
   config even after failure.
8. Remove selection/package via manager, require activation removal, rerun native
   TUI interactions, and verify package objects, original independently built
   package and both frozen host binaries remain byte-for-byte unchanged.

A failed case preserves raw PTY bytes, current-screen snapshots, event logs,
commands and process identities. Emergency SIGKILL is attempted only after the
case has already failed; its report is permanently a failure, never normal
shutdown proof. A paused worker is resumed before any failure cleanup.

## Prerequisites and invocation

Parent owns applying/compiling/testing all Rust stages and freezing the final
full CLI. The earlier build proof names a **standalone file-search CLI**; this
runner does not relabel that artifact as the final full CLI. Supply a new explicit
full-CLI SHA256 plus the root's matching successful build-source report and a
small frozen-artifact binding envelope. The source report must show finished,
returncode0, unchanged nonempty before/after source fingerprints, a cargo build
of `-p codex-cli`, and include the actual App Server/TUI composition source.
The independent03 worker package can remain unchanged: it was built separately
and has its own source/lock/package verification chain.

Use the existing unchanged subreaper runner; its successful report is mandatory
in addition to `tui-acceptance.json.passed`. The subreaper collects adopted
orphans, reports command status and refuses successful descendant leaks. Use a
fresh work/report path each time and retain the original failed runs.

```text
python /workspace/codex-harness-everythings-a-plugin/component-sdk/tests/subreaper_runner.py \
  --report /workspace/acceptance/p02b-tui-installed-01.subreaper.json -- \
  python /workspace/codex-harness-everythings-a-plugin/component-sdk/tests/file_search/tui_acceptance.py \
  --repo /workspace/codex-harness-everythings-a-plugin \
  --cli FULL_FROZEN_CODEX_PATH \
  --cli-sha256 FULL_FROZEN_CODEX_SHA256 \
  --host-source-report FULL_CLI_SOURCE_EVIDENCE_PATH \
  --host-build-binding FULL_CLI_FROZEN_ARTIFACT_BINDING_PATH \
  --manager FROZEN_COMPONENT_MANAGER_PATH \
  --manager-sha256 FROZEN_COMPONENT_MANAGER_SHA256 \
  --build-report /workspace/acceptance/p02b-search-independent-03/independent-build.json \
  --work-dir /workspace/acceptance/p02b-tui-installed-01
```

Create the binding while freezing the actual build output (no source/host rebuild
is performed by the acceptance script). Its exact schema is:

```json
{
  "schema_version": 1,
  "artifact_kind": "codex-cli",
  "binary": {"path": "/absolute/frozen/codex", "sha256": "actual binary SHA256"},
  "source_report": {"path": "/absolute/full-cli-build.source.json", "sha256": "actual report SHA256"}
}
```

The runner checks the envelope against actual supplied binary/report paths and
hashes, parses the successful build/source receipt, and retains all fingerprints.
The envelope is the build owner's provenance assertion; recording a random JSON
file or the older standalone-CLI report will not pass. Root still owns correctly
freezing the artifact produced by that successful build.

The placeholders represent artifacts parent has not yet supplied, not executable
paths invented by this stage. Use absolute paths. No build, install dependency,
network listener, Git operation or external publication runs from this script.
Manager operations affect only the newly created fixture CODEX_HOME.

## Evidence and practical limits

- Each assertion uses current terminal cells reconstructed from cursor/erase/scroll
  operations. Raw transcript files are retained for independent audit. Five helper
  tests check split ANSI highlighting/query negotiation/stale erasure; they are
  observer validation, **not five Codex runtime tests**.
- Fixed40×140 terminal, ASCII filename assertions. Unicode cells are handled enough
  to align ordinary UI, but the observer is not a complete Unicode terminal
  emulator. Unsupported screen mutations abort acceptance. This is not a browser,
  visual design or accessibility review.
- PTY process sampling follows actual PID/start identities and executable paths,
  never command lines or environment. It cannot promise to sample every extremely
  short descendant; the unchanged outer subreaper adds the terminal descendant
  drain check. Do not accept merely absent known worker PIDs when the outer report
  fails.
- No model turn, streaming/auth/approval/tool execution, real remote transport
  reconnect, daemon mode or desktop GUI/Browser exercise is claimed. Existing
  facade/unit regressions remain necessary for race coverage. Browser validation
  of the separately installed GUI remains a separate workstream.
- `/cd` is sourced from the actual supported TUI command and may expose a genuine
  existing startup/working-directory bug. Do not weaken a failing assertion into
  two fresh launches while continuing to claim within-session root replacement.
- Per-lease Preparing cancel+wait and pre-existing early App::run `?` joined-server
  cleanup remain explicit source gaps; this normal-path runner does not close them.
- Runtime evidence directories are cloud-local until explicitly backed up or published;
  a local checksum is not an external durable backup.

## Source audit references

Existing terminal query and PTY patterns: `codex-rs/cli/tests/worktree.rs` and
`daemon_startup.rs`. The runner answers cursor/primary attributes/keyboard flags
and foreground/background queries incrementally. `/cd` completion and picker
root reset: `codex-rs/tui/src/app/working_directory.rs`. Normal `/quit` dispatch:
`codex-rs/tui/src/chatwidget/slash_dispatch.rs`. Exact log evidence uses existing
`CODEX_TUI_RECORD_SESSION` and frozen staged `file_search_ready` records. Package
proof and descendant checks reuse `component-sdk/tests/file_search/acceptance_support.py`
read directly from the supplied repository; its digest is saved in the report.

## Independent static review corrections

The original candidate is preserved at
`/tmp/p02b-tui-installed-acceptance-review0` in the originating cloud executor. Review corrected descendant emergency
cleanup even after direct CLI exit, the SIGCONT/exit race, ED3 scrollback semantics,
explicit no-UserTurn evidence and the full-CLI build binding. These fixes have
only syntax/helper-test validation so far; no runtime pass is inferred.

The durable source location is `component-sdk/tests/file_search/`. The staging
paths above record historical review evidence and are not runtime dependencies.
Use the adjacent scripts after checkout; `--repo` identifies the same repository.
