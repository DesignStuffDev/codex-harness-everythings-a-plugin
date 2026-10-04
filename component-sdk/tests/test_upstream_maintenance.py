"""Adapter-boundary tests; existing planner/lineage suites remain separate gates."""

import hashlib
import importlib.util
from pathlib import Path
import sys
import tempfile
import threading
from types import SimpleNamespace
import unittest
from unittest.mock import patch

PROJECT = Path(__file__).resolve().parents[1] / "examples" / "upstream-maintenance"
sys.path.insert(0, str(PROJECT))
spec = importlib.util.spec_from_file_location(
    "maintenance_review", PROJECT / "maintenance_review.py"
)
review = importlib.util.module_from_spec(spec)
spec.loader.exec_module(review)
sys.modules["maintenance_review"] = review
plugin_spec = importlib.util.spec_from_file_location(
    "maintenance_plugin", PROJECT / "plugin.py"
)
plugin = importlib.util.module_from_spec(plugin_spec)
plugin_spec.loader.exec_module(plugin)
sys.path.pop(0)


class InstalledImpactBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.index = self.root / "index.json"
        self.index.write_bytes(b"{}")
        self.request = {
            "contract_version": 1,
            "repository": str(self.root),
            "lineage_index": str(self.index),
            "lineage_sha256": hashlib.sha256(b"{}").hexdigest(),
            "candidate_revision": "1" * 40,
            "composition_revision": "2" * 40,
        }

    def test_wrong_digest_prevents_git_adapter_creation(self):
        self.request["lineage_sha256"] = "0" * 64
        with patch.object(review, "LocalObjects") as objects:
            result = review.review_request(self.request)
        objects.assert_not_called()
        self.assertEqual(result["diagnostic_code"], "input_digest_mismatch")
        self.assertFalse(result["update_allowed"])

    def test_unresolved_report_has_stable_content_id_and_never_approves(self):
        report = {
            "status": "unresolved",
            "invalid": 0,
            "unresolved": ["ownership_unknown"],
            "changed_paths": [],
        }
        with patch.object(review.planner, "plan", return_value=report):
            first = review.review_request(self.request)
            second = review.review_request(self.request)
        self.assertEqual(first, second)
        self.assertEqual(first["status"], "review_required")
        self.assertEqual(first["report"]["unresolved"], ["ownership_unknown"])
        self.assertFalse(first["update_allowed"])

    def test_bootstrap_full_report_preserves_identity_and_blocking(self):
        report = {
            "status": "unresolved",
            "invalid": 0,
            "unresolved": ["semantic_impact_not_reviewed"],
            "changed_paths": [],
            "detail": "x" * 10000,
        }
        with patch.object(review.planner, "plan", return_value=report):
            bounded = review.review_request(self.request)
            full = review.review_request(self.request, full_report=True)
        self.assertTrue(bounded["detailed_report_omitted"])
        self.assertLessEqual(len(review.canonical(bounded)), review.MAX_RESULT)
        self.assertEqual(bounded["plan_id"], full["plan_id"])
        self.assertEqual(full["report"], report)
        self.assertFalse(full["update_allowed"])

    def test_packaged_planner_change_invalidates_plan_identity(self):
        report = {"status": "unresolved", "invalid": 0, "changed_paths": []}
        loader = review.planner.__loader__
        changed = loader.get_data(review.planner.__file__) + b"\n"
        with patch.object(review.planner, "plan", return_value=report):
            before = review.review_request(self.request)
            with (
                patch.object(loader, "get_data", return_value=changed),
                patch.dict(
                    review.TOOL_HASHES,
                    {"plan_upstream_impact.py": hashlib.sha256(changed).hexdigest()},
                ),
            ):
                after = review.review_request(self.request)
        self.assertEqual(before["status"], "review_required")
        self.assertEqual(after["status"], "review_required")
        self.assertEqual(before["report_sha256"], after["report_sha256"])
        self.assertNotEqual(before["plan_id"], after["plan_id"])

    def test_invalid_report_stays_invalid_when_details_are_omitted(self):
        report = {
            "status": "invalid",
            "invalid": 1,
            "unresolved": [],
            "detail": "x" * 10000,
        }
        with patch.object(review.planner, "plan", return_value=report):
            result = review.review_request(self.request)
        self.assertEqual(result["status"], "invalid")
        self.assertTrue(result["detailed_report_omitted"])
        self.assertFalse(result["update_allowed"])

    def test_publication_path_requires_its_digest(self):
        self.request["publication_receipt"] = str(self.root / "receipt.json")
        with patch.object(review, "LocalObjects") as objects:
            result = review.review_request(self.request)
        objects.assert_not_called()
        self.assertEqual(
            result["diagnostic_code"], "publication_path_and_digest_required_together"
        )

    def test_cancellation_blocks_new_git_admission(self):
        stop = threading.Event()
        stop.set()
        objects = review.LocalObjects(self.root, stop)
        with patch.object(review.owned_git.subprocess, "Popen") as command:
            with self.assertRaisesRegex(
                review.owned_git.OwnedGitError, "cancelled_or_deadline_reached"
            ):
                objects.command("cat-file", "-t", "1" * 40)
        command.assert_not_called()

    def test_terminal_custody_failures_escape_planner_recovery(self):
        def recover_value_errors(index, objects, *args):
            for _ in range(2):
                try:
                    objects.command("cat-file", "-t", "1" * 40)
                except ValueError:
                    continue
            return {"status": "unresolved", "invalid": 0, "changed_paths": []}

        for code, status in (
            ("owned_git_cleanup_unconfirmed", "invalid"),
            ("cancelled_or_deadline_reached", "cancelled"),
            ("local_object_timeout", "cancelled"),
        ):
            with (
                self.subTest(code=code),
                patch.object(review.planner, "plan", side_effect=recover_value_errors),
                patch.object(
                    review.owned_git,
                    "capture",
                    side_effect=review.owned_git.OwnedGitError(code),
                ) as command,
            ):
                result = review.review_request(self.request)
            self.assertEqual(result["status"], status)
            self.assertEqual(result["diagnostic_code"], code)
            self.assertNotIn("report", result)
            self.assertFalse(result["update_allowed"])
            command.assert_called_once()

    def test_terminal_object_owner_does_not_admit_more_commands(self):
        objects = review.LocalObjects(self.root, threading.Event())
        with patch.object(
            review.owned_git,
            "capture",
            side_effect=review.owned_git.OwnedGitError("owned_git_cleanup_unconfirmed"),
        ) as command:
            for _ in range(2):
                with self.assertRaisesRegex(
                    review.owned_git.OwnedGitError, "owned_git_cleanup_unconfirmed"
                ):
                    objects.command("cat-file", "-t", "1" * 40)
        command.assert_called_once()

    def test_cancelled_report_is_not_success(self):
        stop = threading.Event()
        stop.set()
        with patch.object(
            review.planner,
            "plan",
            return_value={"status": "unresolved", "invalid": 0, "changed_paths": []},
        ):
            result = review.review_request(self.request, stop)
        self.assertEqual(result["status"], "cancelled")
        self.assertFalse(result["update_allowed"])

    def test_real_tool_v1_envelope_calls_review_and_watches_shutdown(self):
        stop = threading.Event()
        context = SimpleNamespace(watch_shutdown=lambda: stop)
        result = {"status": "review_required", "update_allowed": False}
        with patch.object(plugin, "review_request", return_value=result) as invoke:
            tool = plugin.plan(
                {
                    "call_id": "test-call",
                    "name": "upstream_impact_review",
                    "arguments": self.request,
                },
                context,
            )
        invoke.assert_called_once_with(self.request, stop)
        self.assertTrue(tool["success"])
        self.assertEqual(review.lineage.json_bytes(tool["text"].encode()), result)

    def test_incomplete_or_wrong_tool_envelope_is_rejected(self):
        context = SimpleNamespace(watch_shutdown=lambda: threading.Event())
        for params in (
            {"arguments": self.request},
            {"call_id": "test", "name": "different", "arguments": self.request},
        ):
            with self.subTest(params=params):
                with self.assertRaises(plugin.PluginError):
                    plugin.plan(params, context)

    def test_failed_partial_report_assembly_resets_status_and_output(self):
        report = {
            "status": "unresolved",
            "invalid": 0,
            "detail": "x" * 10000,
            "unresolved": 3,
        }
        with patch.object(review.planner, "plan", return_value=report):
            result = review.review_request(self.request)
        self.assertEqual(result["status"], "invalid")
        self.assertNotIn("report", result)
        self.assertLessEqual(len(review.canonical(result)), review.MAX_RESULT)


if __name__ == "__main__":
    unittest.main()
