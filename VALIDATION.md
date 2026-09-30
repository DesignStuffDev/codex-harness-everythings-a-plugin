# Validation record — 2026-09-30

The installable component foundation, selected model transport, tool/context
adapters, native storage packages, and an independent GUI have working runtime evidence. This is a
partial compartmentalization of the actual Codex engine. It does not establish
replacement of every provider or functional subsystem; the remaining contracts
are listed in [COMPONENTS.md](COMPONENTS.md).

**Current source checkpoint:** storage contract 2 corrects the confirmed
version-one state-fidelity defect: the native-versus-installed-process raw
persistence comparison now passes in Legacy and Paginated modes. The
[original failing regression](verification/2026-09-30/storage-wire-fidelity-before.json)
remains retained. All 27 distinct initial fidelity cases passed across two runs;
the later native maintenance gate passed all 653 distinct cases across its main
run and a corrected fixture rerun, with clean final scoped Clippy. This is not a
claim of one all-green combined run. The latest host gate passed all 56 cases
together, including presentation service lifetimes and persistent reaping. The
host rejects storage contract 1 before activation.

The shared state codec is a domain serialization library with 37 passing codec
and three trusted-helper tests. It preserves in-memory state before native store
policy applies; ordinary provider and disk-read validation stay unchanged. It
does not establish activation of context/replay workers or a new independent
subsystem. [Storage wire design](component-sdk/STORAGE_WIRE_V2.md) describes that
trust distinction.

Native migration/compression now have owned cancellation and join lifetimes.
A real installed native process test blocks migration using a SQLite write lock,
proves close remains pending, then verifies canonical history/projection and
cleanup after releasing the lock. Compression's 140-test native suite includes
four new cancellation/join tests. Ordinary maintenance operation failures retain
native warning policy; panic/join failures prevent confirmed shutdown. Process
reaping alone is not a durability guarantee, and deadlines may still report
unknown completion.

**Storage-v2 external checkpoint passed:** the native package was built outside
the checkout, its source removed, then installed and used by the real CLI and
GUI with unchanged host binaries. Two manager Ctrl+C shutdowns exited zero with
all tracked PIDs absent; a cold restart recovered history and completed a new
turn. This is separate from the blocked-maintenance test above. Earlier
**contract-1** reports remain historical evidence and are not relabeled as v2.

The selected model transport also has a reproduced size limit: native HTTP
accepted 4,552,986-byte history and 4,844,921-byte image requests that the v1
component rejected at its 4 MiB frame cap. No request reached that plugin and no
native fallback occurred. [The actual CLI comparison](verification/2026-09-30/model-wire-size-audit.json)
uses preserved checkpoint binaries; chunked model v2 remains proposed.

Checks ran in the selected Linux cloud checkout of
`DesignStuffDev/codex-harness-everythings-a-plugin`, based on official OpenAI Codex
`d42056091aded7feb1d88ac7e83972108b2aa478`. Source provenance and upstream notices
are retained. Publication records are separate from runtime evidence; no official
desktop installation was changed. See [recovery checkpoint](RECOVERY.md).

## Recovery revalidation

The recovered source and both worktrees were archived before changes; see
[RECOVERY.md](RECOVERY.md) for archive scope and the distinction between cloud-local
backup and externally published source.

The corrected complete `codex-core --lib` run **passed all 2,694 executed tests**,
zero failures/errors/retries, with one helper excluded. Run
`30e4ea84-f664-40f8-ba22-fac95cb27b80` took 246.369 seconds after a 152-second build.
[Terminal result](verification/2026-09-30/recovery-core-lib-portable-20260930.json),
[JUnit](verification/2026-09-30/recovery-core-lib-portable-20260930.junit.xml),
[log](verification/2026-09-30/recovery-core-lib-portable-20260930.log), and
[subreaper result](verification/2026-09-30/recovery-core-lib-portable-20260930.subreaper.json)
retain exact evidence. The actual tested 8,600-file Rust/SDK source manifest has
SHA-256 `38a866549659cca98219fa6c3e28d56de2ebb3104add642a1f7c043900188899`.
The report corrects an older hardcoded wrapper source label; result data is unchanged.

The [first run](verification/2026-09-30/recovery-core-lib-20260930.json) remains
recorded as 2,690 passed/four failed. Two test-only DNS/proxy fixture corrections
and init-style orphan reaping address those failures. The strict process-absence
assertions and production behavior remain unchanged and pass in the complete
rerun. The wrapper's three real-process tests also passed. This is not a claim
that Codex can itself reap arbitrary grandchildren under a non-reaping PID 1.

Fresh [CLI/storage and attachment recovery](verification/2026-09-30/recovery-runtime-summary.json)
and [GUI lifecycle acceptance](verification/2026-09-30/recovery-gui-acceptance.json)
passed using the unchanged accepted host binaries. The GUI check includes actual
manager Ctrl+C during an active turn, cold recovery, a further completed turn and
a second manager Ctrl+C; all tracked manager/gateway/server/store/model PIDs were
absent afterward. Three separate manager signal cases passed and retain the forced
shutdown durability warning. These are actual engine/process checks with fixture
inference, not live-provider testing. The in-app Browser is unavailable; the
manual UI driver used Chromium/Playwright. No supported cloud-to-local preview
route is exposed. Screenshots are retained with the GUI evidence.

