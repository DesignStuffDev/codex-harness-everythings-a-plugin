"""Actual manager SIGINT acceptance with an installed, explicitly blocked fixture.

Run with --host PATH --work-dir NEW_DIRECTORY to retain evidence. Unittest
discovery requires CODEX_COMPONENT_MANAGER to opt into these built-binary tests.
The fixture implements the wire directly; this does not test the desktop SDK/UI.
Successful shutdown requires the fixture PID to disappear, including zombie state.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest

FIXTURE = """import json, os, pathlib, sys, time
def send(value):
    print(json.dumps(value), flush=True)
initialize = json.loads(sys.stdin.readline())
assert initialize['type'] == 'initialize' and initialize['api_version'] == 1
state = pathlib.Path(initialize['state_dir'])
state.mkdir(parents=True, exist_ok=True)
identity = {'pid':os.getpid(),'parent_pid':os.getppid(),'entrypoint':__file__}
if DELAY_READY:
    (state / 'starting.tmp').write_text(json.dumps(identity))
    (state / 'starting.tmp').replace(state / 'starting.json')
    while not (state / 'allow_ready').exists():
        time.sleep(0.01)
send({'type':'ready','api_version':1})
request = json.loads(sys.stdin.readline())
assert request['method'] == 'launch'
assert request['component'] == {'kind':'presentation','name':'desktop'}
assert pathlib.Path(request['params']['codex_bin']).is_file()
(state / 'started.json').write_text(json.dumps(identity))
send({'type':'event','id':request['id'],'event':{'type':'fixture/ready',**identity}})
message = json.loads(sys.stdin.readline())
assert message == {'type':'shutdown'}, message
(state / 'shutdown.json').write_text(json.dumps({'received':True}))
send({'type':'event','id':request['id'],'event':{'type':'fixture/draining'}})
while not (state / 'release').exists():
    time.sleep(0.01)
with (state / 'finalized.json').open('w') as saved:
    json.dump({'accepted_write':'durable'}, saved)
    saved.flush()
    os.fsync(saved.fileno())
