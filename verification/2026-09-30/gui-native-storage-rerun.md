# Repeatable GUI regression with selected native storage

This is the repeatable procedure. The completed storage-v2 run is recorded in
[gui-manager-native-storage-v2.json](gui-manager-native-storage-v2.json) and
[gui-native-storage-v2-browser.json](gui-native-storage-v2-browser.json). Use the newly
built manager and Codex CLI, plus the separately built native storage package.
Independent native compilation has separate evidence. This
runbook now targets storage contract version 2. Earlier version-one reports and
packages remain historical evidence and are not rewritten by this procedure.

## Fresh isolated installation

Run from the verified repository. The acceptance directory must not exist yet:

```sh
REPO=/workspace/codex-harness-everythings-a-plugin
MANAGER="$REPO/codex-rs/target/debug/codex-component"
CODEX_BIN="$REPO/codex-rs/target/debug/codex"
STORE_PACKAGE=/workspace/acceptance/thread-store-native-v2-linux-20260930/package
GUI_RUN=/workspace/gui-native-storage-v2

python3 "$REPO/component-sdk/tests/gui_harness_fixture.py" \
  --host "$MANAGER" --codex "$CODEX_BIN" --work-dir "$GUI_RUN"
"$MANAGER" --codex-home "$GUI_RUN/codex-home" install "$STORE_PACKAGE"
"$MANAGER" --codex-home "$GUI_RUN/codex-home" select thread_store default native.thread-store-local
cd "$GUI_RUN/project"
"$MANAGER" --codex-home "$GUI_RUN/codex-home" launch desktop \
  --codex-bin "$CODEX_BIN" --port 0
```

The fixture copies GUI/model sources outside the repository, builds their zipapps,
deletes the copied source, and installs them. Its `fixture-ready.json` records the
launch command, selected home and model state; `setup-commands.json` records setup.
`binaries-before.json` fingerprints the manager and Codex CLI. The model plugin
provides deterministic inference; execution, approval, session state and storage
are the real Codex implementations. No remote model or credentials are needed.

Keep launch attached to a PTY for the actual Ctrl+C check. Record manager, gateway,
app-server and selected storage child PIDs. The ready event contains the two GUI
PIDs. The storage executable path is under this selected home's installed objects;
resolve `/proc/<pid>/exe`, parent PID and command line to identify it. Require each tracked PID to be absent after successful shutdown. A Linux zombie
has terminated but is not reaped, and does not satisfy this check.

## Browser cycle

The requested in-app Browser tool is unavailable. The existing persistent driver
uses real Chromium through Playwright; label this distinction in the result.
Redirect the launcher's stdout to a private ready file and stderr to a separate
log while retaining its PTY. The ready file contains a bearer URL and must not be
published:

```sh
CODEX_PLAYWRIGHT_MODULE=/opt/codex/runtimes/cua/lib/node_modules/playwright-core \
  node "$REPO/component-sdk/tests/gui_browser_driver.cjs" \
  --ready-file "$GUI_RUN/launch-1.stdout" \
  --report-file "$GUI_RUN/browser.json"
```

The driver accepts one JavaScript statement sequence per input line and retains
`page`. It records commands, their results and page errors. Use
`gotoReadyFile('/absolute/path/to/launch-2.stdout')` for a fresh launch instead of
putting the private URL in a command. Inspect and redact the private driver report
before copying evidence into the repository.

1. Create a task and inspect streaming, using this driver input:

   ```js
   await page.locator('#prompt').fill('Verify normal response'); await page.locator('#send').click(); await page.waitForFunction(() => document.querySelector('#run-status').textContent === 'Task complete'); return {text: await page.locator('#messages').innerText(), thread: await page.evaluate(() => localStorage.getItem('codexSelectedThread'))};
   ```

   Record the thread ID. For live streaming evidence, capture the growing text
   before completion in an additional browser observation.

2. Submit `Exercise approval`. Inspect the approval card for
   `printf gui-native-approval`, click `Allow once`, and require both the native
   command output and `Native command approval result:` in the completed reply.
   The approval locator is `page.getByRole('button', {name:'Allow once', exact:true})`.

3. Submit `Exercise cancel`. Wait for `Waiting for Stop:` in the conversation,
   read the model PID from `codex-home/components/state/test.gui-model/blocked-<thread>.json`,
   click `#stop`, and require `#run-status` to become `Task interrupted`, no pending
   approval, and the blocked model process to exit. Earlier messages must remain.

4. Reload the page and select/recover the same task. Check completed text, tool
   output and interrupted status. Send another ordinary prompt and require a
   completed response. Use screenshots plus DOM assertions, and inspect console
   errors. Recheck the sidebar, composer, approval buttons, Stop, and history.

5. Send **one actual Ctrl+C to the manager PTY**, not directly to gateway/app-server.
   Expect graceful-shutdown progress and successful exit only after all tracked
   children close. Save exit status and verify no live selected storage process
   remains. A second interrupt is a forced path and must not count as graceful.

6. Repeat the same launch command against the same Codex home. Navigate to the
   new ready URL (the ephemeral origin can change), click
   `#threads .thread[data-thread-id="<recorded thread>"]`, verify recovered history,
   and send a fresh normal prompt. This must use new gateway/app-server/storage
   PIDs, not only reload an existing browser or reuse a live engine.

7. Stop with one manager Ctrl+C again and record clean termination. Forced
   second-interrupt/deadline behavior has separate controlled-process tests.

The gateway allows app-server 200 seconds, then bounds TERM/KILL escalation and
reports an unknown write outcome. The manager's outer budget is 210 seconds.
Forced group termination or parent death may interrupt an accepted write;
do not replay it or claim durable success without authoritative recovery.
Completed/recovered history, graceful termination, and forced cancellation are
separate observations in the final evidence.