Final `just fix -p codex-core --lib` passed without warnings in 9m03s;
`just fmt` and `git diff --check` passed. The [final checks](verification/2026-09-30/recovery-final-checks.json)
record source changes relative to the tested manifest. No runtime rerun solely
after lint/format is claimed.

The complete workspace gate remains disk-blocked, as described below. Newer
isolated implementation source is not covered by this core or runtime result.

## Focused checks

Rust commands use the repository's `just test` entrypoint. Counts below describe
completed focused runs; a successful build alone is not counted as runtime
coverage.

| Check | Recorded result | Behavior exercised and limits |
| --- | --- | --- |
| Shared component state codec and trusted protocol helpers | 37 codec and 3 helper tests passed; scoped Clippy clean | Lossless native state, precise numbers, provenance, native paths, custom variants, null/presence distinctions and preservation of ordinary provider distrust. [Results and retained artifacts](verification/2026-09-30/component-state-codec-results.json). Linux only; this is not context-replay activation evidence. |
| Storage contract v2 fidelity and regression gate | All 27 distinct cases passed across two runs | Initial run passed 26; one new test supplied a native-forbidden pending metadata field, and the corrected case passed separately. Includes raw durable evidence parity in Legacy/Paginated modes, pending budgets/custom variants, Unix paths, timeline mirrors, and earlier actual-process lifecycle/recovery cases. [Exact split results](verification/2026-09-30/storage-wire-v2-regressions.json). The later maintenance gate below covers owned background work; independent v2 acceptance passed separately below. |
| Owned native maintenance and storage-v2 regression gate | All 653 distinct cases passed across main run plus corrected-case rerun; final scoped Clippy clean | Main run `8309963e-60b5-47cc-8370-1e5520c4fa39`: 652 passed, one new fixture lacked native metadata indexing. Corrected setup passed in run `31d33d5a-4f0d-4d75-a81c-f6ed1c93dc20`; production unchanged by the correction. Includes rollout 140, state 213, thread-store 272, component 12 and native-plugin process 16 cases. Actual migration is held by a SQLite lock while close waits, then drains and cleans up. [Exact results and limits](verification/2026-09-30/storage-maintenance-regressions.json). No all-green combined rerun is claimed; fresh independent CLI/GUI evidence is recorded separately below. |
| Presentation service lifetime and latest complete host gate | 56 passed, 0 failed, one helper-only skip; run `ceff8efc-0a99-422f-9f84-31b34464000c`; scoped host/API Clippy clean | Bounded initialization/request delivery, no ready-service execution deadline, full bounded exit grace, and ordinary invocation deadlines retained. Includes the prior persistent reaping cases. Three actual manager OS-signal cases then passed with an unchanged rebuilt binary and direct-child PIDs absent. [Source gate](verification/2026-09-30/component-host-service-lifetime.json), [actual manager cases](verification/2026-09-30/manager-service-lifetime-acceptance.json). Protocol fixture signal tests are separate from the actual GUI/storage checks below. |
| Persistent startup/error reaping and host compatibility gate | 52 passed together; one helper-only skip; run `90daff09-d81e-4078-a28d-b839f2871e04`; scoped Clippy clean | Returned handshake/driver/deadline/exit failures follow direct-child reap; cancelled startup retains a cleanup owner; worker join errors survive cleanup. Also rejects storage v1 before activation. [Exact results](verification/2026-09-30/component-host-persistent-reap.json). Cleanup exceeding its bound reports reaping/joins unconfirmed; abandoned startup requires the runtime to stay alive. Reaping alone does not prove durability, background maintenance completion, legacy codec worker ownership or Windows behavior. |
| Historical host gate (superseded by the latest 56-case gate above) | 46 passed together, one ignored helper, run `5d48e579-2655-4790-a57d-e773c9fd1cca`; scoped host/API Clippy passed | Handshakes, streams, bounded/malformed frames, cancellation, deadlines, Unix descendant cleanup, sensitive stderr suppression, catalogs, immutable package retention and Debug redaction. Persistent checks cover 17 MiB payloads, accepted writes, abandoned waiters, deadlines without replay, fork cleanup priority, terminal crashes, graceful draining, cancellation-safe server reads, and synchronous closure despite retained clones and abandoned waiters. The ignored Linux parent-death helper runs inside its passing nested-process regression, which kills an intermediate owner while handlers are blocked in separate process groups. Graceful control preserves partial frames and fixes its deadline; explicit forced termination waits for direct-child reaping. [Historical force-and-reap evidence](verification/2026-09-30/component-host-force-reap.json). |
| `just test -p codex-component-adapters` | 5 passed | Independent native-executor tool calls, output limits/errors, cancellation, bounded context/lifecycle values, and explicit replacement selection. |
| `python3 -m unittest discover -s component-sdk/tests -p test_sdk.py -v` | 14 passed again in 1.589 seconds during SDK distribution validation; [log](verification/2026-09-30/sdk-wheel-sdk-tests.log) | Independently built zipapps after removing source access, actual child protocol, tools/context/model events, template/build, static assets, error hygiene, shutdown, executable entrypoint, and per-invocation response identities. |
| `python3 -m unittest discover -s component-sdk/tests -p test_desktop.py -v` | 12 passed | Packaged GUI gateway with a deterministic app-server fixture: streaming, approval correlation, interrupts, recovery, distinct turns, origin/token/body limits, static assets, home/workspace forwarding, and signal cleanup. Later cases cover delayed cleanup beyond the old three-second deadline, bounded forced shutdown, nonzero/dead child errors, a real client TCP reset after an accepted action without replay or a second response, and preserving engine-side failures. [Latest Python evidence](verification/2026-09-30/desktop-client-disconnect-python.json). This row uses a fixture app-server. |
| `just test -p codex-core --test all component_model` | 4 passed together, run `9f04a2e6-8f0f-4e16-87a0-71e150e2515c` | Actual engine with external model processes: streaming and native tool execution, persisted resume/native restoration, terminal plugin failure without fallback, cancellation/recovery, and isolation preserving explicitly injected extensions. See [tests](codex-rs/core/tests/suite/component_model.rs). |
| Focused existing `codex-core --test all` regression suites | 28 passed, run `98d78499-1a5e-4e1e-acac-048d91ea401b` | Native HTTP/WebSocket request interceptors, abort lifecycle/tools/history, startup cancellation, session resume variants, compaction resume/fork, stream retries and usage validation, HTTP/WebSocket turn state, stream failure/early closure, and network recovery. |
| `just test -p codex-login component_auth` | 6 passed, run `5624fa02-ad7d-4191-88b6-779d69908bc8` | Actual child credential acquisition and refresh through native AuthManager, ChatGPT account ownership, native login restrictions, sanitized error/Debug output, deselection restoring native credentials, and workload-identity rejection. An earlier disk-exhausted compile was retried after disposable cache cleanup. |
| `just test -p codex-login --status-level fail --final-status-level fail` | 249 passed, 0 skipped, run `b0a14dd7-03c2-499d-855e-8e3a7ae5c449` | Complete login crate regression run including the six component tests; [retained results](verification/2026-09-30/login-results.md) and [JUnit](verification/2026-09-30/login-junit.xml). |
| Attachment API/inline/facade/component crate checks | 12 passed, 0 skipped; scoped Clippy passed | Four retained facade checks plus eight adapter/staging checks for payloads above 4 MiB, original-byte preservation, private staging cleanup, error redaction, URL lifetime validation, process cancellation and cancellation before/between staged writes. [Retained log](verification/2026-09-30/attachment-component-results.log). Production currently consumes upload only; resolve is adapter-tested. |
| Integrated `codex-core --test all` component and recovery gate | 18 passed, run `a2dbb1f0-541f-40f7-b8b6-3f1042ecdbc7` | Four model tests, four attachment tests, eight native resume tests, and two compaction/resume/fork tests. Attachment checks exercise real image input, restart without re-upload, explicit injection precedence, isolated-session defaults and native inline fallback after upload failure. [Retained log](verification/2026-09-30/core-components-and-resume.log) and JUnit accompany the run. The later 24-case engine gate below adds selected process-storage composition. |
| Historical storage-v1 `codex-core --test all` component, persistence and recovery gate | 24 passed, 2,211 excluded by filter, 0 retries, run `131d4160-e9cf-4b46-8fc0-5c1a965b072a` | Repeats model, attachment, native resume and compaction/fork checks, then exercises installed native storage through actual engine turns in Legacy/Paginated modes, cold process recovery, reference forks, exact history parity and resumed inference through the built-in store. Also covers selected-storage startup failure without fallback, ephemeral prompt debugging and cancellation while a thread-stop hook is stalled. [Exact command and cases](verification/2026-09-30/core-native-storage-final.json), [log](verification/2026-09-30/core-native-storage-final.log) and [JUnit](verification/2026-09-30/core-native-storage-final.junit.xml). This is a filtered engine gate, not the complete core suite. |
| Core registry/context/delegate library gate | 36 passed, run `4e705ca3-86e1-4406-ab09-8410ac570158` | Twenty registry tests, fifteen contextual-message tests and isolated attachment delegate behavior; [exact command and results](verification/2026-09-30/core-component-gates.md). |
| `just test -p codex-thread-store-local-plugin -p codex-thread-store-component --test-threads 2` | 10 passed | Nine installed native-process cases and one controlled cancellation case; [retained results](verification/2026-09-30/storage-component-results.json). Full history/recovery, large payload, lazy discard, fork leases, delete conflicts and collection operations. Engine composition validation remains separate. |
| Historical storage-v1 lifecycle and native library/binary packaging | 14 passed, 0 skipped, run `768ed842-cbff-4715-b807-6cfb30354af6` | All ten earlier cases plus invalid-open cleanup, cancelled initialization, cleanup-timeout uncertainty, and synchronous shutdown with retained clones. [Results](verification/2026-09-30/storage-lifecycle-regressions-results.json) and JUnit retained. The final storage, engine and app-server gates supersede this intermediate run. |
| Historical storage-v1 gate after acquisition/owner fixes | 17 passed, 0 skipped, 0 retries, run `8a7cc975-2870-480e-98f1-2bc81de18a2a` | All prior process cases plus actual delayed native create/resume replies, whole-store fencing without replay, cold recovery after releasing the writer, wrong-reply fencing and healthy handling of confirmed native errors. The generic owner also closes with retained handles and an unpolled shutdown future. [Exact results and cases](verification/2026-09-30/storage-final-regressions-results.json). |
| `just test -p codex-thread-store -p codex-state --test-threads 2` | 477 passed, 0 skipped | Existing native storage/state regressions after interface and serialization changes. Retained logs and JUnit are in `verification/2026-09-30/storage-native-regressions*`. |
| `just test -p codex-app-server --lib --test-threads 2 --retries 0` | 382 passed, 0 skipped, run `00a47a5e-7559-432f-b257-864a3a69a473` | Complete app-server library test binary, including five lifecycle cases for aborted runtimes, cancelled client shutdown, retained storage handles, stalled processor/store cleanup and combined primary/cleanup failures. Those five cases use a controlled store probe; real native-process behavior is covered by the storage and engine gates above. [Exact command and results](verification/2026-09-30/app-server-lifecycle-library-results.json), [log](verification/2026-09-30/app-server-lifecycle-library.log) and [JUnit](verification/2026-09-30/app-server-lifecycle-library.junit.xml). This does not include the separate app-server integration-test binaries. |

