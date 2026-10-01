"""P01 real Chromium/UI, manager SIGINT and cold-engine recovery acceptance.

Requires frozen binaries and a passed manual_migration_acceptance.py report.
Uses the installed deterministic model fixture, not a live provider. This is the
Playwright fallback, not the unavailable in-app Browser. All artifacts are private.
"""

import argparse
import ctypes
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import queue
import signal
import subprocess
import sys
import threading
import time
from uuid import uuid4

from harness_acceptance import SDK, fingerprint


def identity(pid):
    try:
        fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        return {
            "pid": pid,
            "ppid": int(fields[1]),
            "group": int(fields[2]),
            "state": fields[0],
            "start_ticks": fields[19],
        }
    except (FileNotFoundError, ProcessLookupError):
        return None


def same_process(saved):
    current = identity(saved["pid"])
    return current is not None and current["start_ticks"] == saved["start_ticks"]


def send_signal(saved, signum):
    try:
        descriptor = os.pidfd_open(saved["pid"])
    except ProcessLookupError:
        return
    try:
        if same_process(saved):
            signal.pidfd_send_signal(descriptor, signum)
    finally:
        os.close(descriptor)


def wait_for(predicate, description, seconds=60):
    deadline = time.monotonic() + seconds
    while not predicate():
        if time.monotonic() >= deadline:
            raise AssertionError(f"Timed out: {description}")
        time.sleep(0.025)


class OwnedProcess:
    """Observe descendants without reading argv or signaling unrelated processes."""

    def __init__(self, argv, directory, prefix, environment, interactive=False):
        self.stdout_path = prefix.with_suffix(".stdout")
        self.stderr_path = prefix.with_suffix(".stderr")
        self.stdout = self.stdout_path.open("w")
        self.stderr = self.stderr_path.open("w")
        try:
            self.process = subprocess.Popen(
                list(map(str, argv)),
                cwd=directory,
                env=environment,
                stdin=subprocess.PIPE if interactive else subprocess.DEVNULL,
                stdout=subprocess.PIPE if interactive else self.stdout,
                stderr=self.stderr,
                text=True,
                bufsize=1,
                start_new_session=True,
            )
        except BaseException:
            self.stdout.close()
            self.stderr.close()
            raise
        self.root = identity(self.process.pid)
        if not self.root:
            self.process.wait(timeout=10)
            self.stdout.close()
            self.stderr.close()
            raise AssertionError("Child identity disappeared during startup")
        self.records = {self.root["pid"]: self.root}
        self.reaped_orphans = []
        self.lock, self.stop_monitor = threading.Lock(), threading.Event()
        self.monitor = threading.Thread(target=self.observe, daemon=True)
        self.monitor.start()

    def observe(self):
        while not self.stop_monitor.is_set():
            observed = {}
            for path in Path("/proc").glob("[0-9]*"):
                value = identity(int(path.name))
                if value:
                    observed[value["pid"]] = value
            with self.lock:
                owned = {
                    pid
                    for pid, saved in self.records.items()
                    if observed.get(pid, {}).get("start_ticks") == saved["start_ticks"]
                }
                changed = True
                while changed:
                    added = {
                        pid
                        for pid, value in observed.items()
                        if value["ppid"] in owned and pid not in owned
                    }
                    owned.update(added)
                    changed = bool(added)
                for pid in owned:
                    saved = dict(observed[pid])
                    try:
                        saved["executable"] = os.readlink(f"/proc/{pid}/exe")
                    except OSError:
                        saved["executable"] = self.records.get(pid, {}).get(
                            "executable"
                        )
                    self.records[pid] = saved
            self.stop_monitor.wait(0.01)

    def snapshot(self):
        with self.lock:
            return list(self.records.values())

    def reap_and_check(self, seconds=10):
        def absent():
            records = self.snapshot()
            for saved in records:
                if saved["pid"] == self.process.pid:
                    continue  # Popen is the sole waiter for its direct child.
                if same_process(saved):
                    try:
                        pid, status = os.waitpid(saved["pid"], os.WNOHANG)
                        if pid:
                            self.reaped_orphans.append(
                                {
                                    "pid": pid,
                                    "returncode": os.waitstatus_to_exitcode(status),
                                }
                            )
                    except ChildProcessError:
                        pass
            return all(not Path(f"/proc/{saved['pid']}").exists() for saved in records)

        wait_for(absent, "tracked processes absent (including zombies)", seconds)

    def finish(self):
        self.stop_monitor.set()
        self.monitor.join()
        self.stdout.close()
        self.stderr.close()

    def emergency_cleanup(self):
        # Signal only individually verified identities, including separate groups.
        for saved in reversed(self.snapshot()):
            send_signal(saved, signal.SIGKILL)
        if self.process.poll() is None:
            self.process.wait(timeout=10)
        self.reap_and_check()


