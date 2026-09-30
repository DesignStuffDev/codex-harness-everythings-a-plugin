"""Real-process checks for the recovery test runner's ownership and exit status."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


@unittest.skipUnless(sys.platform.startswith("linux"), "Linux subreaper")
class SubreaperRunnerTests(unittest.TestCase):
    def run_fixture(self, program):
        with tempfile.TemporaryDirectory(prefix="codex-subreaper-test-") as directory:
            report_path = Path(directory) / "report.json"
            completed = subprocess.run(
                [
                    sys.executable,
                    str(Path(__file__).with_name("subreaper_runner.py")),
                    "--report",
                    str(report_path),
                    "--",
                    sys.executable,
                    "-c",
                    program,
                ],
                capture_output=True,
                text=True,
                timeout=10,
            )
            return completed, json.loads(report_path.read_text())

    def test_failed_command_status_survives_later_successful_orphan(self):
        completed, report = self.run_fixture(
            "import os,time; pid=os.fork(); "
            "time.sleep(0.1) if pid == 0 else None; os._exit(0 if pid == 0 else 7)"
        )
        self.assertEqual(completed.returncode, 7, completed.stderr)
        self.assertEqual(report["command_returncode"], 7)
        self.assertIsNone(report["runner_error"])
        self.assertEqual(len(report["reaped"]), 2)
        self.assertEqual(report["reaped"][-1]["returncode"], 0)
        for child in report["reaped"]:
            self.assertFalse(Path(f"/proc/{child['pid']}").exists())

    def test_signal_status_is_not_reported_as_success(self):
        completed, report = self.run_fixture(
            "import os,signal; os.kill(os.getpid(), signal.SIGTERM)"
        )
        self.assertEqual(completed.returncode, 143, completed.stderr)
        self.assertEqual(report["command_returncode"], -15)
        self.assertIsNone(report["runner_error"])

    def test_success_waits_until_orphan_is_reaped(self):
        completed, report = self.run_fixture(
            "import os,time; pid=os.fork(); "
            "time.sleep(0.1) if pid == 0 else None; os._exit(0)"
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(report["command_returncode"], 0)
        self.assertIsNone(report["runner_error"])
        self.assertEqual(len(report["reaped"]), 2)
        for child in report["reaped"]:
            self.assertFalse(Path(f"/proc/{child['pid']}").exists())


if __name__ == "__main__":
    unittest.main()