The initial complete Python SDK discovery run passed 23 tests; [log](verification/2026-09-30/sdk-all-results.log). The later graceful-shutdown run passed 30 tests (14 SDK, ten desktop, six exporter); [record](verification/2026-09-30/desktop-graceful-shutdown-python.json). Two additional desktop disconnect cases then passed with all 12 desktop tests. Manager signal tests require the rebuilt executable and have separate evidence.

These are Linux results. Windows/macOS conformance, the complete upstream suite,
native TUI regression coverage, and the final repository-wide validation steps
remain separate work. In particular, Windows termination of plugin descendants
has not been established by the Unix process-group tests.

`just bazel-lock-update` completed after the storage/attachment workspace and
fixture dependency changes; [log](verification/2026-09-30/bazel-lock-update-storage.log).
This resolves dependency metadata; it is not a Bazel build or test pass. Native
fixture targets and compile-time fixture resources are declared in their BUILD files.

Earlier scoped native-component/acquisition fixes and formatting passed at the
historical contract-1 checkpoint; their logs remain retained. For current
storage-v2 source, scoped codec/protocol, storage-wire and host/API lint gates
passed as recorded above. The maintenance `just fix` completed but reported one
new fixture opening SQLite outside the native shim. That fixture was changed to
`codex_state::open_thread_history_db`, its focused runtime case passed in run
`81cf859e-1d07-45c6-bef6-49aac277f2a5`, and final read-only scoped Clippy passed
without warnings in 96 seconds. Exact commands/logs are in
[storage-maintenance-regressions.json](verification/2026-09-30/storage-maintenance-regressions.json).
No production behavior changed in either corrected test fixture.

