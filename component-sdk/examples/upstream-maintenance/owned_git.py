"""Bounded direct-Git custody; Linux parent-death protection, not tree isolation."""

import os
from pathlib import Path
import signal
import subprocess
import sys
import time

COMMAND_SECONDS = 30
POLL_SECONDS = 0.1
CLEANUP_SECONDS = 2
LOADER = (
    "import sys; sys.path.insert(0, sys.argv.pop(1)); "
    "from owned_git import child_main; child_main()"
)


class OwnedGitError(RuntimeError):
    """Stable diagnostic without command output or environment values."""


def child_main():
    """Arm after fresh Python exec, then replace this process with actual Git."""
    expected_parent = int(sys.argv[1])
    if os.getppid() != expected_parent:
        os._exit(125)
    if sys.platform == "linux":
        # No preexec_fn: the plugin already has the SDK shutdown watcher thread.
        import ctypes

        try:
            libc = ctypes.CDLL(None, use_errno=True)
            libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
            libc.prctl.restype = ctypes.c_int
            if libc.prctl(1, signal.SIGKILL, 0, 0, 0) != 0:  # PR_SET_PDEATHSIG
                os._exit(125)
        except (OSError, AttributeError):
            os._exit(125)
        # Parent may die before or while prctl is being armed.
        if os.getppid() != expected_parent:
            os._exit(125)
    os.execvpe("git", ["git", *sys.argv[2:]], os.environ)


def _kill_and_reap(process):
    """Confirm the direct child was reaped; never silently accept failed cleanup."""
    try:
        process.kill()
    except (OSError, KeyboardInterrupt):
        # A racing exit can make kill fail. A successful wait still proves reap.
        pass
    try:
        process.wait(timeout=CLEANUP_SECONDS)
    except (OSError, subprocess.TimeoutExpired, KeyboardInterrupt) as error:
        raise OwnedGitError("owned_git_cleanup_unconfirmed") from error


def capture(arguments, *, env, stop, deadline):
    """Run local Git with active cancellation and a bounded direct-child cleanup."""
    started = time.monotonic()
    if stop.is_set() or started >= deadline:
        raise OwnedGitError("cancelled_or_deadline_reached")
    command_deadline = min(deadline, started + COMMAND_SECONDS)
    process = subprocess.Popen(
        [
            sys.executable,
            "-I",
            "-B",
            "-c",
            LOADER,
            str(Path(__file__).parent),  # Directory or installed plugin.pyz.
            str(os.getpid()),
            *arguments,
        ],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        env=env,
    )
    try:
        while True:
            now = time.monotonic()
            if stop.is_set() or now >= deadline:
                raise OwnedGitError("cancelled_or_deadline_reached")
            if now >= command_deadline:
                raise OwnedGitError("local_object_timeout")
            try:
                stdout, _ = process.communicate(
                    timeout=min(POLL_SECONDS, command_deadline - now)
                )
                break
            except subprocess.TimeoutExpired:
                continue
        now = time.monotonic()
        if stop.is_set() or now >= deadline:
            raise OwnedGitError("cancelled_or_deadline_reached")
        if now >= command_deadline:
            raise OwnedGitError("local_object_timeout")
        return subprocess.CompletedProcess(arguments, process.returncode, stdout)
    except BaseException:
        _kill_and_reap(process)
        raise
    finally:
        # communicate closes this after success; close also on interrupted reads.
        try:
            process.stdout.close()
        except (OSError, KeyboardInterrupt) as error:
            raise OwnedGitError("owned_git_cleanup_unconfirmed") from error
