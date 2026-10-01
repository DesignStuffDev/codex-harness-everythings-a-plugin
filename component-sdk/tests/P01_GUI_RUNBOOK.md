# P01 GUI, launcher shutdown and cold recovery runbook

Prepared by read-only inspection on 2026-10-01. **These P01 browser/launcher
checks have not been run by preparing this document.** Run only after the P01
candidate binaries and independent storage package have been frozen. Existing
baseline GUI evidence does not verify a new binary or source tree.

The requested in-app Browser is not exposed by the available cloud tools. The
installed fallback is real Chromium driven through Playwright; report it by that
name, not as an in-app Browser check. No viewer, relay, network-policy or existing
remote-desktop changes are needed for this regression. Inference is the explicit
deterministic model fixture; the GUI, manager, App Server, storage and native
command execution are real. This does not test a live model provider.

## Freeze the exact input tuple

Record absolute paths and SHA-256 fingerprints for the candidate `codex-component`
and `codex` binaries, the actual source inventory/diff, acceptance scripts and
independent package/report. Also retain sizes and mtime values because the
existing helpers compare complete fingerprint objects, not only hashes.

Use new work directories outside both source checkouts. Keep the accepted
baseline homes, viewer home and all failed-run evidence untouched. Shell names
below are run-specific variables, not changes to system home directories:

```sh
P01_REPO=/workspace/codex-harness-everythings-a-plugin
P01_MANAGER=/absolute/path/to/frozen-p01/codex-component
P01_CODEX=/absolute/path/to/frozen-p01/codex
P01_NATIVE_WORK=/absolute/new/p01-independent-storage
P01_NATIVE_TARGET=/absolute/path/to/coordinated-build-target
P01_MIGRATION_WORK=/absolute/new/p01-migration-runtime
```

Replace placeholder paths with the owner's frozen candidates; do not substitute
the previously accepted baseline. At inspection time the successful report at
`/workspace/acceptance/thread-store-native-v2-linux-20260930/independent-report.json`
described the older storage package. It is not P01 evidence. Preserve the failed
older report beside it as failed evidence.

## Prerequisite independent-package and migration acceptance

If the owner has already produced matching successful reports, validate and reuse
them rather than repeating builds. Otherwise the following commands are the
existing entry points, **not instructions to start an uncoordinated Rust build**.
The first requires the owner's established Rust verification environment and
build target. It builds only the exported worker and never builds the host.

```sh
python3 "$P01_REPO/component-sdk/tests/native_storage_acceptance.py" \
  --repo "$P01_REPO" --host "$P01_MANAGER" --codex "$P01_CODEX" \
  --work-dir "$P01_NATIVE_WORK" --target-dir "$P01_NATIVE_TARGET" \
  --contract-version 2 --package-version 0.2.0

python3 "$P01_REPO/component-sdk/tests/subreaper_runner.py" \
  --report "$P01_MIGRATION_WORK-subreaper.json" -- \
  python3 "$P01_REPO/component-sdk/tests/manual_migration_acceptance.py" \
  --host "$P01_MANAGER" --codex "$P01_CODEX" \
  --independent-report "$P01_NATIVE_WORK/independent-report.json" \
  --work-dir "$P01_MIGRATION_WORK"
```

Require `independent-report.json` to pass, declare storage contract 2, match both
host fingerprints before/after, record every local dependency inside its export,
and confirm exported source removal before installation. Match the packaged file
fingerprints too. Source removal here applies only to the helper-created export,
not either working checkout or preserved source archive.

Require `manual-migration-report.json` to pass with `host_unchanged: true`, and
the subreaper report to show command exit 0, no runner error and successful drain.
It prepares `$P01_MIGRATION_WORK/gui`, installs the same native storage package,
seeds a real legacy conversation, checks dry-run parity/unchanged storage bytes,
applies and repeats migration, resumes through cold CLI/App Server, interrupts a
blocked turn and checks recorded process disappearance. Its
`browser_manager_regression` deliberately remains pending.

Read the retained report's `thread_id` and `gui_fixture` internally. The GUI
fixture contains `launch_command`, `cwd`, `codex_home` and `model_state`.
Do not rerun `gui_harness_fixture.py` against that populated directory: its fresh
home creation is intentional. Used alone, that helper installs only the desktop
and model plugins; it does not install native storage or migrate a thread.

## Launch through the actual component manager

Use the retained fixture and matching frozen binaries:

```sh
GUI_FIXTURE="$P01_MIGRATION_WORK/gui"
GUI_CODEX_HOME="$GUI_FIXTURE/codex-home"
GUI_CHECK="$P01_MIGRATION_WORK/browser-manager"
umask 077
mkdir "$GUI_CHECK"
cd "$GUI_FIXTURE/project"

env CODEX_SQLITE_HOME="$GUI_CODEX_HOME" \
  python3 "$P01_REPO/component-sdk/tests/subreaper_runner.py" \
  --report "$GUI_CHECK/run-a-subreaper.json" -- \
  "$P01_MANAGER" --codex-home "$GUI_CODEX_HOME" launch desktop \
  --codex-bin "$P01_CODEX" --port 0 \
  > "$GUI_CHECK/run-a-ready.jsonl" 2> "$GUI_CHECK/run-a-manager.stderr"
```