These latest source gates use consistent dev/test debug information disabled,
no incremental compilation, and bounded jobs to fit the 32 GiB cloud filesystem.
The historical independent-build/CLI/GUI gates below used their recorded earlier
binary profiles. The first full-workspace attempt stopped during compilation on
missing native GLib development metadata; no tests ran. Repaired GLib/GStreamer,
V8 and patched-zsh prerequisites are setup evidence, not a complete-suite pass.
Fresh v2 independent package/CLI/GUI acceptance passed as recorded below. The
[full-suite retry was blocked during test linking by disk exhaustion](verification/2026-09-30/full-workspace-regressions-attempt2.json);
no complete-workspace tests ran or runtime result is claimed.


## Versioned Python SDK distribution

The [distribution report](verification/2026-09-30/sdk-wheel-distribution.json) and
[15 exact command results](verification/2026-09-30/sdk-wheel-distribution-commands.json)
record the passed `codex-component-sdk` 0.1.0 wheel milestone. The wheel was built
from an independent source copy using local setuptools 84.0.0 and wheel 0.48.0,
then installed without network access into a fresh venv with `PYTHONPATH` unset.
Both SDK module import and the `codex-component-sdk` console entry point resolved
to that venv. LICENSE/NOTICE were verified byte-for-byte at the package paths
used by the builder, in wheel license metadata, and in the built plugin.

The installed SDK generated a custom tool project, then packaged its Python
handler and static resource. Before invoking the manager, both exported source
trees were deleted and the SDK was uninstalled from the venv. The actual frozen
manager installed and invoked the embedded-SDK zipapp, returning exactly
`Wheel-built greeting: ADA LOVELACE (12 characters)`. Removal disabled the plugin;
a subsequent call failed as expected. The manager retained SHA-256
`acffaefb470b796b47f36fd9a9d5d4c23d2b5d3622316256b6c8c770ccc8a8f3`, size
7,092,568 bytes and its original modification time throughout.

