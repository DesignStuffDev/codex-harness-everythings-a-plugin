"""Direct-child custody unit checks; installed actual-Git acceptance is separate."""

import importlib.util
from pathlib import Path
import subprocess
import sys
import threading
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

PROJECT = Path(__file__).resolve().parents[1] / "examples" / "upstream-maintenance"
spec = importlib.util.spec_from_file_location("owned_git", PROJECT / "owned_git.py")
owned = importlib.util.module_from_spec(spec)
spec.loader.exec_module(owned)


class OwnedGitTests(unittest.TestCase):
    def setUp(self):
        self.stop = threading.Event()
        self.process = Mock(returncode=0)
        self.process.communicate.return_value = (b"commit\n", None)
        self.process.wait.return_value = -9

    def capture(self):
        return owned.capture(["--version"], env={}, stop=self.stop, deadline=100)

    def test_active_cancel_kills_and_reaps_before_returning(self):
        def interrupted_read(**_):
            self.stop.set()
            raise subprocess.TimeoutExpired("git", 0.1)

        self.process.communicate.side_effect = interrupted_read
        with (
            patch.object(owned.subprocess, "Popen", return_value=self.process),
            patch.object(owned.time, "monotonic", return_value=1),
        ):
            with self.assertRaisesRegex(
                owned.OwnedGitError, "cancelled_or_deadline_reached"
            ):
                self.capture()
        self.process.kill.assert_called_once_with()
        self.process.wait.assert_called_once_with(timeout=owned.CLEANUP_SECONDS)
        self.process.stdout.close.assert_called_once_with()

    def test_command_timeout_reaps_and_does_not_return_partial_output(self):
        self.process.communicate.side_effect = subprocess.TimeoutExpired("git", 0.1)
        with (
            patch.object(owned.subprocess, "Popen", return_value=self.process),
            patch.object(owned.time, "monotonic", side_effect=[1, 1, 32]),
        ):
            with self.assertRaisesRegex(owned.OwnedGitError, "local_object_timeout"):
                self.capture()
        self.process.kill.assert_called_once_with()
        self.process.wait.assert_called_once_with(timeout=owned.CLEANUP_SECONDS)

    def test_keyboard_interrupt_reaps_then_preserves_interrupt(self):
        self.process.communicate.side_effect = KeyboardInterrupt
        with (
            patch.object(owned.subprocess, "Popen", return_value=self.process),
            patch.object(owned.time, "monotonic", return_value=1),
        ):
            with self.assertRaises(KeyboardInterrupt):
                self.capture()
        self.process.kill.assert_called_once_with()
        self.process.wait.assert_called_once_with(timeout=owned.CLEANUP_SECONDS)

    def test_failed_reap_is_not_reported_as_successful_cleanup(self):
        self.process.communicate.side_effect = OSError("interrupted read")
        self.process.wait.side_effect = subprocess.TimeoutExpired("git", 2)
        with (
            patch.object(owned.subprocess, "Popen", return_value=self.process),
            patch.object(owned.time, "monotonic", return_value=1),
        ):
            with self.assertRaisesRegex(
                owned.OwnedGitError, "owned_git_cleanup_unconfirmed"
            ):
                self.capture()
        self.process.kill.assert_called_once_with()
        self.process.stdout.close.assert_called_once_with()

    def test_racing_exit_requires_reap_even_when_kill_fails(self):
        self.process.communicate.side_effect = OSError("read failed")
        self.process.kill.side_effect = ProcessLookupError
        with (
            patch.object(owned.subprocess, "Popen", return_value=self.process),
            patch.object(owned.time, "monotonic", return_value=1),
        ):
            with self.assertRaisesRegex(OSError, "read failed"):
                self.capture()
        self.process.wait.assert_called_once_with(timeout=owned.CLEANUP_SECONDS)

    def test_parent_change_while_arming_never_executes_git(self):
        libc = SimpleNamespace(prctl=Mock(return_value=0))
        ctypes = SimpleNamespace(CDLL=Mock(return_value=libc), c_int=int, c_ulong=int)
        with (
            patch.object(owned.sys, "argv", ["-c", "1234", "--version"]),
            patch.object(owned.sys, "platform", "linux"),
            patch.object(owned.os, "getppid", side_effect=[1234, 9999]),
            patch.object(owned.os, "_exit", side_effect=SystemExit(125)) as exit_now,
            patch.object(owned.os, "execvpe") as execute,
            patch.dict(sys.modules, {"ctypes": ctypes}),
        ):
            with self.assertRaises(SystemExit) as stopped:
                owned.child_main()
        self.assertEqual(stopped.exception.code, 125)
        exit_now.assert_called_once_with(125)
        execute.assert_not_called()
        libc.prctl.assert_called_once()


if __name__ == "__main__":
    unittest.main()