class Browser:
    def __init__(self, ready, directory, prefix, environment, register):
        self.report_path = prefix.with_suffix(".json")
        self.owner = OwnedProcess(
            [
                "node",
                SDK / "tests/gui_browser_driver.cjs",
                "--ready-file",
                ready,
                "--report-file",
                self.report_path,
            ],
            directory,
            prefix,
            environment,
            True,
        )
        self.lines = queue.Queue()
        self.reader = threading.Thread(target=self.collect, daemon=True)
        self.reader.start()
        register(self)  # Failed readiness must still leave an owned cleanup handle.
        self.receive(ready=True)
        self.command("page.setDefaultTimeout(60000); return {ready: true};")

    def collect(self):
        for line in self.owner.process.stdout:
            self.owner.stdout.write(line)
            self.owner.stdout.flush()
            self.lines.put(line.strip())
        self.lines.put(None)

    def receive(self, ready=False):
        try:
            line = self.lines.get(timeout=90)
        except queue.Empty:
            raise AssertionError(
                "Browser command timed out; inspect private artifacts"
            ) from None
        if line is None or line.startswith(("ERROR ", "PAGEERROR ")):
            raise AssertionError("Browser command failed; inspect private artifacts")
        if ready:
            assert line.startswith("READY "), "Browser did not announce readiness"
            return None
        return json.loads(line)

    def command(self, source):
        assert "\n" not in source
        self.owner.process.stdin.write(source + "\n")
        self.owner.process.stdin.flush()
        return self.receive()

    def close(self):
        if self.owner.process.poll() is None:
            self.owner.process.stdin.write("close\n")
            self.owner.process.stdin.flush()
            self.owner.process.stdin.close()
        assert self.owner.process.wait(timeout=30) == 0, "Browser driver failed"
        self.reader.join(timeout=5)
        assert not self.reader.is_alive(), "Browser output did not close"
        self.owner.reap_and_check()
        report = json.loads(self.report_path.read_text())
        assert (
            report["closed"]
            and not report["pageErrors"]
            and not report.get("driverError")
        ), (
            "Browser report contains an unclosed browser, page error or driver error; inspect private artifacts"
        )
        assert all(entry.get("passed") is True for entry in report["commands"]), (
            "Browser closed and descendants were reaped, but a recorded GUI command failed; inspect private artifacts"
        )


def submit(browser, prompt):
    assert prompt == prompt.strip(), "Prompt must match the GUI's trimmed submission"
    return browser.command(
        "const prior = await page.locator('#messages article').evaluateAll(ns => ns.map(n => n.dataset.turnId)); "
        f"const prompt = {json.dumps(prompt)}; "
        "await page.locator('#prompt').fill(prompt); await page.locator('#send').click(); "
        "await page.waitForFunction(({prompt, prior}) => [...document.querySelectorAll('#messages article')]"
        ".some(n => n.dataset.type === 'userMessage' && n.querySelector('pre').textContent === prompt "
        "&& n.dataset.turnId && !prior.includes(n.dataset.turnId)), {prompt, prior}); "
        "return await page.locator('#messages article').evaluateAll((ns, prompt) => ns.find(n => "
        "n.dataset.type === 'userMessage' && n.querySelector('pre').textContent === prompt).dataset.turnId, prompt);"
    )