The retained wheel is
`/workspace/acceptance/sdk-wheel-final-20260930/wheels/codex_component_sdk-0.1.0-py3-none-any.whl`,
21,901 bytes, SHA-256
`41a13395059a4c315530e9533624e5b2cae1131988760a32acd0bc0614b3e97c`.
The report records every input source and output package hash; the final exported
SDK files match the checkout. Fourteen existing SDK process tests also passed.
[The reusable acceptance script](component-sdk/tests/sdk_distribution_acceptance.py)
performs the full independent build/install/invoke/remove check.

This verifies SDK distribution and process-tool packaging on Linux/Python 3.12;
it does not add a new native subsystem extraction or whole-engine claim. Python
3.10+ remains the declared support range, with other versions/platforms awaiting
conformance runs. Runtime protocol and handler semantics were unchanged. No
Rust build, package registry upload or publication occurred in this milestone.

## Independent packages against the actual built CLI

The [retained report](verification/2026-09-30/harness-cli-acceptance.json) and
[exact command outputs](verification/2026-09-30/harness-cli-commands.json) record
the successful end-to-end milestone. Reproduce it with
[harness_acceptance.py](component-sdk/tests/harness_acceptance.py) and already
built `codex` and `codex-component` executables.

1. Copy the SDK examples into a project outside the harness checkout, build
   their packages separately, delete both plugin source trees, and install the
   resulting calculator/context and model packages.
2. Explicitly select `model_transport:default`. The actual `codex exec --json`
   creates a real engine session. Its model request advertises the installed
   calculator and includes the context contribution. The model component requests
   `6 * 7`; the next real engine request contains `function_call_output: "42"`.
   The component supplies streaming response events and the CLI emits the final
   assistant message and a completed turn.
3. A second actual CLI process resumes session
   `01a0f1ce-5873-7751-9929-f548f8229122`. Its request retains the prior call,
   calculator result, and assistant text. A durable rollout is present.
4. While the replacement is selected, the loopback provider observes **zero
   native inference requests**. After resetting the selection, another CLI
   process resumes the same session through the native transport. Exactly one
   native request reaches the loopback provider, retains the prior history, and
   completes with `native transport restored`.
5. Both executable hashes, modification times, and sizes match before and after
   every plugin build/install/invocation in this milestone.

| Executable | Bytes | Unchanged SHA-256 |
| --- | ---: | --- |
| `codex-component` | 15,384,800 | `25a6862b6f14de4a7bbfefe66013e8cde305bf094eba57dcd2136858abff902a` |
| `codex` | 1,034,217,768 | `df41ac1c2ffed751d2bacb4602d11c92673e03cc3e4c557055edc7cbc592b549` |

The separate [host acceptance report](verification/2026-09-30/component-host-acceptance.json)
also records install, direct tool/context invocation, list, removal, an empty
enabled list, and an unchanged manager executable. Removal retains old immutable
package objects so already-created bindings continue to reference their original
implementation; physical garbage collection is a separate operation.

Inference in these checks is deterministic test data, not a live model service.
The engine, external tool, context injection, persistence, session recovery, and
native transport are actual code paths. CLI JSON reports completed assistant
messages rather than displaying each text delta; per-delta behavior is covered
by the engine tests and browser checks separately. The fixture model produces a
model-metadata fallback warning, preserved in the command output. The executable
hashes identify this milestone build, which predates the new credential adapter;
they must not be cited as authentication validation for later source changes.

The original native inline attachment implementation has also passed a distinct
[outside-source build and install milestone](verification/2026-09-30/attachment-inline-independent.json).
Its exported Cargo project contains its own source dependencies and notices,
with no paths into this checkout. After building it offline, the exported source
was deleted, the package installed and selected, and a 5,242,898-byte staged
upload produced the native inline acknowledgment; resolution returned native
NotFound. The staged input hash and both host binary hashes stayed unchanged.
The manager in this later check has SHA-256
`0c120ccaf35502447c81927f6c4dccf697fbe6c349c6c0a831de4f7dffb8a7a3`.
Exact byte preservation by the host adapter is covered by its subprocess tests;
the four engine image/selection integration tests also passed in the combined gate.

## Storage-v2 independent package and actual CLI

The [independent-build report](verification/2026-09-30/thread-store-native-v2-independent.json)
records a fresh export of 81 local crates and 917 resolved packages. Every local
dependency stayed inside the export; core, CLI, TUI, app-server and core test
support were excluded. Cargo pruned the copied lockfile from 1,482 to 1,071
identities with zero new identities. The offline, locked native build ran from
`/workspace/acceptance/thread-store-native-v2-linux-20260930/source/codex-rs`
and completed in 109.286 seconds. It reused third-party caches and the shared
Cargo target, but the native worker's recorded compilation source was the export
and its artifact was not fresh. This is not a clean-cache or cross-platform build.

The package retained LICENSE/NOTICE and explicitly declared storage contract 2.
Its stripped native executable is 70,824,776 bytes, SHA-256
`0b5c433e116cf9b542697dbc80475f557c152ae2a4e0a1c8a99666c5ed2cc723`.
The exported source was deleted before installation. The [actual CLI report](verification/2026-09-30/thread-store-native-v2-cli.json)
then passed real engine turns, external tool/context execution, streamed fixture
inference, selected-storage cold restart/resume, native model restoration and
native storage restoration with retained history. Three observed selected-store
children terminated. Only inference is deterministic fixture data; this is not
live-provider validation.

