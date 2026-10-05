"""Bounded tool adapter fixtures; installed package proof remains separate."""

import json
from pathlib import Path
import sys
import threading
import unittest
from unittest.mock import patch

import test_selection_plan as fixtures

PROJECT = Path(__file__).resolve().parents[1] / "examples" / "upstream-maintenance"
sys.path.insert(0, str(PROJECT))
import selection_tools as adapter

sys.path.pop(0)


class SelectionToolsTests(unittest.TestCase):
    def setUp(self):
        self.fixture = fixtures.SelectionPlanTests()
        self.fixture.setUp()
        self.stop = threading.Event()
        self.inputs = dict(
            graph=self.fixture.graph,
            profile=self.fixture.profile,
            policy=self.fixture.scopes,
            review=self.fixture.review,
        )
        self.request = dict(contract_version=1, purpose="plan_selection", **self.inputs)

    def call(self, arguments=None, name=adapter.PLAN_TOOL):
        value = adapter.invoke(
            name,
            dict(
                call_id="adapter-fixture",
                name=name,
                arguments=self.request if arguments is None else arguments,
            ),
            self.stop,
        )
        return value, json.loads(value["text"])

    def test_caller_owned_plan_round_trips_without_authorizing_updates(self):
        tool, report = self.call()
        self.assertTrue(tool["success"])
        self.assertEqual(report["status"], "scoped_plan")
        expected = adapter.policy.plan_selection(**self.inputs)
        self.assertEqual(report["plan_id"], expected.plan_id)
        self.assertEqual(report["plan_json"].encode(), expected.payload)
        restored_tool, restored = self.call(
            dict(
                contract_version=1,
                purpose="restore_selection",
                plan_id=report["plan_id"],
                plan_json=report["plan_json"],
            ),
            adapter.RESTORE_TOOL,
        )
        self.assertTrue(restored_tool["success"])
        self.assertEqual(restored, dict(report, plan_revalidated=True))
        for key in (
            "state_written",
            "update_allowed",
            "activation_allowed",
            "enforcement_authority",
        ):
            self.assertIs(restored[key], False)

    def test_invalid_selection_is_computed_but_remains_blocked(self):
        self.fixture.profile["include"] = ["feature:unknown"]
        tool, report = self.call()
        self.assertTrue(tool["success"])
        self.assertEqual(report["status"], "blocked")
        self.assertIn("unknown_selection:feature:unknown", report["findings"])

    def test_changed_profile_cannot_restore_under_older_plan_identity(self):
        _, first = self.call()
        self.fixture.profile["revision"] += 1
        _, second = self.call()
        tool, rejected = self.call(
            dict(
                contract_version=1,
                purpose="restore_selection",
                plan_id=first["plan_id"],
                plan_json=second["plan_json"],
            ),
            adapter.RESTORE_TOOL,
        )
        self.assertFalse(tool["success"])
        self.assertEqual(rejected["diagnostic_code"], "stale_plan_evidence")

    def test_rows_and_summed_references_reject_before_pure_graph_work(self):
        for change, diagnostic in (
            (
                lambda: self.fixture.graph.update(
                    units=[self.fixture.graph["units"][0]] * 65
                ),
                "tool_row_limit_or_shape",
            ),
            (
                lambda: self.fixture.profile.update(include=["component:a"] * 1025),
                "tool_reference_limit",
            ),
        ):
            self.setUp()
            change()
            with patch.object(adapter.policy, "plan_selection") as run:
                tool, report = self.call()
            run.assert_not_called()
            self.assertFalse(tool["success"])
            self.assertEqual(report["diagnostic_code"], diagnostic)

    def test_request_bytes_reject_before_pure_graph_work(self):
        self.fixture.profile["id"] = "x" * adapter.MAX_PLAN_REQUEST
        with patch.object(adapter.policy, "plan_selection") as run:
            tool, report = self.call()
        run.assert_not_called()
        self.assertFalse(tool["success"])
        self.assertEqual(report["diagnostic_code"], "tool_input_limit")

    def test_restore_admission_precedes_revalidation_of_large_graph(self):
        _, report = self.call()
        saved = json.loads(report["plan_json"])
        saved["inputs"]["graph"]["units"] *= 33
        with patch.object(adapter.policy, "restore_plan") as run:
            tool, rejected = self.call(
                dict(
                    contract_version=1,
                    purpose="restore_selection",
                    plan_id=report["plan_id"],
                    plan_json=json.dumps(saved),
                ),
                adapter.RESTORE_TOOL,
            )
        run.assert_not_called()
        self.assertFalse(tool["success"])
        self.assertEqual(rejected["diagnostic_code"], "tool_row_limit_or_shape")

    def test_corrupted_plan_invalid_contract_and_malformed_envelope_reject(self):
        tool, report = self.call(dict(self.request, contract_version=2))
        self.assertFalse(tool["success"])
        self.assertEqual(report["diagnostic_code"], "unsupported_policy_contract")
        tool, report = self.call(
            dict(
                contract_version=1,
                purpose="restore_selection",
                plan_id="0" * 64,
                plan_json="{",
            ),
            adapter.RESTORE_TOOL,
        )
        self.assertFalse(tool["success"])
        self.assertEqual(report["diagnostic_code"], "malformed_or_unsupported_request")
        tool = adapter.invoke(adapter.PLAN_TOOL, {}, self.stop)
        self.assertFalse(tool["success"])

    def test_cancellation_checked_at_boundaries_not_claimed_preemptive(self):
        self.stop.set()
        with patch.object(adapter.policy, "plan_selection") as run:
            tool, report = self.call()
        run.assert_not_called()
        self.assertFalse(tool["success"])
        self.assertEqual(report["status"], "cancelled")
        self.stop.clear()
        original = adapter.policy.plan_selection

        def complete_then_stop(**values):
            value = original(**values)
            self.stop.set()
            return value

        with patch.object(
            adapter.policy, "plan_selection", side_effect=complete_then_stop
        ):
            tool, report = self.call()
        self.assertFalse(tool["success"])
        self.assertEqual(report["status"], "cancelled")

    def test_output_plan_limit_returns_no_partial_plan(self):
        with patch.object(adapter, "MAX_PLAN_BYTES", 1):
            tool, report = self.call()
        self.assertFalse(tool["success"])
        self.assertEqual(report["diagnostic_code"], "tool_plan_limit")
        self.assertNotIn("plan_json", report)

    def test_large_valid_plan_returns_explicit_bounded_json_summary(self):
        self.fixture.review["context_note"] = "x" * (adapter.MAX_TEXT_BYTES + 1)
        self.fixture.refresh()
        tool, report = self.call()
        self.assertTrue(tool["success"])
        self.assertTrue(report["plan_json_omitted"])
        self.assertIsNone(report["plan_json"])
        self.assertEqual(
            report["plan_id"], adapter.policy.plan_selection(**self.inputs).plan_id
        )
        self.assertLessEqual(len(tool["text"].encode()), adapter.MAX_TEXT_BYTES)

    def test_escaped_output_remains_complete_json_under_outer_sdk_frame_budget(self):
        self.fixture.review["context_note"] = 'λ\\"' * 8
        self.fixture.refresh()
        tool, report = self.call()
        self.assertTrue(tool["success"])
        wire = (
            json.dumps(
                dict(type="result", id=2**64 - 1, result=tool),
                allow_nan=False,
                separators=(",", ":"),
            ).encode()
            + b"\n"
        )
        self.assertLessEqual(len(tool["text"].encode()), adapter.MAX_TEXT_BYTES)
        self.assertLessEqual(
            len(adapter.policy.canonical(tool)), adapter.MAX_TOOL_BYTES
        )
        self.assertLessEqual(len(wire), 64 * 1024)
        self.assertEqual(json.loads(json.loads(wire)["result"]["text"]), report)
        self.assertFalse(report["plan_json_omitted"])
        self.assertEqual(
            json.loads(report["plan_json"])["inputs"]["review"]["context_note"],
            self.fixture.review["context_note"],
        )

    def test_finding_omission_is_visible_and_never_authorizes_updates(self):
        self.fixture.profile["include"] = [f"feature:unknown-{n}" for n in range(20)]
        tool, report = self.call()
        self.assertTrue(tool["success"])
        self.assertEqual(report["status"], "blocked")
        self.assertEqual(len(report["findings"]), 16)
        self.assertGreater(report["finding_count"], 16)
        self.assertTrue(report["findings_omitted"])
        self.assertFalse(report["activation_allowed"])

    def test_real_review_projection_still_cannot_authorize_update(self):
        fixture = json.loads(
            (
                Path(__file__).parent / "fixtures/selection_scope_actual_impact.json"
            ).read_bytes()
        )
        self.request["review"] = dict(
            fixture["review_template"],
            changed_paths=[
                dict(path=path, unresolved=fixture["finding_sets"][key])
                for path, key in fixture["paths"].items()
            ],
        )
        self.fixture.graph["bindings"]["review_sha256"] = adapter.policy.digest(
            self.request["review"]
        )
        self.fixture.profile["catalog_sha256"] = adapter.policy.digest(
            self.fixture.graph
        )
        tool, report = self.call()
        self.assertTrue(tool["success"])
        self.assertEqual(report["status"], "blocked")
        self.assertIn(
            "impact:historical_index_not_current_composition", report["findings"]
        )
        self.assertFalse(report["activation_allowed"])


if __name__ == "__main__":
    unittest.main()