Run this in a retained execution session/supervisor, allowing independent browser
commands while it stays alive. Do not use a short shell timeout. The ready log
contains a private bearer URL: keep the directory 0700/files 0600 and do not cat
that log, print the URL or include it in process arguments or public reports.
Read JSON internally until `presentation/ready` appears. A listening port alone
does not prove that the engine or selected storage is working.

Record ownership before interaction: manager PID and `/proc/<pid>/stat` start
ticks, its actual executable, and descendant PID/start-tick identities. Identify
this manager by its unique redirected ready-log file, expected executable and
fixture cwd, not a process-name search. Continue collecting owned descendants
throughout the run, including processes that create separate process groups.
Record the selected storage executable under this fixture's
`components/objects` and blocked model PID markers under `model_state`.
Keep these observations free of command-line/token dumps.

The supplied subreaper owns/reaps adopted descendants and preserves the command's
actual exit status. It is not a substitute for the product's shutdown behavior.
It forwards signals to its child's process group, so the graceful test below
must address the verified manager PID directly, not send Ctrl+C to the wrapper.

## Attach the existing Chromium driver

Read-only inspection found Playwright/core 1.57.0 at the module path below and
Chromium at `/usr/bin/chromium`. Recheck file availability if the environment
changes; no browser launch was performed while preparing this runbook.

```sh
env CODEX_PLAYWRIGHT_MODULE=/opt/codex/runtimes/cua/lib/node_modules/playwright-core \
  CODEX_BROWSER_BIN=/usr/bin/chromium \
  node "$P01_REPO/component-sdk/tests/gui_browser_driver.cjs" \
  --ready-file "$GUI_CHECK/run-a-ready.jsonl" \
  --report-file "$GUI_CHECK/run-a-browser-private.json"
```

Keep stdin open. Each subsequent input line is JavaScript evaluated with `page`
available. The driver reads the private URL internally. Its viewport is
1440x960; this is browser automation, not the separate remote-viewer canvas.
Driver process exit 0 alone is insufficient: individual command failures are
recorded but do not change its final exit status. Require every intended command
to pass and inspect `pageErrors`/`driverError`. Preserve observer mistakes as
failed attempts rather than overwriting them.

## Browser actions and assertions

Use locator actions to drive the UI. DOM reads are assertions; do not call
internal `rpc()`/`submit()` functions and label that as UI interaction. The status
selector is **`#run-status`**, not `#status`.

1. Wait for `#connection` to say `Connected`, `#error` to be empty and the migrated
   thread to appear in the sidebar. Click `.thread[data-thread-id="..."]` using
   the report's nonsecret `thread_id`. Verify the retained seed prompt and its
   assistant response, plus the cold-CLI continuation, appear in `#messages`.
   This checks the migrated thread rather than a newly created empty task.
2. Submit `Verify P01 GUI streaming after migration: ` plus
   `"stream observation ".repeat(20)` through `#prompt` and `#send`. Observe a
   partial assistant response while `#run-status` is `Working on your task`, then
   the complete `GUI fixture response: ...` and `Task complete`. Assert Stop is
   hidden, Send is enabled and no error is shown. A final screenshot alone does
   not prove streaming; retain the intermediate observation.
3. Submit `Exercise approval`. Wait for an approval card, inspect its harmless
   `printf gui-native-approval` command, and click `Allow once`. Require native
   command output/tool completion and `Native command approval result:` in the
   final assistant response. Exercise `Decline` on a separate approval turn and
   retain its actual denial result. Never approve a different unexpected command
   merely to make the test pass.
4. Submit `Exercise cancel`. Wait for the deliberately blocked fixture text and
   the new `blocked-<thread-id>.json` invocation marker; capture that invocation's
   PID/start ticks. Click `#stop`. Require `Task interrupted`, no pending approval/tool
   state, enabled Send, and actual disappearance of that model process.
5. Submit a normal follow-up and require successful completion. Click New task,
   then reselect the migrated thread and verify its prior history. Reload the
   page and check selection/history recovery; label this **browser reload**, not
   cold engine recovery.

Example single-line driver commands (substitute the verified thread ID and safe
output paths; never paste bearer URLs):