Both executables retained their exact sizes, modification times and hashes
through the independent build, installation, actual CLI checks and the GUI
checkpoint below:

| Executable | Bytes | Unchanged SHA-256 |
| --- | ---: | --- |
| `codex-component` | 7,092,568 | `acffaefb470b796b47f36fd9a9d5d4c23d2b5d3622316256b6c8c770ccc8a8f3` |
| `codex` | 625,072,120 | `b7a39b3534c51a6115135083ee9ffcf7c1eced81e51eef246e80c7b706a64371` |

## Historical storage-v1 packages through the actual CLI

The contract-1 CLI passed a first installed-storage engine milestone; see
[report](verification/2026-09-30/storage-engine-cli-acceptance.json) and
[commands](verification/2026-09-30/storage-engine-cli-commands.json). The selected
native worker handled real turns and persisted tool output, was recreated for
cold resume, and terminated after each CLI process. Deselecting the model and
then the storage replacement preserved the same history through the original
native paths. The observed three selected storage children all exited. Both
host executable hashes were unchanged throughout installation and invocation.
The CLI hash was `785d87c030c8c6cc1e1720a1edbe56f4cff2879b9caa0c946c26d3ad63819227`.

That first storage package was assembled from the repository build, and its
evidence predates the acquisition-failure fence and graceful launcher shutdown
changes. It remains a historical checkpoint, distinct from the final milestone
below.

The historical contract-1 [independent native storage report](verification/2026-09-30/thread-store-native-independent.json)
records an offline, locked build from an exported source project outside this
checkout. All local dependency paths stayed inside the export; its 80 local
crates exclude core, CLI, TUI and app-server. The build reused registry/git and
third-party Cargo caches, so this is not a clean-cache or cross-platform build.
The native worker was rebuilt from exported source, packaged with notices, and
the exported source was removed before installation.

The unchanged actual CLI then completed a turn with the installed native store,
external calculator/context and model components, resumed through a fresh
storage process, and retained the same history after restoring native model and
storage implementations. All three observed selected-storage children exited.
The separately packaged storage executable has SHA-256
`290dcf1bbfa069ea7413e12893d61f267ed81a2f2caf39dbe2975b6efd94dc98`.
The same contract-1 package was used in the historical GUI milestone below.
Its unchanged hashes describe that historical build, not the current storage-v2
package verified in the separate section above.

| Executable | Bytes | Unchanged SHA-256 across historical v1 independent build, installation and runtime checks |
| --- | ---: | --- |
| `codex-component` | 16,462,992 | `4dd8ba7e78102f76349acc21ca1eb0772e9106dd1a80f9c757222bcd55742a76` |
| `codex` | 1,035,548,088 | `3ff68d59d27a0434af9f0342b15749f9a2138c1fc074e0a79b5b3d39910de02a` |

The four preserved v1 checkpoint binaries are now in a lossless
[verified archive](verification/2026-09-30/v1-checkpoint-archive.json),
`/workspace/verified-component-checkpoint-20260930/v1-checkpoint.tar.zst`.
Every member's bytes, SHA-256, length and mode were checked by streaming
extraction before removing only those generated binary copies. Source, evidence,
installed packages and active target binaries were retained. The archive report
includes restoration commands; historical paths require restoration before reuse.

One native maintenance boundary remains: [the `migrate-rollouts` CLI](codex-rs/cli/src/migrate_rollouts.rs)
constructs `LocalThreadStore` directly and does not consult component selection.
Its documented operation is inspection or migration of legacy local sessions;
`--apply` writes that native store even when a process store is selected for
engine sessions. This is not an engine fallback. The current background
maintenance RPC only acknowledges scheduling and cannot replace this command's
dry-run/apply options, thread filters, throughput limit, progress, complete
per-thread report and cancellation semantics. These require a separate typed
maintenance contract and selected-provider CLI integration before claiming
that this native maintenance path is replaceable.

## GUI evidence and current scope

### Current storage-v2 checkpoint

The [actual GUI/manager report](verification/2026-09-30/gui-manager-native-storage-v2.json)
and [browser observations](verification/2026-09-30/gui-native-storage-v2-browser.json)
record seven successful Chromium/Playwright evaluations, zero JavaScript errors
and six screenshots. The installed native storage executable exactly matches the
independent v2 package above. GUI/model source copies were also removed before
invocation, and both host binaries remained unchanged.

Checks cover progressive streaming, native command approval and execution of
`printf gui-native-approval`, Stop terminating a blocked model, reload/history
recovery, and another completed turn. A subsequent blocked turn was interrupted
by **one actual Ctrl+C through the manager's foreground PTY**: manager exit was
zero after 0.4329 seconds and all five tracked manager, GUI, app-server, storage
and model PIDs were absent. A cold restart created four fresh processes, recovered
`Task interrupted`, previous tool output and history, then completed a new turn.
A second single Ctrl+C exited zero after 0.3604 seconds with all four PIDs absent.
Both shutdowns allowed up to 210 seconds; they were not forced second-interrupt
cases. [Recovered history](verification/2026-09-30/gui-v2-recovered.png) and
[the new completed turn](verification/2026-09-30/gui-v2-completed-after-restart.png)
are retained with the other screenshots.

