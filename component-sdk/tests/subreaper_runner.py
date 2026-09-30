#!/usr/bin/env python3
"""Run a Linux test command with an init-like owner for orphaned descendants.

Usage: python3 subreaper_runner.py --report /tmp/run.json -- just test ...

The command's environment is unchanged. This only supplies the orphan-reaping
behavior normally provided by init; it does not alter tests or production code.
The command's actual wait status is retained even when an orphan exits later.
After the command exits, a bounded drain reports remaining children as a runner
failure without killing them. Interrupts are forwarded to the command's group.
"""

import argparse
import ctypes
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--drain-seconds", type=float, default=5.0)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not sys.platform.startswith("linux"):
        parser.error("PR_SET_CHILD_SUBREAPER requires Linux")
    if not command:
        parser.error("provide a command after --")
    if args.drain_seconds < 0:
        parser.error("--drain-seconds must not be negative")
    if args.report.exists():
        parser.error("report already exists; choose a new path")
    args.report.parent.mkdir(parents=True, exist_ok=True)

    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
    libc.prctl.restype = ctypes.c_int
    if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error))

    report = {"command": command, "subreaper": True, "reaped": []}
    started = time.monotonic()
    # This module is the only waiter. Popen.poll()/wait() must not compete with
    # waitpid(-1), which also receives adopted descendants.
    child = subprocess.Popen(command, start_new_session=True)
    report["command_pid"] = child.pid

    def forward(signum, _frame):
        try:
            os.killpg(child.pid, signum)
        except ProcessLookupError:
            pass

    previous_handlers = {
        signum: signal.signal(signum, forward)
        for signum in (signal.SIGINT, signal.SIGTERM)
    }
    command_status = None
    drain_deadline = None
    runner_error = None
    try:
        while True:
            try:
                pid, status = os.waitpid(-1, os.WNOHANG)
            except InterruptedError:
                continue
            except ChildProcessError:
                if command_status is None:
                    runner_error = "command wait status was not observed"
                break
            if pid:
                returncode = os.waitstatus_to_exitcode(status)
                report["reaped"].append(
                    {"pid": pid, "wait_status": status, "returncode": returncode}
                )
                if pid == child.pid:
                    command_status = status
                    child.returncode = returncode
                    drain_deadline = time.monotonic() + args.drain_seconds
                continue
            if drain_deadline is not None and time.monotonic() >= drain_deadline:
                runner_error = "descendants remain after the command exited"
                break
            time.sleep(0.01)
    finally:
        for signum, handler in previous_handlers.items():
            signal.signal(signum, handler)

    command_returncode = (
        os.waitstatus_to_exitcode(command_status)
        if command_status is not None
        else None
    )
    # Signals use the conventional shell status. Never turn a missing status or
    # a failed descendant drain into a successful command result.
    exit_status = (
        128 - command_returncode
        if command_returncode is not None and command_returncode < 0
        else command_returncode
    )
    if exit_status is None or (runner_error is not None and exit_status == 0):
        exit_status = 125
    report.update(
        command_wait_status=command_status,
        command_returncode=command_returncode,
        runner_error=runner_error,
        exit_status=exit_status,
        elapsed_seconds=time.monotonic() - started,
    )
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    if runner_error:
        print(f"Subreaper runner: {runner_error}; see {args.report}", file=sys.stderr)
    return exit_status


if __name__ == "__main__":
    sys.exit(main())
