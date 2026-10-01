"""Evidence helpers for the staged Linux file-search acceptance gate."""

import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import time


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def fingerprint(path):
    path = Path(path)
    info = path.stat()
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {
        "sha256": digest,
        "bytes": info.st_size,
        "mtime_ns": info.st_mtime_ns,
        "mode": stat.S_IMODE(info.st_mode),
    }


def inventory(directory):
    result = {}
    for path in sorted(directory.rglob("*")):
        mode = path.lstat().st_mode
        require(
            stat.S_ISDIR(mode) or stat.S_ISREG(mode),
            "artifact contains a symlink or special file",
        )
        if stat.S_ISREG(mode):
            result[path.relative_to(directory).as_posix()] = fingerprint(path)
    return result


def same_content(left, right):
    """An install copies content and execute bits, not source timestamps."""
    return {
        name: (data["sha256"], data["bytes"], data["mode"])
        for name, data in left.items()
    } == {
        name: (data["sha256"], data["bytes"], data["mode"])
        for name, data in right.items()
    }


def write_report(path, report):
    temporary = path.with_suffix(".pending")
    temporary.write_text(json.dumps(report, indent=2) + "\n")
    temporary.replace(path)


def process_snapshot():
    """Read identity/parent/executable only; never command lines or environment."""
    result = {}
    for path in Path("/proc").iterdir():
        if not path.name.isdecimal():
            continue
        try:
            fields = (path / "stat").read_text().rsplit(") ", 1)[1].split()
            result[int(path.name)] = {
                "pid": int(path.name),
                "parent": int(fields[1]),
                "group": int(fields[2]),
                "start_ticks": int(fields[19]),
                "state": fields[0],
            }
            try:
                result[int(path.name)]["executable"] = os.readlink(path / "exe")
            except (FileNotFoundError, PermissionError, ProcessLookupError):
                pass
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            continue
    return result


def is_pid_present(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


class Runner:
    """Retain bounded logs and require all observed descendant PIDs absent.

    The unchanged repository subreaper must wrap the acceptance script. It owns
    adopted orphans. No zombie exemption, hidden retry, or forced-cleanup pass.
    Commands get private process groups; workers may create their own groups.
    Emergency signals affect only the command and identity-checked descendants.
    """

    def __init__(self, directory, report, report_path):
        self.directory, self.report, self.report_path = directory, report, report_path
        self.labels = set()

    def run(
        self,
        label,
        argv,
        *,
        cwd,
        environment=None,
        timeout=120,
        expected_status="zero",
        interrupt_when_executable=None,
    ):
        require(label not in self.labels, "duplicate evidence label")
        self.labels.add(label)
        stdout, stderr = (
            self.directory / f"{label}.{suffix}" for suffix in ("stdout", "stderr")
        )
        entry = {
            "label": label,
            "argv": list(map(str, argv)),
            "cwd": str(cwd),
            "stdout": str(stdout),
            "stderr": str(stderr),
        }
        self.report["commands"].append(entry)
        write_report(self.report_path, self.report)
        started = time.monotonic()
        tracked = {}
        timed_out = False
        forced = False
        interrupted = False
        with stdout.open("w") as output, stderr.open("w") as errors:
            child = subprocess.Popen(
                entry["argv"],
                cwd=cwd,
                env=environment,
                stdin=subprocess.DEVNULL,
                stdout=output,
                stderr=errors,
                start_new_session=True,
            )
            entry["pid"] = child.pid
            try:
                while True:
                    snapshot = process_snapshot()
                    parents = {child.pid, *tracked}
                    while True:
                        children = {
                            pid
                            for pid, item in snapshot.items()
                            if item["parent"] in parents
                        }
                        expanded = parents | children
                        if expanded == parents:
                            break
                        parents = expanded
                    for pid in parents:
                        if pid in snapshot:
                            previous = tracked.get(pid, {})
                            tracked[pid] = {**previous, **snapshot[pid]}
                    if (
                        interrupt_when_executable is not None
                        and not interrupted
                        and any(
                            item.get("executable") == str(interrupt_when_executable)
                            for item in tracked.values()
                        )
                    ):
                        # Signal only the real CLI. Its lifecycle must drain the
                        # worker; sending SIGINT to the worker would hide that.
                        os.kill(child.pid, signal.SIGINT)
                        interrupted = True
                        entry["requested_cli_sigint_after_observed_worker"] = True
                    if child.poll() is not None:
                        break
                    if time.monotonic() - started > timeout:
                        timed_out = True
                        self._signal(child, tracked, signal.SIGINT)
                        try:
                            child.wait(timeout=30)
                        except subprocess.TimeoutExpired:
                            forced = True
                            self._signal(child, tracked, signal.SIGKILL)
                            child.wait(timeout=10)
                        break
                    time.sleep(0.002)
            except BaseException:
                # This is failure cleanup, never evidence of graceful shutdown.
                forced = True
                self._signal(child, tracked, signal.SIGKILL)
                child.wait(timeout=10)
                raise
            finally:
                entry.update(
                    status=child.returncode,
                    timed_out=timed_out,
                    emergency_forced_cleanup=forced,
                    elapsed_seconds=time.monotonic() - started,
                )
                entry["observed_processes"] = list(tracked.values())
                deadline = time.monotonic() + 10
                while (
                    any(is_pid_present(pid) for pid in tracked)
                    and time.monotonic() < deadline
                ):
                    time.sleep(0.01)
                remaining = [pid for pid in tracked if is_pid_present(pid)]
                entry["remaining_pids"] = remaining
                entry["all_observed_pids_absent"] = not remaining
                entry["stdout_fingerprint"] = fingerprint(stdout)
                entry["stderr_fingerprint"] = fingerprint(stderr)
                write_report(self.report_path, self.report)
        require(
            not timed_out and not forced, f"{label}: timed out or needed forced cleanup"
        )
        require(not remaining, f"{label}: observed processes remain")
        if expected_status == "zero":
            require(child.returncode == 0, f"{label}: nonzero status; see saved stderr")
        elif expected_status == "nonzero":
            require(
                child.returncode > 0,
                f"{label}: expected explicit error exit, not success/signal",
            )
        elif type(expected_status) is int:
            require(
                child.returncode == expected_status,
                f"{label}: expected status {expected_status}, got {child.returncode}",
            )
        else:
            raise ValueError("invalid expected status")
        require(
            interrupt_when_executable is None or interrupted,
            f"{label}: worker was never observed before interruption",
        )
        return stdout.read_text(), stderr.read_text(), entry

    @staticmethod
    def _signal(child, tracked, signum):
        snapshot = process_snapshot()
        groups = {child.pid}
        for pid, prior in tracked.items():
            current = snapshot.get(pid)
            if current and current["start_ticks"] == prior["start_ticks"]:
                groups.add(current["group"])
        for group in groups:
            # The acceptance owner and subreaper are never signal targets.
            if group == os.getpgrp():
                continue
            try:
                os.killpg(group, signum)
            except ProcessLookupError:
                pass
