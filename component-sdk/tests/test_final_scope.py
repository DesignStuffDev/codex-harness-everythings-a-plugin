"""Declared inventory/scope fixtures; not a trusted filesystem collection test."""

import copy
import json
from pathlib import Path
import sys
import unittest

import test_selection_plan as fixtures

PROJECT = Path(__file__).resolve().parents[1] / "examples" / "upstream-maintenance"
sys.path.insert(0, str(PROJECT))
import final_scope as audit

sys.path.pop(0)


def file(character, mode="100644"):
    return dict(
        kind="file", mode=mode, blob=character * 40, sha256=character * 64, bytes=10
    )


class FinalScopeTests(unittest.TestCase):
    def setUp(self):
        self.fixture = fixtures.SelectionPlanTests()
        self.fixture.setUp()
        self.before = dict(
            contract_version=1,
            namespace="source",
            completeness="complete",
            entries={name + ".rs": file(name) for name in ("a", "b")},
        )
        self.after = copy.deepcopy(self.before)
        self.after["entries"]["a.rs"] = file("c")
        self.bind()

    def bind(self):
        self.fixture.graph["bindings"]["before_inventory_sha256"] = audit.policy.digest(
            self.before
        )
        self.fixture.refresh()

    def run_audit(self, plan=None, expected=None):
        plan = self.fixture.plan() if plan is None else plan
        return audit.audit_scope(
            plan,
            self.before,
            self.after,
            plan.plan_id if expected is None else expected,
        )

    def test_complete_comparison_receipt_binds_exact_inputs_and_never_authorizes(self):
        originals = copy.deepcopy((self.before, self.after))
        report = self.run_audit()
        self.assertEqual(report["status"], "supplied_inventory_conforms")
        self.assertEqual(
            report["differences"],
            [
                dict(
                    path="a.rs",
                    before=file("a"),
                    after=file("c"),
                    operations=["modify"],
                )
            ],
        )
        self.assertEqual(report["after_sha256"], audit.policy.digest(self.after))
        receipt = report.pop("receipt_sha256")
        self.assertEqual(receipt, audit.policy.digest(report))
        self.assertFalse(
            any(
                report[key]
                for key in (
                    "update_allowed",
                    "activation_allowed",
                    "enforcement_authority",
                    "complete_candidate",
                )
            )
        )
        self.assertEqual((self.before, self.after), originals)

    def test_whole_baseline_binding_detects_unchanged_unmapped_input_added_later(self):
        plan = self.fixture.plan()
        for inventory in (self.before, self.after):
            inventory["entries"]["unmapped.txt"] = file("d")
        self.assertIn(
            "before_inventory_binding_mismatch", self.run_audit(plan)["findings"]
        )

    def test_changed_output_invalidates_receipt(self):
        first = self.run_audit()
        self.after["entries"]["a.rs"] = file("d")
        second = self.run_audit()
        self.assertEqual(second["status"], "supplied_inventory_conforms")
        self.assertNotEqual(first["after_sha256"], second["after_sha256"])
        self.assertNotEqual(first["receipt_sha256"], second["receipt_sha256"])

    def test_unselected_and_generated_changes_cannot_hide_behind_selected_scope(self):
        self.after["entries"].update({"b.rs": file("d"), "generated.lock": file("e")})
        findings = self.run_audit()["findings"]
        self.assertIn("unselected_or_unmapped_diff:b.rs", findings)
        self.assertIn("unselected_or_unmapped_diff:generated.lock", findings)

    def test_missing_selected_change_and_incorrect_declared_baseline_block(self):
        self.after = copy.deepcopy(self.before)
        self.assertIn("selected_change_not_observed:a", self.run_audit()["findings"])
        self.fixture.graph["changes"][1]["before"] = fixtures.entry("d")
        self.fixture.refresh()
        self.assertIn("before_object_mismatch:b.rs", self.run_audit()["findings"])

    def test_every_owner_must_allow_content_and_mode_operations(self):
        self.fixture.graph["changes"][0]["plugins"].append("plugin.shared")
        self.bind()
        self.after["entries"]["a.rs"]["mode"] = "100755"
        self.fixture.scopes["rules"][0]["operations"] = ["modify", "mode"]
        self.assertIn(
            "operation_denied:a.rs:plugin.shared", self.run_audit()["findings"]
        )
        self.fixture.scopes["rules"][1]["operations"] = ["modify", "mode"]
        self.assertEqual(self.run_audit()["status"], "supplied_inventory_conforms")

    def test_mode_only_change_does_not_gain_permission_from_content_grant(self):
        self.after = copy.deepcopy(self.before)
        self.after["entries"]["a.rs"]["mode"] = "100755"
        self.assertIn("operation_denied:a.rs:plugin.a", self.run_audit()["findings"])
        self.fixture.scopes["rules"][0]["operations"] = ["mode"]
        report = self.run_audit()
        self.assertEqual(report["status"], "supplied_inventory_conforms")
        self.assertEqual(report["differences"][0]["operations"], ["mode"])

    def test_wrong_category_binding_and_ambiguous_classification_remain_blocking(self):
        self.fixture.scopes["rules"][0]["category"] = "schemas"
        self.assertIn("scope_binding_missing:a:plugin.a", self.run_audit()["findings"])
        self.fixture.graph["changes"][0]["unresolved"] = ["classification_ambiguous"]
        self.bind()
        self.assertIn("mapping:classification_ambiguous", self.run_audit()["findings"])

    def test_delete_and_add_need_their_exact_operation_grants(self):
        del self.after["entries"]["a.rs"]
        self.assertIn("operation_denied:a.rs:plugin.a", self.run_audit()["findings"])
        self.fixture.scopes["rules"][0]["operations"] = ["delete"]
        self.assertEqual(self.run_audit()["status"], "supplied_inventory_conforms")
        del self.before["entries"]["a.rs"]
        self.fixture.graph["changes"][0]["before"] = None
        self.bind()
        self.after["entries"]["a.rs"] = file("c")
        self.assertIn("operation_denied:a.rs:plugin.a", self.run_audit()["findings"])
        self.fixture.scopes["rules"][0]["operations"] = ["add"]
        self.assertEqual(self.run_audit()["status"], "supplied_inventory_conforms")

    def test_rename_requires_both_selected_path_scopes(self):
        self.after["entries"]["new.rs"] = self.after["entries"].pop("a.rs")
        self.fixture.scopes["rules"][0]["operations"] = ["delete"]
        self.assertIn(
            "unselected_or_unmapped_diff:new.rs", self.run_audit()["findings"]
        )
        row = copy.deepcopy(self.fixture.graph["changes"][0])
        row.update(id="renamed", path="new.rs", before=None)
        row["origin"].update(path="new.rs", blob=None, symbol=None)
        self.fixture.graph["changes"].append(row)
        self.fixture.review["changed_paths"].append(dict(path="new.rs", unresolved=[]))
        self.fixture.review["summary"].update(
            changed_paths_total=3, changed_paths_reported=3
        )
        self.bind()
        self.fixture.scopes["rules"][0]["operations"] = ["delete"]
        self.fixture.scopes["rules"][2]["operations"] = ["add"]
        self.assertEqual(self.run_audit()["status"], "supplied_inventory_conforms")

    def test_new_executable_file_requires_add_and_mode_permissions(self):
        del self.before["entries"]["a.rs"]
        self.fixture.graph["changes"][0]["before"] = None
        self.after["entries"]["a.rs"] = file("c", "100755")
        self.bind()
        self.fixture.scopes["rules"][0]["operations"] = ["add"]
        self.assertIn("operation_denied:a.rs:plugin.a", self.run_audit()["findings"])
        self.fixture.scopes["rules"][0]["operations"] = ["add", "mode"]
        self.assertEqual(self.run_audit()["status"], "supplied_inventory_conforms")

    def test_type_alias_prefix_and_normalization_ambiguity_reject(self):
        for change in (
            {"kind": "symlink"},
            {"mode": "120000"},
            {"bytes": True},
            {"bytes": -1},
            {"bytes": 1.5},
            {"sha256": "x" * 64},
        ):
            bad = copy.deepcopy(self.before)
            bad["entries"]["a.rs"].update(change)
            with (
                self.subTest(change=change),
                self.assertRaises(audit.policy.PolicyError),
            ):
                audit.inventory(bad)
        for name in (
            "A.rs",
            "a.rs/child",
            "../outside",
            "/absolute",
            "a//b",
            "e\u0301.txt",
        ):
            self.after = copy.deepcopy(self.before)
            self.after["entries"][name] = file("d")
            with self.subTest(path=name), self.assertRaises(audit.policy.PolicyError):
                self.run_audit()

    def test_contradictory_current_hashes_and_sizes_reject_across_inventories(self):
        for change in ({"sha256": "c" * 64}, {"bytes": 11}, {"blob": "c" * 40}):
            self.after = copy.deepcopy(self.before)
            self.after["entries"]["a.rs"].update(change)
            with (
                self.subTest(change=change),
                self.assertRaisesRegex(audit.policy.PolicyError, "contradictory_"),
            ):
                self.run_audit()

    def test_python_integer_path_is_rejected_before_json_can_coerce_it(self):
        self.before["entries"] = {1: file("d")}
        with self.assertRaises(audit.policy.PolicyError):
            audit.inventory(self.before)

    def test_case_only_rename_and_file_directory_transition_fail_closed(self):
        for destination in ("A.rs", "a.rs/child"):
            self.after = copy.deepcopy(self.before)
            self.after["entries"][destination] = self.after["entries"].pop("a.rs")
            with (
                self.subTest(destination=destination),
                self.assertRaises(audit.policy.PolicyError),
            ):
                self.run_audit()

    def test_unknown_incomplete_namespace_and_stale_plan_reject(self):
        for key, value in (
            ("contract_version", 2),
            ("completeness", "partial"),
            ("namespace", "another-root"),
        ):
            self.after[key] = value
            with self.subTest(field=key), self.assertRaises(audit.policy.PolicyError):
                self.run_audit()
            self.after[key] = self.before[key]
        with self.assertRaisesRegex(audit.policy.PolicyError, "stale_plan_evidence"):
            self.run_audit(expected="0" * 64)
        saved = self.fixture.plan().document()
        saved["result"]["activation_allowed"] = True
        plan = audit.policy.SelectionPlan(audit.policy.canonical(saved))
        with self.assertRaisesRegex(audit.policy.PolicyError, "altered_plan"):
            self.run_audit(plan)

    def test_ambiguous_owners_categories_or_duplicate_paths_do_not_receive_receipts(
        self,
    ):
        for change in (
            {"plugins": []},
            {"plugins": ["plugin.a", "plugin.a"]},
            {"category": ["implementation", "schemas"]},
        ):
            self.setUp()
            self.fixture.graph["changes"][0].update(change)
            self.fixture.refresh()
            with (
                self.subTest(change=change),
                self.assertRaises(audit.policy.PolicyError),
            ):
                self.run_audit()
        self.setUp()
        self.fixture.graph["changes"][1]["path"] = "a.rs"
        self.fixture.refresh()
        with self.assertRaisesRegex(audit.policy.PolicyError, "ambiguous_change_path"):
            self.run_audit()

    def test_actual_impact_gaps_survive_otherwise_conforming_inventory(self):
        fixture = json.loads(
            (
                Path(__file__).parent / "fixtures/selection_scope_actual_impact.json"
            ).read_bytes()
        )
        self.fixture.review = dict(
            fixture["review_template"],
            changed_paths=[
                dict(path=name, unresolved=fixture["finding_sets"][key])
                for name, key in fixture["paths"].items()
            ],
        )
        self.bind()
        report = self.run_audit()
        self.assertEqual(report["status"], "blocked")
        self.assertIn(
            "impact:historical_index_not_current_composition", report["findings"]
        )
        self.assertFalse(report["enforcement_authority"])