```js
await page.waitForFunction(() => document.querySelector('#connection').classList.contains('connected')); return {connected: true};
await page.locator('.thread[data-thread-id="MIGRATED_THREAD_ID"]').click(); await page.waitForFunction(() => document.querySelector('#messages').textContent.includes('Preserve this real legacy conversation through manual migration.')); return {migratedHistoryVisible: true};
const prior = await page.locator('#messages .assistant').count(); await page.locator('#prompt').fill('Verify P01 GUI streaming after migration: ' + 'stream observation '.repeat(20)); await page.locator('#send').click(); await page.waitForFunction((n) => document.querySelector('#run-status').textContent === 'Working on your task' && document.querySelectorAll('#messages .assistant').length > n && document.querySelector('#messages .assistant:last-child pre')?.textContent.length > 0, prior); const partial = await page.locator('#messages .assistant').last().innerText(); await page.waitForFunction(() => document.querySelector('#run-status').textContent === 'Task complete'); const complete = await page.locator('#messages .assistant').last().innerText(); if (complete.length <= partial.length) throw new Error('No partial-to-final text growth observed'); return {partial, complete, status: await page.locator('#run-status').innerText(), error: await page.locator('#error').textContent()};
await page.locator('#prompt').fill('Exercise approval'); await page.locator('#send').click(); await page.getByRole('button', {name: 'Allow once', exact: true}).waitFor(); return {approval: await page.locator('#approvals').innerText()};
await page.getByRole('button', {name: 'Allow once', exact: true}).click(); await page.waitForFunction(() => document.querySelector('#run-status').textContent === 'Task complete'); return {messages: await page.locator('#messages').innerText()};
await page.locator('#prompt').fill('Exercise cancel'); await page.locator('#send').click(); await page.waitForFunction(() => document.querySelector('#messages').textContent.includes('Waiting for Stop: this model process is deliberately blocked.')); return {stopVisible: await page.locator('#stop').isVisible()};
await page.locator('#stop').click(); await page.waitForFunction(() => document.querySelector('#run-status').textContent === 'Task interrupted'); return {status: await page.locator('#run-status').innerText()};
```

Strengthen waits with the new turn/message identity or increased message count:
old `Task complete`, repeated response text or an old blocked marker must not
satisfy a new turn's assertion. The concise examples do not replace that fence,
the streaming observation or external process-identity assertions.

## Actual Launch Ctrl+C, then cold recovery

For the first shutdown, finish a distinctive normal turn. Optionally start a
fresh blocked cancellation turn to exercise cleanup with an active model; record
its new invocation and expected interrupted outcome separately.

Send exactly one SIGINT to the **verified manager process**, equivalent to its
Ctrl+C handler. Prefer `pidfd_open`, then revalidate saved start ticks, and use
`signal.pidfd_send_signal`. Do not signal the App Server, gateway or storage
first; do not kill the presentation group as the normal shutdown action.

Allow the manager's 210-second graceful budget plus a small observation margin
(220 seconds total). The gateway has a 200-second App Server cleanup allowance.
Keep the launch stream and owner alive while waiting. Record the manager's real
wait status, the subreaper drain result and all tracked process identities.
Require exit 0 and absence of every tracked manager/gateway/App Server/storage/
model PID from `/proc`; zombies are failures, not successful disappearance.
Descendant sampling has a stated visibility limit, so retain subreaper drain
evidence as well as explicit tracked-PID checks.

If the deadline is exceeded, preserve failure evidence first. A second manager
SIGINT exercises the explicit forced path. Only a final cleanup fallback may
signal verified, newly owned groups, checking identities to avoid PID reuse;
never `pkill` names or touch the existing viewer. Forced cleanup must be reported
as failed graceful shutdown with durability unknown. The dedicated
`test_manager_shutdown.py` fixture covers held-drain/second-interrupt semantics;
it is complementary evidence and does not replace this real GUI/native-storage
launcher check.

After successful shutdown, close the first browser driver by sending `close`.
Start a **new** manager via the same command and same retained home, changing all
`run-a` output paths to `run-b`. Require fresh manager/gateway/server/storage
identities and a fresh ready event. Start a fresh browser driver/context against
the new ready file; never reuse the old bearer URL. Select the migrated thread
from its sidebar and verify seed, migrated continuation, approval output and the
distinctive GUI turn survived. Submit a new normal turn and stop it through the
manager's same graceful path. This second process/context cycle, not merely
`page.reload()`, is the cold GUI recovery proof.

## Screenshots and evidence disposition

Capture page-content screenshots with `page.screenshot` after verifying the
frontend has consumed/removed its URL fragment. Page screenshots exclude browser
chrome; do not capture the raw ready log, session storage, request headers or
network inspector. Inspect the actual image, including visible errors, before
sharing it. Keep reports private until reviewed: the driver stores exception
messages, which can contain navigation URLs, and acceptance logs contain fixture
paths and conversation content.

Record source/binary/package fingerprints before and after the run; both hosts
and installed package files must remain unchanged. Preserve private raw command
reports, status/wait evidence, PIDs/start ticks, subreaper reports, turn/thread
identities, screenshots and exact failed attempts. Publish only a sanitized
summary with artifact references, explicit deterministic inference and the
in-app Browser limitation. Do not mark browser-manager regression passed unless
UI actions, actual launcher shutdown and fresh-process recovery all pass.
