"""Small in-memory corruption fixtures; no Git, network, filesystem mutation or Rust."""

import copy
import hashlib
import json
import unittest
from unittest.mock import patch

import validate_checkpoint_lineage as validator


def blob(value):
    data = value if isinstance(value, bytes) else json.dumps(value).encode()
    oid = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
    return oid, data


class Objects:
    def __init__(self, trees, blobs):
        self.trees = trees
        self.blobs = blobs
        self.commits = {}

    def commit_tree(self, identity):
        if identity not in self.commits:
            raise ValueError("commit_unavailable")
        return self.commits[identity]

    def tree(self, identity):
        return self.trees[identity]

    def blob(self, identity):
        return self.blobs[identity]


class EvidenceCorruptionTests(unittest.TestCase):
    def setUp(self):
        self.path = "upstream/lineage.json"
        self.document = {"schema_version": 1, "source": {}, "boundaries": []}
        oid, data = blob(self.document)
        self.entry = {"mode": "100644", "type": "blob", "blob": oid}
        self.objects = Objects(
            {"1" * 40: {}, "2" * 40: {self.path: self.entry}}, {oid: data}
        )
        self.receipt = json.dumps(
            {"sha": "3" * 40, "tree": "2" * 40, "remote_verified": True}
        ).encode()
        self.index = {
            "schema_version": 1,
            "source": {
                "import_tree": "1" * 40,
                "checkpoint_tree": "2" * 40,
                "checkpoint_commit": "3" * 40,
                "checkpoint_repository": "https://example.test/fork",
                "upstream_repository": "https://example.test/upstream",
                "upstream_revision": "4" * 40,
                "publication": {"sha256": validator.digest(self.receipt)},
            },
            "paths": [
                {
                    "path": self.path,
                    "before": None,
                    "after": self.entry,
                    "inventory_owners": ["C27"],
                    "semantic_status": "unresolved",
                    "edge_ids": [],
                }
            ],
            "historical_maps": [
                {"path": self.path, "blob": oid, "sha256": validator.digest(data)}
            ],
            "boundary_edges": [],
        }
        self.schemas = patch.dict(
            validator.SCHEMA_SIGNATURES,
            {self.path: validator.schema_signature(self.document)},
        )
        self.schemas.start()
        self.addCleanup(self.schemas.stop)

    def result(self):
        return validator.validate(self.index, self.objects, self.receipt)

    def test_valid_metadata_preserves_unresolved_semantics(self):
        result = self.result()
        self.assertEqual(result["invalid"], 0)
        self.assertEqual(result["status"], "unresolved")
        self.assertFalse(result["runtime_or_updater_acceptance"])

    def test_wrong_historical_hash_is_invalid(self):
        self.index["historical_maps"][0]["sha256"] = "0" * 64
        self.assertEqual(
            self.result()["counts"]["invalid:historical_map_hash_mismatch"], 1
        )

    def test_native_edge_requires_original_source(self):
        self.index["boundary_edges"] = [
            {
                "id": "missing-native",
                "relation": "native_derivation",
                "origins": [],
                "destinations": [],
                "mapping_refs": [],
            }
        ]
        self.assertEqual(self.result()["counts"]["invalid:native_origin_missing"], 1)

    def test_unknown_schema_is_unresolved_even_with_correct_hash(self):
        document = copy.deepcopy(self.document)
        document["schema_version"] = 2
        oid, data = blob(document)
        self.objects.blobs[oid] = data
        self.entry["blob"] = oid
        self.index["historical_maps"][0].update(blob=oid, sha256=validator.digest(data))
        result = self.result()
        self.assertEqual(result["invalid"], 0)
        self.assertEqual(
            result["counts"]["unresolved:historical_schema_unsupported"], 1
        )

    def test_omitted_changed_path_is_invalid(self):
        self.index["paths"] = []
        self.assertEqual(
            self.result()["counts"]["invalid:changed_path_coverage_mismatch"], 1
        )

    def test_original_anchor_must_exist_in_its_bound_blob(self):
        oid, data = blob(b"pub trait ActualNative {}")
        self.objects.blobs[oid] = data
        findings = []
        validator.check_anchor(
            {
                "path": "native.rs",
                "revision": "4" * 40,
                "blob": oid,
                "symbols": ["pub trait InventedNative"],
            },
            self.objects,
            lambda *finding: findings.append(finding),
            "fixture",
        )
        self.assertEqual(findings, [("invalid", "anchor_not_found", "fixture")])

    def test_nested_malformed_documents_return_json_outcomes(self):
        for field, value in [
            ("paths", [None]),
            ("source", []),
            ("boundary_edges", [None]),
        ]:
            with self.subTest(field=field):
                candidate = copy.deepcopy(self.index)
                candidate[field] = value
                self.assertEqual(
                    validator.validate(candidate, self.objects, self.receipt)["status"],
                    "invalid",
                )
        self.assertEqual(
            validator.validate(self.index, self.objects, b"[]")["status"], "invalid"
        )

    def test_unrelated_edge_does_not_mark_a_path_reviewed(self):
        self.index["paths"][0].update(
            semantic_status="reviewed_edge", edge_ids=["unrelated"]
        )
        self.index["boundary_edges"] = [
            {
                "id": "unrelated",
                "relation": "original_custom",
                "origins": [],
                "destinations": [],
                "mapping_refs": [],
            }
        ]
        self.assertEqual(
            self.result()["counts"]["invalid:reviewed_path_without_exact_edge"], 1
        )

    def test_exact_edge_binds_only_its_pinned_destination(self):
        self.index["paths"][0].update(
            semantic_status="reviewed_edge", edge_ids=["exact"]
        )
        destination = {
            "repository": "https://example.test/fork",
            "revision": "3" * 40,
            "path": self.path,
            "blob": self.entry["blob"],
            "symbols": ['"boundaries"'],
        }
        self.index["boundary_edges"] = [
            {
                "id": "exact",
                "relation": "original_custom",
                "origins": [],
                "destinations": [destination],
                "mapping_refs": [{"path": self.path, "pointer": "/source"}],
            }
        ]
        self.assertEqual(self.result()["invalid"], 0)
        destination["revision"] = "5" * 40
        self.assertEqual(
            self.result()["counts"]["invalid:reviewed_path_without_exact_edge"], 1
        )

    def test_current_custom_dependency_cannot_supply_native_derivation(self):
        self.index["boundary_edges"] = [
            {
                "id": "not-native",
                "relation": "native_derivation",
                "origins": [
                    {
                        "repository": "https://example.test/fork",
                        "revision": "3" * 40,
                        "path": self.path,
                        "blob": self.entry["blob"],
                        "symbols": ['"boundaries"'],
                    }
                ],
                "destinations": [],
                "mapping_refs": [],
            }
        ]
        self.assertEqual(
            self.result()["counts"]["unresolved:native_origin_unverified"], 1
        )

    def test_local_checkpoint_commit_needs_no_publication_receipt(self):
        self.objects.commits["3" * 40] = "2" * 40
        result = validator.validate(self.index, self.objects)
        self.assertEqual(result["invalid"], 0)
        self.assertNotIn("unresolved:publication_receipt_unavailable", result["counts"])

    def test_local_checkpoint_mismatch_cannot_be_hidden_by_receipt(self):
        self.objects.commits["3" * 40] = "5" * 40
        self.assertEqual(
            self.result()["counts"]["invalid:checkpoint_revision_tree_mismatch"], 1
        )

    def test_absent_publication_receipt_is_unresolved(self):
        result = validator.validate(self.index, self.objects)
        self.assertEqual(
            result["counts"]["unresolved:publication_receipt_unavailable"], 1
        )


if __name__ == "__main__":
    unittest.main()