def completed(browser, turn_id, expected):
    return browser.command(
        f"const turn = {json.dumps(turn_id)}, expected = {json.dumps(expected)}; "
        "await page.waitForFunction(({turn, expected}) => "
        "document.querySelector('#run-status').textContent === 'Task complete' && "
        "[...document.querySelectorAll('#messages article')].some(n => n.dataset.turnId === turn && "
        "n.dataset.type === 'agentMessage' && n.querySelector('pre').textContent.includes(expected)), {turn, expected}); "
        "if (await page.locator('#error').textContent()) throw new Error('Visible GUI error'); "
        "if (await page.locator('#stop').isVisible() || await page.locator('#send').isDisabled()) throw new Error('Turn controls did not settle'); "
        "return {completed: turn};"
    )


def screenshot(browser, path):
    browser.command(
        "if (await page.evaluate(() => location.hash || /(?:[?&#]token=|Bearer\\s+[A-Za-z0-9_-]{16})/.test(document.body.innerText))) "
        "throw new Error('Screenshot may contain private navigation data'); "
        f"await page.screenshot({{path: {json.dumps(str(path))}}}); return {{screenshot: true}};"
    )


def blocked_turn(browser, prompt, model_state, thread_id):
    marker = model_state / f"blocked-{thread_id}.json"
    previous = (
        json.loads(marker.read_text()).get("invocation") if marker.exists() else None
    )
    turn_id = submit(browser, prompt)
    browser.command(
        f"const turn = {json.dumps(turn_id)}; "
        "await page.waitForFunction(turn => [...document.querySelectorAll('#messages article')].some(n => "
        "n.dataset.turnId === turn && n.textContent.includes('Waiting for Stop:')), turn); return {blocked: turn};"
    )

    def new_marker():
        try:
            return json.loads(marker.read_text()).get("invocation") != previous
        except (OSError, ValueError):
            return False

    wait_for(new_marker, "new blocked model invocation")
    value = json.loads(marker.read_text())
    saved = identity(value["pid"])
    assert saved and saved["state"] not in ("Z", "X"), "Blocked model is not running"
    return turn_id, saved


def stop_manager(owner, report):
    assert owner.process.poll() is None, "Manager exited before the shutdown test"
    started = time.monotonic()
    send_signal(owner.root, signal.SIGINT)  # Only manager; never gateway/store first.
    report["first_sigint"] = True
    try:
        status = owner.process.wait(timeout=220)
    except subprocess.TimeoutExpired:
        report["forced"] = "grace expired; durability unknown"
        send_signal(owner.root, signal.SIGINT)
        try:
            owner.process.wait(timeout=15)
        except subprocess.TimeoutExpired:
            owner.emergency_cleanup()
        raise AssertionError("Manager exceeded graceful cleanup budget") from None
    report.update(manager_status=status, shutdown_seconds=time.monotonic() - started)
    assert status == 0, "Manager did not confirm graceful cleanup; durability unknown"
    owner.reap_and_check()
    report["tracked_processes_absent"] = True


