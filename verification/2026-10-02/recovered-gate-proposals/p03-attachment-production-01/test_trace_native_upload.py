"""Synthetic parser checks only; no strace, native processes or imports of fixtures."""

from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import trace_native_upload as m

NATIVE = Path("/synthetic/package/native")
STAGING = Path("/synthetic/state")
EXEC = 'execve("/synthetic/package/native", ["native"], 0x1 /* 0 vars */) = 0\n'
OPEN = 'openat(AT_FDCWD, "/synthetic/state/.attachment-transfer-abc/input.bin", O_RDONLY|O_CLOEXEC) = 3\n'
EXIT = 'exit_group(0) = ?\n'
CLONE = 'clone(child_stack=NULL, flags=CLONE_VM|CLONE_THREAD|CLONE_SIGHAND) = 20\n'


class TraceTest(unittest.TestCase):
    def inspect(self, files):
        with tempfile.TemporaryDirectory() as directory:
            prefix = Path(directory) / "trace"
            for suffix, text in files.items():
                (Path(directory) / ("trace." + suffix)).write_bytes(text.encode())
            return m.inspect_trace(prefix, NATIVE, STAGING)

    def reject(self, files, code):
        with self.assertRaisesRegex(m.TraceError, code):
            self.inspect(files)

    def test_direct_native_upload(self):
        result = self.inspect({"10": EXEC + OPEN + EXIT})
        self.assertTrue(result["native_upload_path_invoked"])
        self.assertFalse(result["native_result_envelope_proven"])
        self.assertFalse(result["whole_host_clean"])
        self.assertEqual(result["staged_read_open_count"], 1)
        self.assertNotIn("/synthetic", repr(result))
        self.assertNotIn("pid", repr(result))

    def test_owned_thread_and_nested_clone3(self):
        result = self.inspect({"10": EXEC + CLONE + EXIT,
            "20": 'clone3({flags=CLONE_VM|CLONE_THREAD, exit_signal=0}, 88) = 30\n', "30": OPEN})
        self.assertEqual(result["owned_thread_count"], 2)

    def test_fork_is_not_native_thread(self):
        for birth in ('fork() = 20\n', 'clone(child_stack=NULL, flags=SIGCHLD) = 20\n'):
            self.reject({"10": EXEC + birth + EXIT, "20": OPEN}, "staged_read_missing")

    def test_pre_exec_open_and_thread_are_excluded(self):
        self.reject({"10": OPEN + CLONE + EXEC + EXIT, "20": OPEN}, "staged_read_missing")

    def test_unfinished_and_resumed(self):
        result = self.inspect({"10": EXEC + CLONE.replace(') = 20\n', ' <unfinished ...>\n')
            + '<... clone resumed>) = 20\n' + EXIT,
            "20": OPEN.replace(') = 3\n', ' <unfinished ...>\n') + '<... openat resumed>) = 3\n'})
        self.assertEqual(result["staged_read_open_count"], 1)

    def test_unmatched_unfinished_and_resumed_fail(self):
        for extra in ('openat( <unfinished ...>\n', '<... openat resumed>) = 3\n'):
            self.reject({"10": EXEC + OPEN + EXIT + extra}, "unmatched")

    def test_missing_duplicate_or_wrong_native_exec(self):
        self.reject({"10": OPEN + EXIT}, "exec_count")
        self.reject({"10": EXEC + OPEN + EXIT, "20": EXEC + OPEN + EXIT}, "exec_count")
        self.reject({"10": EXEC.replace(' = 0', ' = -1 ENOENT (No such file)') + OPEN + EXIT}, "exec_count")

    def test_exit_missing_failed_or_followed_by_work(self):
        for ending, code in (("", "exit_missing"), (EXIT.replace('(0)', '(1)'), "native_exit"),
                             (EXIT + OPEN, "native_exit")):
            self.reject({"10": EXEC + OPEN + ending}, code)

    def test_wrong_root_and_failed_or_writable_open(self):
        self.reject({"10": EXEC + OPEN.replace('/synthetic/state/', '/elsewhere/') + EXIT}, "staged_read_missing")
        self.reject({"10": EXEC + OPEN.replace(' = 3', ' = -1 EIO (error)') + EXIT}, "staged_read_missing")
        self.reject({"10": EXEC + OPEN.replace('O_RDONLY', 'O_RDWR') + EXIT}, "not_readonly")

    def test_truncation_partial_line_and_missing_thread(self):
        self.reject({"10": EXEC.replace('native", [', 'native"..., [') + OPEN + EXIT}, "path_truncated")
        self.reject({"10": EXEC + OPEN + EXIT.rstrip()}, "partial_final_line")
        self.reject({"10": EXEC + CLONE + OPEN + EXIT}, "child_file_missing")

    def test_all_bounds(self):
        for key, limit, files, code in (
            ("MAX_FILES", 1, {"10": EXEC + OPEN + EXIT, "20": ""}, "file_limit"),
            ("MAX_BYTES", 1, {"10": EXEC + OPEN + EXIT}, "byte_limit"),
            ("MAX_LINES", 2, {"10": EXEC + OPEN + EXIT}, "line_limit"),
        ):
            with self.subTest(key=key), patch.object(m, key, limit):
                self.reject(files, code)

    def test_execveat_and_nonthread_noise(self):
        alternate = EXEC.replace('execve(', 'execveat(AT_FDCWD, ').replace(') = 0', ', 0) = 0')
        self.assertTrue(self.inspect({"10": alternate + OPEN + EXIT,
            "20": 'fork() = 30\n', "30": ""})["native_upload_path_invoked"])

    def test_conflicting_termination_and_reused_identity(self):
        self.reject({"10": EXEC + OPEN + EXIT + '+++ killed by SIGKILL +++\n'}, "native_exit")
        self.reject({"10": EXEC + CLONE + CLONE + EXIT, "20": OPEN}, "reused_identity")


if __name__ == "__main__":
    unittest.main()