Inference is deterministic, while the manager, GUI, native engine, approval,
command execution, storage and recovery are real. This Linux native-layout
package is not evidence for arbitrary backends or other platforms. The GUI run
does not hold native maintenance active; the SQLite-lock test above supplies
that separate evidence. Forced termination still reports unknown durability.

### Earlier GUI checkpoints

The separately packaged GUI has been connected to the actual built Codex
app-server, with an installed deterministic model component. A normal turn and
page reload/history recovery were manually exercised; see the
[real-engine screenshot](verification/2026-09-30/gui-real-engine-chat.png).
The [mobile](verification/2026-09-30/gui-fixture-mobile.png) and
[interrupted](verification/2026-09-30/gui-fixture-interrupted.png) screenshots
come from fixture-backed gateway checks and do not by themselves prove native
approval or cancellation behavior.

Manual checks of native command approval, model-process interruption, and
gateway/app-server restart passed against the real engine. The
[retained report](verification/2026-09-30/gui-real-engine-acceptance.json) links
five screenshots and records unchanged hashes for both executables. The GUI
showed the native command approval, submitted Allow once, and rendered the
successful command output. Stop interrupted the real turn and terminated the
blocked model process. Browser reload and a full gateway/app-server restart
recovered the prior command output and interrupted status; a subsequent turn in
the recovered thread completed successfully.
[gui_harness_fixture.py](component-sdk/tests/gui_harness_fixture.py) prepares
the independent desktop and model packages, captures initial executable hashes,
and configures an isolated home. It does not replace the app-server or launch a
browser. The harmless native command is `printf gui-native-approval`; a separate
test prompt deliberately blocks an external model process for Stop testing.
Only inference was supplied by the deterministic plugin; approval, execution,
interruption, persistence, and session recovery ran through Codex itself.

The historical contract-1 engine also passed the same browser behavior cycle
with selected native process storage: normal response, command approval/execution, Stop with
model-process termination, reload, complete engine/storage restart, recovered
history and another completed turn. The [preliminary composition report](verification/2026-09-30/gui-storage-composition.json)
records exact binaries and PIDs. Its launches were ended by direct GUI SIGTERM;
it does **not** establish graceful manager Ctrl+C. This cycle exposed a client
disconnect double-response error, now fixed and covered by the Python checks.
The later [contract-1 manual report](verification/2026-09-30/gui-manager-native-storage-final.json)
completed that historical manager-shutdown gate; it does not validate storage v2. Ten browser checks passed with no
JavaScript errors, using the independently built native storage package and the
unchanged contract-1 checkpoint executables above. They cover native approval and command output,
progressive streaming, a completed response, Stop terminating a blocked model,
browser reload/history recovery, and manager shutdown during another active
blocked turn. One actual Ctrl+C through the manager's foreground terminal
exited with status zero; all five tracked manager, GUI, app-server, storage and
model PIDs were absent afterward.

A cold restart created fresh processes and recovered interrupted status, prior
tool output and conversation history. A new turn completed successfully. A
second actual manager Ctrl+C again exited zero and all four remaining tracked
processes disappeared. The report links five screenshots and records removal
of GUI and exported storage source before invocation. Only model inference uses
deterministic fixture data; the manager, GUI, engine, approval, tool execution,
storage and shutdown paths run for real.

The requested **Browser (Control the in-app browser with Codex)** tool is not
available in this cloud environment. The actual alternative used for GUI
inspection is Chromium driven by Playwright. Context7's resolve/query tools are
also unavailable; the implementation contracts were inspected at the pinned
upstream source. CLI/runtime-only checks do not need a browser.

## Launcher lifecycle evidence

The [current service-lifetime gate](verification/2026-09-30/component-host-service-lifetime.json)
passed 56 host tests and scoped host/API lint. Presentation initialization and
request delivery are bounded, but the ready service has no execution-lifetime
cap. Normal result/exit receives the full cleanup grace; ordinary RPC deadlines
remain unchanged. The rebuilt manager's [three actual OS-signal cases](verification/2026-09-30/manager-service-lifetime-acceptance.json)
passed with unchanged SHA-256 `acffaefb470b796b47f36fd9a9d5d4c23d2b5d3622316256b6c8c770ccc8a8f3`:
first SIGINT preserves a blocked drain until release and exits zero; a second
SIGINT during drain or SIGINT before readiness exits nonzero with explicit
accepted-write/durability uncertainty. All three direct child PIDs were absent.
These cases use an installed protocol fixture and `/usr/bin/true`, not a GUI or
storage engine. The v2 GUI checkpoint above separately proves graceful shutdown
through the real stack. Cleanup exceeding the force/reap bound remains unconfirmed;
forced termination never establishes write durability.

The earlier launcher checkpoint remains recorded below.

