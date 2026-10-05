"""Pure policy fixtures; no filesystem attestation, installed runtime or UI proof."""

import copy
from dataclasses import FrozenInstanceError
import json
from pathlib import Path
import sys
import unittest

PROJECT = Path(__file__).resolve().parents[1] / "examples" / "upstream-maintenance"
sys.path.insert(0, str(PROJECT))
import selection_plan as policy

sys.path.pop(0)


def entry(character, mode="100644"):
    return dict(blob=character * 40, mode=mode)


def unit(name, **changes):
    return dict(
        dict(id=name, members=[], requires=[], conflicts=[], unresolved=[]), **changes
    )


class SelectionPlanTests(unittest.TestCase):
    def setUp(self):
        self.before = {name + ".rs": entry(name) for name in ("a", "b")}
        self.review = dict(
            schema="codex-upstream-offline-impact-v1",
            status="unresolved",
            unresolved=[],
            changed_paths=[dict(path=name, unresolved=[]) for name in ("a.rs", "b.rs")],
            summary=dict(
                path_inventory_complete=True,
                changed_paths_total=2,
                changed_paths_reported=2,
            ),
        )
        self.graph = dict(
            contract_version=1,
            bindings=dict(
                base_revision="a" * 40,
                upstream_revision="b" * 40,
                custom_revision="c" * 40,
                composition_sha256="d" * 64,
                lineage_sha256="e" * 64,
                review_sha256=policy.digest(self.review),
                before_inventory_sha256=policy.digest(self.before),
                capsule_sha256="f" * 64,
            ),
            unresolved=[],
            units=[unit("component:a"), unit("component:b")],
            changes=[],
        )
        for name in ("a", "b"):
            self.graph["changes"].append(
                dict(
                    id=name,
                    unit="component:" + name,
                    plugins=["plugin." + name],
                    path=name + ".rs",
                    category="implementation",
                    before=entry(name),
                    origin=dict(
                        revision="a" * 40,
                        path=name + ".rs",
                        blob=name * 40,
                        symbol="fn original",
                    ),
                    unresolved=[],
                )
            )
        self.profile = dict(
            contract_version=1,
            id="remembered-choice",
            revision=1,
            catalog_sha256=policy.digest(self.graph),
            include=["component:a"],
            exclude=[],
        )
        self.scopes = dict(
            contract_version=1, id="reviewed-scopes", revision=1, rules=[]
        )
        self.refresh()

    def refresh(self):
        self.graph["bindings"]["review_sha256"] = policy.digest(self.review)
        self.profile["catalog_sha256"] = policy.digest(self.graph)
        self.scopes["rules"] = [
            dict(
                plugin=plugin,
                change=row["id"],
                path=row["path"],
                category=row["category"],
                before=copy.deepcopy(row["before"]),
                origin_sha256=policy.digest(row["origin"]),
                operations=["modify"],
                granularity="file",
            )
            for row in self.graph["changes"]
            for plugin in row["plugins"]
        ]

    def plan(self):
        return policy.plan_selection(self.graph, self.profile, self.scopes, self.review)

    def test_partial_plan_never_authorizes_update(self):
        plan = self.plan()
        result = plan.document()["result"]
        self.assertEqual(result["status"], "scoped_plan")
        self.assertEqual(plan.document()["result"]["selected_changes"], ["a"])
        for key in (
            "update_allowed",
            "activation_allowed",
            "complete_candidate",
            "enforcement_authority",
        ):
            self.assertIs(result[key], False)
        self.assertEqual(set(result["required_later_gates"].values()), {"pending"})

    def test_canonical_key_order_and_input_mutation_do_not_change_frozen_plan(self):
        plan = self.plan()
        reordered = dict(reversed(list(self.graph.items())))
        self.assertEqual(
            plan,
            policy.plan_selection(reordered, self.profile, self.scopes, self.review),
        )
        frozen = plan.payload
        self.graph["changes"][0]["origin"]["symbol"] = "mutated"
        self.assertEqual(plan.payload, frozen)
        self.assertEqual(policy.restore_plan(plan.payload, plan.plan_id), plan)
        with self.assertRaises(FrozenInstanceError):
            plan.payload = b"changed"

    def test_missing_dependency_not_implicitly_selected(self):
        self.graph["units"][0]["requires"] = ["component:b"]
        self.profile["exclude"] = ["component:b"]
        self.refresh()
        result = self.plan().document()["result"]
        self.assertIn("missing_dependency:component:b", result["findings"])
        self.assertNotIn("component:b", result["selected"])

    def test_persistent_exclusions_and_membership_expand_without_scope_broadening(self):
        self.graph["units"] = [
            unit("component:a", members=["feature:a", "feature:b"]),
            unit("feature:a"),
            unit("feature:b"),
        ]
        for row in self.graph["changes"]:
            row["unit"] = "feature:" + row["id"]
        self.profile["exclude"] = ["feature:b"]
        self.refresh()
        self.assertEqual(self.plan().document()["result"]["selected_changes"], ["a"])
        self.assertEqual(self.plan().document()["result"]["status"], "scoped_plan")

    def test_atomic_groups_and_group_constraints_cannot_be_bypassed(self):
        self.graph["units"].append(
            unit(
                "coherent_group:pair",
                members=["component:a", "component:b"],
                conflicts=["component:a"],
            )
        )
        self.refresh()
        self.assertIn(
            "partial_coherent_group:coherent_group:pair",
            self.plan().document()["result"]["findings"],
        )
        self.profile["include"] = ["component:a", "component:b"]
        self.assertIn(
            "conflicting_selection:coherent_group:pair",
            self.plan().document()["result"]["findings"],
        )

    def test_valid_group_selects_all_members_and_requires_each_scope(self):
        self.graph["units"].append(
            unit("coherent_group:pair", members=["component:a", "component:b"])
        )
        self.profile["include"] = ["coherent_group:pair"]
        self.refresh()
        self.assertEqual(self.plan().document()["result"]["status"], "scoped_plan")
        self.scopes["rules"].pop()
        self.assertIn(
            "scope_binding_missing:b:plugin.b",
            self.plan().document()["result"]["findings"],
        )

    def test_feature_cannot_bypass_containing_component_constraints(self):
        self.graph["units"][0].update(
            members=["feature:a"],
            requires=["component:b"],
            unresolved=["ownership_gap"],
        )
        self.graph["units"].append(unit("feature:a"))
        self.graph["changes"][0]["unit"] = "feature:a"
        self.profile["include"] = ["feature:a"]
        self.refresh()
        result = self.plan().document()["result"]
        self.assertIn("missing_dependency:component:b", result["findings"])
        self.assertIn("unit:ownership_gap", result["findings"])
        self.assertNotIn("component:a", result["selected"])
        self.profile["include"].append("component:b")
        self.graph["units"][1]["conflicts"] = ["component:a"]
        self.refresh()
        self.assertIn(
            "conflicting_selection:component:b",
            self.plan().document()["result"]["findings"],
        )

    def test_selected_group_with_all_members_excluded_stays_blocked(self):
        self.graph["units"].append(unit("coherent_group:one", members=["component:a"]))
        self.profile.update(
            include=["coherent_group:one", "component:b"], exclude=["component:a"]
        )
        self.refresh()
        self.assertIn(
            "partial_coherent_group:coherent_group:one",
            self.plan().document()["result"]["findings"],
        )

    def test_unknown_overlap_and_duplicate_choices(self):
        self.profile["include"] += ["feature:missing"]
        self.assertIn(
            "unknown_selection:feature:missing",
            self.plan().document()["result"]["findings"],
        )
        self.profile["exclude"] = ["component:a"]
        self.assertIn("selection_overlap", self.plan().document()["result"]["findings"])
        self.profile["include"].append("component:a")
        with self.assertRaisesRegex(policy.PolicyError, "duplicate_identity"):
            self.plan()

    def test_cycles_and_unknown_graph_edges_are_invalid(self):
        self.graph["units"][0]["requires"] = ["component:b"]
        self.graph["units"][1]["requires"] = ["component:a"]
        with self.assertRaisesRegex(policy.PolicyError, "cyclic_catalog"):
            self.plan()
        self.graph["units"][1]["requires"] = ["component:missing"]
        with self.assertRaisesRegex(policy.PolicyError, "unknown_catalog_reference"):
            self.plan()

    def test_changed_catalog_profile_or_policy_invalidates_earlier_evidence(self):
        old = self.plan()
        self.profile["revision"] += 1
        new = self.plan()
        self.assertNotEqual(old.plan_id, new.plan_id)
        with self.assertRaisesRegex(policy.PolicyError, "stale_plan_evidence"):
            policy.restore_plan(new.payload, old.plan_id)
        self.graph["bindings"]["capsule_sha256"] = "1" * 64
        self.assertIn("stale_profile", self.plan().document()["result"]["findings"])
        self.refresh()
        current = self.plan()
        self.scopes["revision"] += 1
        self.assertNotEqual(current.plan_id, self.plan().plan_id)

    def test_every_owner_and_exact_original_symbol_binding_required(self):
        self.graph["changes"][0]["plugins"].append("plugin.shared")
        self.profile["catalog_sha256"] = policy.digest(self.graph)
        self.assertIn(
            "scope_binding_missing:a:plugin.shared",
            self.plan().document()["result"]["findings"],
        )
        self.refresh()
        self.scopes["rules"][0]["origin_sha256"] = "0" * 64
        self.assertIn(
            "scope_binding_missing:a:plugin.a",
            self.plan().document()["result"]["findings"],
        )

    def test_each_semantic_category_requires_its_own_explicit_scope(self):
        for category in ("config", "contracts", "dependencies", "schemas"):
            with self.subTest(category=category):
                self.graph["changes"][0]["category"] = category
                self.profile["catalog_sha256"] = policy.digest(self.graph)
                self.scopes["rules"][0]["category"] = "implementation"
                self.assertEqual(self.plan().document()["result"]["status"], "blocked")
                self.scopes["rules"][0]["category"] = category
                self.assertEqual(
                    self.plan().document()["result"]["status"], "scoped_plan"
                )

    def test_paths_symlinks_and_symbol_only_scope_fail_closed(self):
        original = self.graph["changes"][0]["path"]
        for name in ("../escape", "/absolute", "a//b", "a/./b", "C:/drive", "a\\b"):
            self.graph["changes"][0]["path"] = name
            with self.subTest(path=name), self.assertRaises(policy.PolicyError):
                self.plan()
        self.graph["changes"][0]["path"] = original
        self.graph["changes"][0]["before"]["mode"] = "120000"
        with self.assertRaises(policy.PolicyError):
            self.plan()
        self.graph["changes"][0]["before"]["mode"] = "100644"
        self.scopes["rules"][0]["granularity"] = "symbol"
        with self.assertRaisesRegex(
            policy.PolicyError, "unsupported_scope_granularity"
        ):
            self.plan()

    def test_unsupported_versions_and_boolean_revisions_are_rejected(self):
        self.profile["contract_version"] = True
        with self.assertRaises(policy.PolicyError):
            self.plan()
        self.profile["contract_version"] = 1
        self.profile["revision"] = True
        with self.assertRaises(policy.PolicyError):
            self.plan()

    def test_oversized_input_rejected_before_graph_walk(self):
        self.graph["units"] = None
        self.profile["id"] = "x" * (policy.MAX_BYTES + 1)
        with self.assertRaisesRegex(policy.PolicyError, "input_byte_limit"):
            self.plan()

    def test_persisted_plan_revalidation_rejects_self_modified_policy(self):
        doc = self.plan().document()
        doc["result"]["activation_allowed"] = True
        plan = policy.SelectionPlan(policy.canonical(doc))
        with self.assertRaisesRegex(policy.PolicyError, "altered_plan"):
            policy.restore_plan(plan.payload, plan.plan_id)

    def test_actual_impact_projection_keeps_historical_mapping_blocked(self):
        fixture = json.loads(
            (
                Path(__file__).parent / "fixtures/selection_scope_actual_impact.json"
            ).read_bytes()
        )
        review = dict(
            fixture["review_template"],
            changed_paths=[
                dict(path=path, unresolved=fixture["finding_sets"][key])
                for path, key in fixture["paths"].items()
            ],
        )
        self.assertEqual(policy.digest(review), fixture["required_projection_sha256"])
        findings, paths = policy.impact_findings(review)
        self.assertEqual(len(paths), 13)
        self.assertIn("impact:historical_index_not_current_composition", findings)
        self.assertIn("impact:original_source_boundary_mapping_missing", findings)
        self.assertIn("impact:ownership_unknown", findings)
        self.review = review
        self.refresh()
        result = self.plan().document()["result"]
        self.assertEqual(result["status"], "blocked")
        self.assertIn("mapped_change_inventory_incomplete", result["findings"])
        self.assertFalse(result["activation_allowed"])

    def test_incomplete_or_rebound_impact_report_cannot_clear_plan(self):
        self.review["summary"]["path_inventory_complete"] = False
        result = self.plan().document()["result"]
        self.assertIn("impact:incomplete_inventory", result["findings"])
        self.assertIn("review_binding_mismatch", result["findings"])


if __name__ == "__main__":
    unittest.main()