def drain_subreaper():
    reaped, deadline = [], time.monotonic() + 10
    while True:
        try:
            pid, status = os.waitpid(-1, os.WNOHANG)
        except ChildProcessError:
            return {"passed": True, "reaped": reaped}
        if pid:
            reaped.append({"pid": pid, "returncode": os.waitstatus_to_exitcode(status)})
        elif time.monotonic() >= deadline:
            remaining = [
                saved
                for path in Path("/proc").glob("[0-9]*")
                if (saved := identity(int(path.name))) and saved["ppid"] == os.getpid()
            ]
            return {
                "passed": False,
                "reaped": reaped,
                "error": "owned descendants remain",
                "remaining_adopted_children": remaining,
            }
        else:
            time.sleep(0.025)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--codex", type=Path, required=True)
    parser.add_argument("--manual-migration-report", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "linux" or not hasattr(signal, "pidfd_send_signal"):
        parser.error("Linux /proc, pidfds and child-subreaper support are required")
    host, codex = args.host.resolve(strict=True), args.codex.resolve(strict=True)
    work, manual_path = (
        args.work_dir.resolve(),
        args.manual_migration_report.resolve(strict=True),
    )
    if (
        work.exists()
        or work == SDK.parent
        or SDK.parent in work.parents
        or any((parent / ".git").exists() for parent in work.parents)
    ):
        parser.error("work-dir must be new and outside the source checkout")
    os.umask(0o077)
    work.mkdir(parents=True, mode=0o700)
    report = {
        "passed": False,
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "browser": "Chromium through Playwright; not in-app Browser",
        "inference": "deterministic fixture; no live provider",
        "runs": [],
        "tracking_scope": "observed descendant identities plus child-subreaper drain; scans can miss short-lived descendants",
    }
    owners, browsers = [], []
    try:
        manual = json.loads(manual_path.read_text())
        before = {"host": fingerprint(host), "codex": fingerprint(codex)}
        assert manual["passed"] and manual["host_unchanged"]
        assert before == manual["binaries_before"] == manual["binaries_after"]
        independent_path = Path(manual["independent_report"])
        assert fingerprint(independent_path) == manual["independent_report_fingerprint"]
        independent = json.loads(independent_path.read_text())
        assert independent["passed"] and independent["contract_version"] == 2
        assert (
            independent["source_removed_before_install"]
            and before == independent["binaries_after"]
        )
        package = Path(independent["artifact_directory"]) / "package"
        for name, expected in independent["package_files"].items():
            assert fingerprint(package / name) == expected, (
                "Independent package changed"
            )
        fixture = json.loads(Path(manual["gui_fixture"]).read_text())
        gui_codex_home, project = Path(fixture["codex_home"]), Path(fixture["cwd"])
        model_state, thread_id = Path(fixture["model_state"]), manual["thread_id"]
        installed = gui_codex_home / "components/objects"
        package_before = {
            str(p.relative_to(installed)): fingerprint(p)
            for p in installed.rglob("*")
            if p.is_file()
        }
        settings = json.loads((gui_codex_home / "components/config.json").read_text())
        storage_id = json.loads((package / "codex-component.json").read_text())["id"]
        assert settings["selections"]["thread_store:default"] == storage_id
        report.update(
            binaries_before=before,
            manual_report=str(manual_path),
            manual_report_fingerprint=fingerprint(manual_path),
            thread_id=thread_id,
            script_fingerprint=fingerprint(Path(__file__)),
            driver_fingerprint=fingerprint(SDK / "tests/gui_browser_driver.cjs"),
        )
        libc = ctypes.CDLL(None, use_errno=True)
        libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
        libc.prctl.restype = ctypes.c_int
        if libc.prctl(36, 1, 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), "Cannot enable child subreaper")
        environment = dict(
            os.environ,
            CODEX_SQLITE_HOME=str(gui_codex_home),
            PYTHONPATH="",
            CODEX_PLAYWRIGHT_MODULE=os.environ.get(
                "CODEX_PLAYWRIGHT_MODULE",
                "/opt/codex/runtimes/cua/lib/node_modules/playwright-core",
            ),
            CODEX_BROWSER_BIN=os.environ.get("CODEX_BROWSER_BIN", "/usr/bin/chromium"),
        )
        nonce = uuid4().hex[:12]
        persisted = [
            "Preserve this real legacy conversation through manual migration.",
            "GUI fixture response: Preserve this real legacy conversation through manual migration.",
            "Continue the migrated acceptance conversation.",
        ]
        for cycle in (1, 2):
            run = {"cycle": cycle, "passed": False}
            report["runs"].append(run)
            manager = OwnedProcess(
                [
                    host,
                    "--codex-home",
                    gui_codex_home,
                    "launch",
                    "desktop",
                    "--codex-bin",
                    codex,
                    "--port",
                    "0",
                ],
                project,
                work / f"manager-{cycle}",
                environment,
            )
            owners.append(manager)

            def ready():
                assert manager.process.poll() is None, "Manager exited before readiness"
                try:
                    return any(
                        json.loads(line).get("method") == "presentation/ready"
                        for line in manager.stdout_path.read_text().splitlines()
                    )
                except ValueError:
                    return False

            wait_for(ready, "presentation readiness", 125)
            browser = Browser(
                manager.stdout_path,
                project,
                work / f"browser-{cycle}",
                environment,
                browsers.append,
            )
            browser.command(
                "await page.waitForFunction(() => document.querySelector('#connection').classList.contains('connected')); return {connected: true};"
            )
            browser.command(
                f"await page.locator({json.dumps('.thread[data-thread-id=' + json.dumps(thread_id) + ']')}).click(); "
                f"await page.waitForFunction(texts => texts.every(t => document.querySelector('#messages').textContent.includes(t)), {json.dumps(persisted)}); return {{history: true}};"
            )
            if cycle == 2:
                browser.command(
                    "await page.waitForFunction(() => document.querySelector('#run-status').textContent === 'Task interrupted'); return {coldInterruptedTurn: true};"
                )
            prompt = (
                f"Verify P01 {'streaming' if cycle == 1 else 'cold recovery'} {nonce}: "
                + "stream observation " * 30
            ).strip()
            turn_id = submit(browser, prompt)
            partial = browser.command(
                f"const turn = {json.dumps(turn_id)}; "
                "await page.waitForFunction(turn => document.querySelector('#run-status').textContent === 'Working on your task' && "
                "[...document.querySelectorAll('#messages article')].some(n => n.dataset.turnId === turn && n.dataset.type === 'agentMessage' && n.querySelector('pre').textContent.length > 0), turn); "
                "return await page.locator('#messages article').evaluateAll((ns, turn) => ns.find(n => n.dataset.turnId === turn && n.dataset.type === 'agentMessage').querySelector('pre').textContent, turn);"
            )
            completed(browser, turn_id, "GUI fixture response: " + prompt)
            assert 0 < len(partial) < len("GUI fixture response: " + prompt), (
                "No partial streaming observation"
            )
            run["streaming_turn"] = turn_id
            if cycle == 1:
                approval = submit(browser, f"Exercise approval {nonce}")
                browser.command(
                    "await page.getByRole('button', {name: 'Allow once', exact: true}).waitFor(); "
                    "if (!(await page.locator('#approvals').innerText()).includes('printf gui-native-approval')) throw new Error('Unexpected approval'); return {approval: true};"
                )
                screenshot(browser, work / "approval.png")
                browser.command(
                    "await page.getByRole('button', {name: 'Allow once', exact: true}).click(); return {allowed: true};"
                )
                completed(browser, approval, "Native command approval result:")
                browser.command(
                    f"await page.waitForFunction(turn => [...document.querySelectorAll('#messages article')].some(n => n.dataset.turnId === turn && n.dataset.type === 'commandExecution' && n.firstChild.textContent.endsWith('· completed') && n.querySelector('pre').textContent.split('\\n').slice(1).join('\\n').includes('gui-native-approval') && n.dataset.pending === 'false'), {json.dumps(approval)}); return {{nativeToolCompleted: true}};"
                )
                interrupted, model = blocked_turn(
                    browser, f"Exercise cancel {nonce}", model_state, thread_id
                )
                browser.command(
                    "await page.locator('#stop').click(); await page.waitForFunction(() => document.querySelector('#run-status').textContent === 'Task interrupted'); "
                    "if (await page.locator('#stop').isVisible() || await page.locator('#send').isDisabled() || await page.locator('#approvals').innerText()) throw new Error('Cancel controls did not settle'); return {interrupted: true};"
                )
                wait_for(
                    lambda: not Path(f"/proc/{model['pid']}").exists(),
                    "interrupted model reaped",
                )
                run.update(
                    approval_turn=approval,
                    interrupted_turn=interrupted,
                    interrupted_model=model,
                )
                browser.command(
                    "await page.reload(); await page.waitForFunction(() => document.querySelector('#run-status').textContent === 'Task interrupted'); return {browserReload: true};"
                )
                continuation = f"Verify P01 continuation {nonce}"
                completed(
                    browser,
                    submit(browser, continuation),
                    "GUI fixture response: " + continuation,
                )
                persisted += [prompt, "Native command approval result:", continuation]
            screenshot(browser, work / f"recovered-{cycle}.png")
            shutdown_prompt = f"Exercise cancel at manager shutdown {nonce}-{cycle}"
            shutdown_turn, shutdown_model = blocked_turn(
                browser, shutdown_prompt, model_state, thread_id
            )
            if cycle == 1:
                persisted.append(
                    shutdown_prompt
                )  # Canceled user input persists; partial output need not.
            run.update(shutdown_turn=shutdown_turn, shutdown_model=shutdown_model)
            wait_for(
                lambda: any(
                    p["pid"] == shutdown_model["pid"] for p in manager.snapshot()
                ),
                "owned model tracking",
            )
            tracked = manager.snapshot()
            assert any(p.get("executable") == str(codex) for p in tracked), (
                "App Server not observed"
            )
            native_manifest = json.loads((package / "codex-component.json").read_text())
            native_path = (
                installed
                / settings["installed"][storage_id]
                / native_manifest["entrypoint"]
            )
            native_expected = independent["package_files"][
                native_manifest["entrypoint"]
            ]
            assert any(p.get("executable") == str(native_path) for p in tracked), (
                "Selected native storage not observed"
            )
            native_actual = fingerprint(native_path)
            assert all(
                native_actual[key] == native_expected[key]
                for key in ("sha256", "bytes")
            ), "Installed storage differs from independently built worker"
            assert any(p["ppid"] == manager.process.pid for p in tracked), (
                "Presentation child not observed"
            )
            stop_manager(manager, run)
            browser.close()
            run.update(
                processes=manager.snapshot(),
                browser_processes=browser.owner.snapshot(),
                runner_reaped_manager_orphans=manager.reaped_orphans,
                runner_reaped_browser_orphans=browser.owner.reaped_orphans,
                installed_storage_fingerprint=native_actual,
            )
            run["subreaper_drain"] = drain_subreaper()
            assert run["subreaper_drain"]["passed"], (
                "First runtime must drain before any cold restart"
            )
            run["passed"] = True
            manager.finish()
            browser.owner.finish()
        report["subreaper"] = drain_subreaper()
        assert report["subreaper"]["passed"], "Descendants remain after both cycles"
        assert before == {"host": fingerprint(host), "codex": fingerprint(codex)}, (
            "Host changed"
        )
        assert package_before == {
            str(p.relative_to(installed)): fingerprint(p)
            for p in installed.rglob("*")
            if p.is_file()
        }, "Installed packages changed"
        report["binaries_after"] = before
        report["installed_packages_unchanged"] = True
        report["passed"] = True
    except BaseException as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        for owner in owners:
            cleanup = {"manager_pid": owner.process.pid}
            report.setdefault("cleanup_checks", []).append(cleanup)
            try:
                if owner.process.poll() is None:
                    stop_manager(owner, cleanup)
                owner.reap_and_check()
            except Exception as error:
                report.setdefault("cleanup_errors", []).append(str(error))
                cleanup["forced"] = (
                    "verified owned processes killed; durability unknown"
                )
                try:
                    owner.emergency_cleanup()
                except Exception as cleanup_error:
                    report["cleanup_errors"].append(str(cleanup_error))
            owner.finish()
        for browser in browsers:
            try:
                if browser.owner.process.poll() is None:
                    browser.close()
                browser.owner.reap_and_check()
            except Exception as error:
                report.setdefault("cleanup_errors", []).append(str(error))
                try:
                    browser.owner.emergency_cleanup()
                except Exception as cleanup_error:
                    report["cleanup_errors"].append(str(cleanup_error))
            browser.reader.join(timeout=5)
            if browser.reader.is_alive():
                report.setdefault("cleanup_errors", []).append(
                    "Browser collector did not terminate"
                )
                browser.owner.stop_monitor.set()
                browser.owner.monitor.join()
            else:
                browser.owner.finish()
        report["final_drain"] = drain_subreaper()
        report["passed"] = (
            report["passed"]
            and report["final_drain"]["passed"]
            and not report.get("cleanup_errors")
        )
        drain = report["final_drain"]
        for _ in range(3):
            if drain["passed"]:
                break
            report.setdefault("forced_adopted_cleanup", []).append(drain)
            report["forced_cleanup_warning"] = (
                "owned adopted children forced; durability unknown"
            )
            for saved in drain["remaining_adopted_children"]:
                current = identity(saved["pid"])
                if current and current["ppid"] == os.getpid():
                    send_signal(saved, signal.SIGKILL)
            drain = drain_subreaper()
        if report.get("forced_adopted_cleanup"):
            report["after_forced_drain"] = drain
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        (work / "acceptance-private.json").write_text(
            json.dumps(report, indent=2) + "\n"
        )
    print(work / "acceptance-private.json")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
