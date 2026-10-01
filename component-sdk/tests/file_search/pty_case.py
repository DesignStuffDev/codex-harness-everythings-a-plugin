"""Linux real-CLI PTY owner with strict descendant accounting.

The caller loads the unchanged SDK acceptance_support module before importing us.
This process owns its direct child; the outer unchanged subreaper owns orphans.
"""

import errno
import fcntl
import os
from pathlib import Path
import select
import signal
import struct
import termios
import time

from acceptance_support import (
    fingerprint,
    is_pid_present,
    process_snapshot,
    require,
    write_report,
)
from terminal import Screen


class PtyCase:
    def __init__(self, runner, label, argv, cwd, environment, worker=None):
        self.runner, self.label, self.worker = runner, label, worker
        self.screen = Screen()
        self.tracked, self.stopped = {}, set()
        self.status, self.forced, self.eof = None, False, False
        self.started = time.monotonic()
        self.transcript = runner.directory / (label + ".pty")
        self.output = self.transcript.open("xb")
        self.bytes = 0
        self.entry = {
            "label": label,
            "argv": list(map(str, argv)),
            "cwd": str(cwd),
            "transcript": str(self.transcript),
            "steps": [],
            "screens": [],
        }
        runner.report["pty_cases"].append(self.entry)
        self.pid, self.fd = os.forkpty()
        if self.pid == 0:
            try:
                os.chdir(cwd)
                fcntl.ioctl(
                    0,
                    termios.TIOCSWINSZ,
                    struct.pack("HHHH", self.screen.rows, self.screen.cols, 0, 0),
                )
                os.execve(str(argv[0]), list(map(str, argv)), environment)
            except BaseException:
                os.write(2, b"Acceptance child could not exec the supplied CLI\n")
                os._exit(127)
        os.set_blocking(self.fd, False)
        self.entry["pid"] = self.pid
        self.save()

    def save(self):
        write_report(self.runner.report_path, self.runner.report)

    def sample(self):
        snapshot = process_snapshot()
        parents = {self.pid, *self.tracked}
        while True:
            expanded = parents | {
                pid for pid, info in snapshot.items() if info["parent"] in parents
            }
            if expanded == parents:
                break
            parents = expanded
        for pid in parents:
            if pid in snapshot:
                old = self.tracked.get(pid)
                require(
                    old is None or old["start_ticks"] == snapshot[pid]["start_ticks"],
                    "tracked PID was reused",
                )
                self.tracked[pid] = {**(old or {}), **snapshot[pid]}
        if self.status is None:
            pid, status = os.waitpid(self.pid, os.WNOHANG)
            if pid:
                self.status = os.waitstatus_to_exitcode(status)
        return snapshot

    def pump(self, seconds=0.02):
        snapshot = self.sample()
        if not self.eof and select.select([self.fd], [], [], seconds)[0]:
            try:
                data = os.read(self.fd, 65536)
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
                data = b""
            if not data:
                self.eof = True
            else:
                self.bytes += len(data)
                require(
                    self.bytes <= 16 * 1024 * 1024,
                    "PTY transcript exceeded 16 MiB evidence budget",
                )
                self.output.write(data)
                self.output.flush()
                reply = self.screen.feed(data)
                if reply:
                    self.write(reply, label="terminal-query-reply", record=False)
        elif self.eof:
            time.sleep(min(seconds, 0.02))
        return snapshot

    def write(self, data, *, label, record=True):
        require(self.status is None, f"{label}: CLI already exited")
        if record:
            self.entry["steps"].append(
                {
                    "action": label,
                    "bytes": len(data),
                    "elapsed_seconds": time.monotonic() - self.started,
                }
            )
        pending = memoryview(data)
        deadline = time.monotonic() + 5
        while pending:
            try:
                pending = pending[os.write(self.fd, pending) :]
            except BlockingIOError:
                require(time.monotonic() < deadline, "PTY input remained blocked")
                select.select([], [self.fd], [], 0.05)

    def wait(self, label, predicate, seconds=40):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            self.pump()
            require(
                not self.screen.unknown,
                "unsupported VT screen mutation: " + repr(sorted(self.screen.unknown)),
            )
            if predicate():
                return self.capture(label)
            require(
                self.status is None,
                f"{label}: CLI exited {self.status} before expected screen",
            )
        self.capture(label + "-timeout")
        raise AssertionError(
            label + ": timed out; inspect saved current-screen and raw PTY"
        )

    def capture(self, label):
        path = self.runner.directory / f"{self.label}-{label}.screen.txt"
        require(not path.exists(), "duplicate screen artifact")
        path.write_text(self.screen.text() + "\n")
        item = {
            "label": label,
            "path": str(path),
            "fingerprint": fingerprint(path),
            "elapsed_seconds": time.monotonic() - self.started,
        }
        self.entry["screens"].append(item)
        self.save()
        return self.screen.text()

    def hold(self, seconds, predicate, message):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            self.pump()
            require(self.status is None, "CLI exited during observed hold")
            require(predicate(), message)

    def installed_worker(self):
        snapshot = self.sample()
        identities = [
            item
            for item in self.tracked.values()
            if item.get("executable") == str(self.worker)
        ]
        require(
            len(identities) == 1,
            "expected exactly one selected worker over this whole TUI lifetime",
        )
        info = identities[0]
        now = snapshot.get(info["pid"])
        require(
            now is not None
            and now["start_ticks"] == info["start_ticks"]
            and now.get("executable") == str(self.worker),
            "selected worker is no longer present",
        )
        return info

    def pause_worker(self):
        info = self.installed_worker()
        os.kill(info["pid"], signal.SIGSTOP)
        self.stopped.add(info["pid"])
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            self.pump()
            if self.tracked[info["pid"]]["state"] in ("T", "t"):
                break
        require(self.tracked[info["pid"]]["state"] in ("T", "t"), "worker did not stop")
        self.entry["steps"].append(
            {
                "action": "SIGSTOP-selected-worker",
                "pid": info["pid"],
                "start_ticks": info["start_ticks"],
            }
        )

    def resume_workers(self):
        snapshot = process_snapshot()
        for pid in tuple(self.stopped):
            old, now = self.tracked.get(pid), snapshot.get(pid)
            if old and now and old["start_ticks"] == now["start_ticks"]:
                try:
                    os.kill(pid, signal.SIGCONT)
                except ProcessLookupError:
                    pass  # exit raced the identity snapshot
                self.entry["steps"].append(
                    {"action": "SIGCONT-selected-worker", "pid": pid}
                )
            self.stopped.discard(pid)

    def await_exit(self, expected=0, seconds=200):
        deadline = time.monotonic() + seconds
        while self.status is None and time.monotonic() < deadline:
            self.pump()
        require(self.status is not None, "normal TUI shutdown timed out")
        for _ in range(8):
            if self.eof:
                break
            self.pump()
        if expected == "nonzero":
            require(
                self.status > 0, "expected explicit startup failure, not success/signal"
            )
        else:
            require(
                self.status == expected, f"expected exit {expected}, got {self.status}"
            )

    def finish(self, failure=None):
        self.resume_workers()
        if self.status is None:
            self.forced = True
            snapshot = self.sample()
            for pid, prior in self.tracked.items():
                now = snapshot.get(pid)
                if now and now["start_ticks"] == prior["start_ticks"]:
                    try:
                        os.kill(pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
            if self.status is None:
                _, status = os.waitpid(self.pid, 0)
                self.status = os.waitstatus_to_exitcode(status)
        deadline = time.monotonic() + 10
        while (
            any(is_pid_present(pid) for pid in self.tracked)
            and time.monotonic() < deadline
        ):
            time.sleep(0.01)
        remaining = [pid for pid in self.tracked if is_pid_present(pid)]
        before_emergency = list(remaining)
        if remaining:
            # A direct CLI exit is not a clean descendant receipt. Preserve the
            # failed observation, then clean up identity-checked stragglers even
            # when the CLI was already reaped; the outer subreaper never kills.
            self.forced = True
            snapshot = process_snapshot()
            for pid in remaining:
                old, now = self.tracked[pid], snapshot.get(pid)
                if now and old["start_ticks"] == now["start_ticks"]:
                    try:
                        os.kill(pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
            deadline = time.monotonic() + 5
            while (
                any(is_pid_present(pid) for pid in remaining)
                and time.monotonic() < deadline
            ):
                time.sleep(0.01)
            remaining = [pid for pid in remaining if is_pid_present(pid)]
        self.output.close()
        os.close(self.fd)
        self.entry.update(
            status=self.status,
            emergency_forced_cleanup=self.forced,
            failure=str(failure) if failure else None,
            observed_processes=list(self.tracked.values()),
            remaining_before_emergency_cleanup=before_emergency,
            remaining_pids=remaining,
            all_observed_pids_absent=not remaining,
            unknown_screen_mutations=sorted(self.screen.unknown),
            elapsed_seconds=time.monotonic() - self.started,
            transcript_fingerprint=fingerprint(self.transcript),
        )
        self.save()
        require(
            not self.forced, "TUI case required emergency cleanup; never a passing run"
        )
        require(
            not remaining, "TUI case has remaining descendant PIDs (including zombies)"
        )