send({'type':'result','id':request['id'],'result':{'drained':True,'fixture':'manager-shutdown'}})
"""


def fingerprint(path):
    digest = hashlib.sha256()
    with path.open("rb") as binary:
        for block in iter(lambda: binary.read(1024 * 1024), b""):
            digest.update(block)
    return {
        "sha256": digest.hexdigest(),
        "bytes": path.stat().st_size,
        "mtime_ns": path.stat().st_mtime_ns,
    }


def process_identity(pid):
    try:
        fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        return {"pid": pid, "state": fields[0], "start_ticks": fields[19]}
    except FileNotFoundError:
        return None


def fixture_alive(identity):
    current = process_identity(identity["pid"])
    return bool(
        current
        and current["start_ticks"] == identity["start_ticks"]
        and current["state"] not in ("Z", "X")
    )


def wait_until(condition, description, manager=None, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if condition():
            return
        if manager is not None and manager.poll() is not None:
            raise AssertionError(
                f"manager exited {manager.returncode} before {description}"
            )
        time.sleep(0.01)
    raise AssertionError(f"timed out waiting for {description}")


def exercise(host, directory, *, force, startup=False):
    """Use real installation, subprocesses and signals; never rebuild the manager."""
    directory.mkdir(parents=True, exist_ok=False)
    home = directory / "home"
    package = directory / "package"
    package.mkdir()
    executable = package / "presentation-fixture"
    executable.write_text(f"#!{sys.executable}\nDELAY_READY = {startup!r}\n" + FIXTURE)
    executable.chmod(0o755)
    (package / "codex-component.json").write_text(
        json.dumps(
            {
                "api_version": 1,
                "id": "test.manager-shutdown",
                "version": "0.1.0",
                "entrypoint": executable.name,
                "args": [],
                "dependencies": {},
                "components": [
                    {
                        "kind": "presentation",
                        "name": "desktop",
                        "contract_version": 1,
                        "metadata": {},
                    }
                ],
            }
        )
    )
    commands = []
    command = [str(host), "--codex-home", str(home)]
    environment = dict(os.environ, PYTHONPATH="", PYTHONUNBUFFERED="1")
    installed = subprocess.run(
        command + ["install", str(package)],
        cwd=directory,
        env=environment,
        capture_output=True,
        text=True,
        timeout=15,
    )
    commands.append(
        {
            "command": installed.args,
            "status": installed.returncode,
            "stdout": installed.stdout,
            "stderr": installed.stderr,
        }
    )
    if installed.returncode or installed.stdout.strip() != "test.manager-shutdown":
        raise AssertionError(f"fixture installation failed: {installed.stderr}")
    shutil.rmtree(package)
    harmless = Path(shutil.which("true") or sys.executable).resolve(strict=True)
    launch = command + ["launch", "desktop", "--codex-bin", str(harmless)]
    process = subprocess.Popen(
        launch,
        cwd=directory,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        start_new_session=True,
    )
    ready = threading.Event()
    stdout, stderr = [], []

    def collect(stream, lines):
        for line in stream:
            lines.append(line)
            if lines is stdout:
                try:
                    if json.loads(line).get("type") == "fixture/ready":
                        ready.set()
                except (json.JSONDecodeError, AttributeError):
                    pass

    readers = [
        threading.Thread(target=collect, args=(process.stdout, stdout), daemon=True),
        threading.Thread(target=collect, args=(process.stderr, stderr), daemon=True),
    ]
    for reader in readers:
        reader.start()
    state = home / "components/state/test.manager-shutdown"
    identity = None
    report = {
        "mode": "startup" if startup else "forced" if force else "graceful",
        "launch": launch,
        "manager_pid": process.pid,
    }
    started = time.monotonic()
    try:
        marker = state / ("starting.json" if startup else "started.json")
        wait_until(
            lambda: marker.exists() and (startup or ready.is_set()),
            "installed fixture phase",
            process,
        )
        announced = json.loads(marker.read_text())
        assert announced["parent_pid"] == process.pid, announced
        assert Path(announced["entrypoint"]).is_relative_to(
            home / "components/objects"
        ), announced
        identity = process_identity(announced["pid"])
        assert identity and fixture_alive(identity), identity
        report["fixture"] = {**announced, **identity}
        # The fixture holds its current phase while the signal wait is armed.
        time.sleep(0.1)
        process.send_signal(signal.SIGINT)
        report["first_sigint_seconds"] = time.monotonic() - started
        if not startup:
            wait_until(
                lambda: (state / "shutdown.json").exists(), "protocol shutdown", process
            )
            # Exceed the transport's ordinary 500ms post-result exit budget: a
            # requested graceful drain must retain the launcher's longer budget.
            time.sleep(1.0)
            assert process.poll() is None, (
                "first interrupt terminated the manager during drain"
            )
            assert fixture_alive(identity), (
                "first interrupt terminated the fixture during drain"
            )
            assert not (state / "finalized.json").exists(), (
                "fixture bypassed its drain gate"
            )
            report["blocked_drain_survived_seconds"] = 1.0
            if force:
                process.send_signal(signal.SIGINT)
                report["second_sigint_seconds"] = time.monotonic() - started
            else:
                (state / "release").write_text("finish accepted write")
        status = process.wait(timeout=10)
        for reader in readers:
            reader.join(timeout=2)
        report["exit_status"] = status
        report["stdout"] = "".join(stdout)
        report["stderr"] = "".join(stderr)
        wait_until(
            lambda: process_identity(identity["pid"]) is None,
            "fixture termination and reaping (zombies are failures)",
            timeout=5,
        )
        report["fixture_after"] = process_identity(identity["pid"])
        report["fixture_reaped"] = True
        if force or startup:
            assert status != 0, report
            diagnostic = report["stderr"].lower()
            assert all(
                word in diagnostic
                for word in ("accepted", "write", "durability", "unknown")
            ), report
            assert not (state / "finalized.json").exists(), (
                "forced fixture finished blocked write"
            )
            if startup:
                assert "before" in diagnostic and (
                    "ready" in diagnostic or "readiness" in diagnostic
                ), report
                assert not (state / "started.json").exists(), (
                    "delayed fixture passed readiness gate"
                )
        else:
            assert status == 0, report
            assert json.loads((state / "finalized.json").read_text()) == {
                "accepted_write": "durable"
            }
            assert {"drained": True, "fixture": "manager-shutdown"} in [
                json.loads(line) for line in stdout if line.strip()
            ], report
        report["passed"] = True
        return report
    finally:
        if identity:
            report.setdefault("fixture_after", process_identity(identity["pid"]))
            report.setdefault("fixture_reaped", report["fixture_after"] is None)
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        if identity and fixture_alive(identity):
            # Only the verified fixture process group created for this test.
            try:
                os.killpg(identity["pid"], signal.SIGKILL)
            except ProcessLookupError:
                pass
        for reader in readers:
            reader.join(timeout=2)
        report["stdout"] = "".join(stdout)
        report["stderr"] = "".join(stderr)
        report["exit_status"] = process.returncode
        commands.append(
            {
                "command": launch,
                "status": process.returncode,
                "stdout": report["stdout"],
                "stderr": report["stderr"],
            }
        )
        (directory / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
        (directory / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        process.stdout.close()
        process.stderr.close()


class ManagerShutdownTests(unittest.TestCase):
    def setUp(self):
        if not sys.platform.startswith("linux"):
            self.skipTest(
                "actual SIGINT/process identity acceptance currently runs on Linux"
            )
        configured = os.environ.get("CODEX_COMPONENT_MANAGER")
        if not configured:
            self.skipTest("set CODEX_COMPONENT_MANAGER to an already rebuilt manager")
        self.host = Path(configured).resolve(strict=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="codex-manager-shutdown-")
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name) / "case"
        self.before = fingerprint(self.host)

    def tearDown(self):
        self.assertEqual(fingerprint(self.host), self.before)

    def test_first_interrupt_waits_for_drain_and_returns_success(self):
        exercise(self.host, self.directory, force=False)

    def test_second_interrupt_forces_stop_with_unknown_durability(self):
        exercise(self.host, self.directory, force=True)

    def test_interrupt_before_ready_reports_unknown_durability_and_terminates(self):
        exercise(self.host, self.directory, force=False, startup=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()
    if not sys.platform.startswith("linux"):
        parser.error("this process/signal acceptance currently requires Linux")
    host = args.host.resolve(strict=True)
    work = args.work_dir.resolve()
    repository = Path(__file__).resolve().parents[2]
    if work == repository or repository in work.parents:
        parser.error("work directory must be outside the harness source")
    work.mkdir(parents=True, exist_ok=False)
    before = fingerprint(host)
    cases = [
        exercise(host, work / "graceful", force=False),
        exercise(host, work / "forced", force=True),
        exercise(host, work / "startup", force=False, startup=True),
    ]
    after = fingerprint(host)
    assert before == after, {"before": before, "after": after}
    report = {
        "passed": True,
        "tests": len(cases),
        "host": str(host),
        "host_before": before,
        "host_after": after,
        "cases": cases,
        "scope": "actual manager and installed protocol fixture; no desktop UI or real storage engine",
    }
    path = work / "acceptance.json"
    path.write_text(json.dumps(report, indent=2) + "\n")
    print(path)


if __name__ == "__main__":
    main()