The historical installed-package manager passed three real SIGINT cases with
unchanged binary hash: one interrupt allows the fixture to finish its controlled
drain and fsync before a successful response/exit; a second interrupt and an
interrupt during a stalled handshake both fail explicitly with uncertain write
durability. All three direct child PIDs disappear before acceptance completes.
[Report and commands](verification/2026-09-30/manager-shutdown-reaping-acceptance.json)
identify manager SHA-256
`4dd8ba7e78102f76349acc21ca1eb0772e9106dd1a80f9c757222bcd55742a76`.

The first attempt exposed forced-child zombies; its report is retained separately.
The fix introduces startup ownership before readiness and an explicit bounded
force-and-reap operation. Arbitrary descendant adoption and non-Linux platform
conformance are not established by these direct-child tests. These signal cases
use an installed protocol fixture. The historical contract-1 GUI report above separately proves
graceful manager shutdown with the real GUI/app-server and selected native store,
including an active blocked model, cold recovery and a second clean shutdown.

## Full-workspace preparation, not a test result

The first complete Linux `just test --workspace` attempt stopped before tests ran.
[Attempt two](verification/2026-09-30/full-workspace-regressions-attempt2.json) started
after the completed storage-v2 independent-build/CLI/GUI checkpoint and
**failed during compilation/linking with ENOSPC**, before tests executed. The
reported failures were linking `codex-exec` and `codex-app-server-client` test
targets; the command exited 101. No full-suite runtime result is claimed. The
virtual workspace has 164
current members and no `default-members`; the command includes
normal unit/integration targets, while doctests, ignored tests, other platforms
and remote-environment lanes remain separate coverage. The attempted run used
two build jobs, two test threads, zero retries, no incremental compilation and
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`. No nextest archive is needed
on this disk-constrained worker.

[Attempt four](verification/2026-09-30/full-workspace-regressions-attempt4.json)
also stopped during linking with the workspace filesystem full, after 379 seconds
with one build job. It ran no tests. Attempt three was a command-line rejection,
not a build or runtime result. Recovery preserved the exact accepted CLI/manager
and frozen Rust source in the [version-two archives](verification/2026-09-30/storage-v2-checkpoint-archives.json).
The [symbol-table recovery report](verification/2026-09-30/untested-test-binary-symbol-strip.json)
records 83 untested executables whose allocated sections, program headers and
dynamic symbols were verified unchanged after stripping. Native symbolic
backtraces are limited; one earlier small executable lacks a recorded preimage
and remains separately labelled. Test paths and accepted binaries were preserved.

Broad regressions are now scheduled in bounded target groups. The
[active target inventory](verification/2026-09-30/active-workspace-test-inventory.json)
is source-derived, not a claim that all targets ran. The isolated
`codex/next-components` worktree contains subsequent implementation work; its
[baseline manifest](verification/2026-09-30/next-components-worktree-baseline.json)
separates that work from this frozen checkpoint. Uncompiled work there is not
covered by the earlier runtime evidence.

Source inspection found that V8 150.4.0's default release-asset URL returns 404.
The official OpenAI Codex archive and bindings were downloaded using the
repository's `setup-rusty-v8` procedure: the release manifest was checked against
the pinned manifest checksum, then both artifacts were checked against that
verified manifest. The small [setup evidence](verification/2026-09-30/full-workspace-v8-setup.json)
records paths, provenance, sizes and SHA-256 hashes; it does not contain the
29 MB archive or claim a compiled V8 test result. The scoped environment file is
`/workspace/toolchains/codex-rusty-v8-150.4.0-linux/env.sh`.

The attempted test command includes `--features codex-v8-poc/sandbox`: code-mode
already enables the shared V8 sandbox dependency, and this aligns the PoC's
crate-local assertion with the workspace's linked implementation. It is not an
`--all-features` run. User/mount namespaces and bubblewrap were directly probed
successfully; required libcap/ALSA headers and runtime tools are present. The
exact [patched-zsh DotSlash fixture](verification/2026-09-30/full-workspace-patched-zsh-setup.json)
was fetched and verified against its pinned archive hash, then fetched again
from cache. Its test capability probe exits zero normally and one with
`EXEC_WRAPPER=/usr/bin/false`, confirming interception support. This removes the
missing-artifact and unsupported-interception early-return conditions;
`codex-execve-wrapper` is also included in the root's runtime-helper build. The
scenario results still require the full-workspace runtime gate.

## Activation and remaining gates

The tool, context, lifecycle, and model catalog snapshot is captured for each new
engine session. Existing sessions keep their bindings. Credential selection is
captured when the native `AuthManager` is created, which can precede multiple
sessions; recreating that manager is required to change the selected credential
provider. Restarting the host is sufficient for both, but is not the universal
minimum activation requirement. Live replacement inside active sessions has not
been implemented or claimed.

Still required:
the disk-blocked complete-workspace runtime gate; broader native HTTP/WebSocket, approval, compaction, tool and recovery regressions;
platform and packaging checks; and independent contracts/implementations for the
remaining orchestration, storage, provider catalog, policy, execution, media,
configuration, and service domains. New infrastructure, additive contributions,
and selected replacement of an existing implementation are reported separately.
