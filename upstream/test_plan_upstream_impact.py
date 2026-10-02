"""In-memory impact-planner contract fixtures; no Git/network/runtime.

Existing lineage validation already has its own corruption suite. These tests
stub its result to isolate candidate comparison and unresolved-impact handling.
"""

import copy
import unittest
from unittest.mock import patch

import plan_upstream_impact as planner


BASE, CANDIDATE, CHECKPOINT, OTHER = [c * 40 for c in "abcd"]
OLD_TREE, NEW_TREE, FORK_TREE = [c * 40 for c in "123"]


def entry(oid, mode="100644", kind="blob"):
    return {"mode": mode, "type": kind, "blob": oid * 40}


class Objects:
    def __init__(self):
        self.commits = {BASE: OLD_TREE, CANDIDATE: NEW_TREE, CHECKPOINT: FORK_TREE}
        self.trees = {
            OLD_TREE: {"codex-rs/thread-store/src/store.rs": entry("4")},
            NEW_TREE: {"codex-rs/thread-store/src/store.rs": entry("5")},
        }

    def commit_tree(self, revision):
        if revision not in self.commits:
            raise ValueError("not_retained_locally")
        return self.commits[revision]

    def tree(self, tree):
        return self.trees[tree]


class ImpactPlanningTests(unittest.TestCase):
    def setUp(self):
        self.objects = Objects()
        self.path = "codex-rs/thread-store/src/store.rs"
        self.index = {
            "schema_version": 1,
            "source": {
                "upstream_repository": "https://github.com/openai/codex",
                "upstream_revision": BASE,
                "import_tree": OLD_TREE,
                "checkpoint_commit": CHECKPOINT,
                "checkpoint_tree": FORK_TREE,
            },
            "paths": [{"path": self.path, "inventory_owners": ["C02"]}],
            "boundary_edges": [
                {
                    "id": "thread-store",
                    "component": "C02",
                    "relation": "native_adaptation",
                    "origins": [
                        {
                            "repository": "https://github.com/openai/codex",
                            "revision": BASE,
                            "tree": OLD_TREE,
                            "path": self.path,
                        }
                    ],
                    "destinations": [
                        {
                            "path": "codex-rs/thread-store/src/store.rs",
                            "revision": CHECKPOINT,
                        }
                    ],
                    "current_semantic_closure": False,
                }
            ],
        }
        self.validation = {"status": "unresolved", "invalid": 0, "unresolved": 7}
        self.mock = patch.object(
            planner.lineage, "validate", return_value=self.validation
        )
        self.mock.start()
        self.addCleanup(self.mock.stop)

    def result(self, candidate=CANDIDATE, composition=CHECKPOINT):
        return planner.plan(self.index, self.objects, candidate, composition)

    def test_exact_match_still_requires_semantic_security_and_runtime_gates(self):
        result = self.result()
        row = result["changed_paths"][0]
        self.assertEqual(row["matched_source_edges"][0]["id"], "thread-store")
        self.assertIn("matched_boundary_semantic_closure_incomplete", row["unresolved"])
        self.assertIn(
            "existing_lineage_findings_remain_unresolved", result["unresolved"]
        )
        self.assertFalse(result["automatic_update_eligible"])
        self.assertFalse(result["runtime_or_updater_acceptance"])
        self.assertEqual(result["inputs"]["base_revision"], BASE)
        self.assertEqual(result["inputs"]["candidate_revision"], CANDIDATE)

    def test_truthy_non_boolean_closure_is_not_true(self):
        self.index["boundary_edges"][0]["current_semantic_closure"] = "false"
        row = self.result()["changed_paths"][0]
        self.assertFalse(row["matched_source_edges"][0]["semantic_closure_recorded"])
        self.assertIn("matched_boundary_semantic_closure_incomplete", row["unresolved"])

    def test_destinations_are_referenced_not_repeated_in_each_changed_path(self):
        self.index["boundary_edges"][0]["destinations"] = [{"path": "x" * 100000}]
        row = self.result()["changed_paths"][0]
        edge = row["matched_source_edges"][0]
        self.assertNotIn("recorded_destinations", edge)
        self.assertEqual(edge["recorded_destination_count"], 1)
        self.assertEqual(edge["index_pointer"], "/boundary_edges/0")

    def test_detail_budget_stops_before_retaining_excess_rows(self):
        with patch.object(planner, "MAX_REPORT_DETAILS", 1):
            result = self.result()
        self.assertEqual(result["summary"]["changed_paths_reported"], 0)
        self.assertFalse(result["summary"]["path_inventory_complete"])
        self.assertIn("report_detail_byte_budget_reached", result["unresolved"])

    def test_large_non_ascii_scalar_cannot_bypass_final_output_cap(self):
        result = self.result()
        result["inputs"]["upstream_repository_claim"] = "界" * 10000
        result["index_sha256"] = "e" * 64
        with patch.object(planner.lineage, "MAX_INPUT", 4096):
            output = planner.render_report(result)
        self.assertLess(len(output.encode()), 4096)
        self.assertIn("report_detail_output_limit_reached", output)
        self.assertNotIn("upstream_repository_claim", output)
        self.assertIn("e" * 64, output)

    def test_wrong_origin_revision_cannot_match_current_base(self):
        self.index["boundary_edges"][0]["origins"][0]["revision"] = OTHER
        row = self.result()["changed_paths"][0]
        self.assertEqual(row["matched_source_edges"], [])
        self.assertIn("original_source_boundary_mapping_missing", row["unresolved"])

    def test_multiple_owners_not_converted_to_success(self):
        self.index["paths"][0]["inventory_owners"].append("C08")
        row = self.result()["changed_paths"][0]
        self.assertIn("multiple_recorded_owners_not_resolved", row["unresolved"])

    def test_unknown_native_change_is_retained(self):
        self.objects.trees[NEW_TREE]["codex-rs/new-domain/src/lib.rs"] = entry("6")
        rows = self.result()["changed_paths"]
        unknown = next(row for row in rows if "new-domain" in row["path"])
        self.assertIn("ownership_unknown", unknown["unresolved"])
        self.assertIn("original_source_boundary_mapping_missing", unknown["unresolved"])

    def test_deleted_and_added_paths_are_not_guessed_as_a_rename(self):
        self.objects.trees[NEW_TREE] = {"codex-rs/renamed.rs": entry("4")}
        rows = self.result()["changed_paths"]
        self.assertEqual({row["change"] for row in rows}, {"added", "deleted"})
        self.assertTrue(
            any(
                "deleted_origin_requires_customization_review" in row["unresolved"]
                for row in rows
            )
        )

    def test_stale_index_does_not_label_new_composition_current(self):
        result = self.result(composition=OTHER)
        self.assertIn(
            "lineage_does_not_cover_requested_composition", result["unresolved"]
        )
        self.assertIn("requested_composition_commit_unavailable", result["unresolved"])
        self.assertIn(
            "historical_index_not_current_composition",
            result["changed_paths"][0]["unresolved"],
        )

    def test_same_revision_is_not_an_upstream_integration_pass(self):
        result = self.result(candidate=BASE)
        self.assertEqual(result["summary"]["changed_paths_total"], 0)
        self.assertIn("no_upstream_revision_advance", result["unresolved"])
        self.assertFalse(result["automatic_update_eligible"])

    def test_missing_candidate_objects_remain_unresolved_without_fetch(self):
        result = self.result(candidate=OTHER)
        self.assertEqual(result["status"], "unresolved")
        self.assertEqual(result["inputs"]["candidate_revision"], OTHER)
        self.assertIn(
            "input_or_local_object_unavailable_or_unsupported", result["unresolved"]
        )

    def test_symbolic_revision_is_invalid(self):
        self.assertEqual(self.result(candidate="main")["status"], "invalid")

    def test_invalid_lineage_cannot_produce_impact_success(self):
        self.validation.update(status="invalid", invalid=1)
        result = self.result()
        self.assertEqual(result["status"], "invalid")
        self.assertEqual(result["changed_paths"], [])

    def test_report_limit_is_explicit_partial_coverage(self):
        self.objects.trees[NEW_TREE]["new.rs"] = entry("6")
        with patch.object(planner, "MAX_CHANGED_DETAILS", 1):
            result = self.result()
        self.assertEqual(result["summary"]["changed_paths_total"], 2)
        self.assertEqual(result["summary"]["changed_paths_reported"], 1)
        self.assertFalse(result["summary"]["path_inventory_complete"])
        self.assertIn("changed_path_detail_limit_reached", result["unresolved"])

    def test_license_dependency_and_state_hints_never_clear_other_risks(self):
        self.objects.trees[NEW_TREE].update(
            {"LICENSE": entry("6"), "codex-rs/Cargo.toml": entry("7")}
        )
        result = self.result()
        hints = {
            row["path"]: row["review_routing_hints_not_risk_clearance"]
            for row in result["changed_paths"]
        }
        self.assertIn("license_notice", hints["LICENSE"])
        self.assertIn("dependency_build_toolchain", hints["codex-rs/Cargo.toml"])
        self.assertIn("persistent_state", hints[self.path])
        self.assertIn(
            "permissions_credentials_sandbox_security_review", result["required_gates"]
        )

    def test_symlink_or_submodule_changes_require_review(self):
        self.objects.trees[NEW_TREE][self.path] = entry("6", mode="120000")
        row = self.result()["changed_paths"][0]
        self.assertEqual(row["change"], "mode_or_type_changed")
        self.assertIn("non_regular_source_entry_requires_review", row["unresolved"])


if __name__ == "__main__":
    unittest.main()
