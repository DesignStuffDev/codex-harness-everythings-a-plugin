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


class ProviderSchemaTests(unittest.TestCase):
    def setUp(self):
        self.source = "codex-rs/model-provider/src/provider.rs"
        self.test = "codex-rs/model-provider/src/provider_endpoint_tests.rs"
        self.map_path = "upstream/p03-provider-endpoint-lineage.json"
        self.parent = {}
        self.current = {}
        self.objects = Objects(
            {"1" * 40: {}, "2" * 40: self.current, "5" * 40: self.parent}, {}
        )
        self.objects.commits.update({"3" * 40: "2" * 40, "6" * 40: "5" * 40})
        self.put(self.parent, self.source, b"old native provider")
        self.put(
            self.current, self.source, b"ModelProvider ConfiguredModelProvider::new"
        )
        self.put(self.current, self.test, b"new tests")
        references = [
            "AGENTS.md",
            "codex-rs/models-manager/src/manager.rs",
            "codex-rs/models-manager/src/cache.rs",
            "codex-rs/model-provider/Cargo.toml",
            *[
                "codex-rs/model-provider/src/" + name
                for name in (
                    "models_endpoint.rs",
                    "amazon_bedrock/mod.rs",
                    "amazon_bedrock/catalog.rs",
                    "auth.rs",
                    "shared_state_test_support.rs",
                    "models_identity.rs",
                    "lib.rs",
                )
            ],
        ]
        for path in references:
            self.put(self.parent, path, path.encode())
            self.put(self.current, path, path.encode())
        receipt = {"path": "/unread/fixture", "sha256": "a" * 64, "bytes": 1}
        self.document = {
            "schema_version": 1,
            "classification": "Native compiled provider endpoint authority boundary prerequisite; not independent extraction",
            "upstream_revision": "4" * 40,
            "upstream_mapping_basis": "fixture",
            "publication_parent": "6" * 40,
            "adoption": {
                "receipt": receipt,
                "stage_manifest": receipt,
                "archive": "/unread/archive",
                "archive_sha256": "b" * 64,
                "exact_stage_adoption": True,
                "durability": "fixture",
            },
            "source_paths": [
                {
                    "path": path,
                    "base_sha256": validator.digest(b"old native provider")
                    if path == self.source
                    else None,
                    "adopted_and_tested_sha256": self.current_hash(path),
                    "formatted_sha256": self.current_hash(path),
                    "formatted_bytes": len(
                        self.objects.blobs[self.current[path]["blob"]]
                    ),
                }
                for path in (self.source, self.test)
            ],
            "symbol_mapping": [
                {
                    "path": self.source,
                    "existing_symbols": ["ModelProvider"],
                    "new_symbols": ["ModelProvider::models_endpoint"],
                    "change": "fixture",
                },
                {
                    "path": self.test,
                    "new_symbols": ["five native provider capability tests"],
                    "change": "fixture",
                },
            ],
            "unchanged_reference_paths": [
                {"path": path, "sha256": self.current_hash(path)} for path in references
            ],
            "intentional_customization": "fixture",
            "preserved_boundaries": ["fixture"],
            "source_chain": {
                "source_entries": 2,
                "first_before_after_equals_retry_before_after_equals_fix_before_after_equals_format_before": True,
                "tested_map_sha256": "c" * 64,
                "formatted_map_sha256": "d" * 64,
                "formatted_changed_paths": [],
                "format_review": "fixture",
                "retested_after_format": False,
            },
            "test_boundary": "fixture",
            "limitations": ["fixture"],
        }
        self.index = {
            "schema_version": 1,
            "source": {
                "import_tree": "1" * 40,
                "checkpoint_tree": "2" * 40,
                "checkpoint_commit": "3" * 40,
                "upstream_revision": "4" * 40,
                "publication_normalization": {
                    "source_publication": {"parent": "6" * 40}
                },
            },
            "boundary_edges": [],
        }

    def put(self, tree, path, data):
        oid, data = blob(data)
        self.objects.blobs[oid] = data
        tree[path] = {"mode": "100644", "type": "blob", "blob": oid}

    def current_hash(self, path):
        return validator.digest(self.objects.blobs[self.current[path]["blob"]])

    def result(self):
        # Rebind the synthetic map itself so corruptions exercise the adapter, not the outer hash guard.
        self.put(self.current, self.map_path, json.dumps(self.document).encode())
        self.index["paths"] = [
            {
                "path": path,
                "before": None,
                "after": entry,
                "inventory_owners": ["C27"],
                "semantic_status": "unresolved",
            }
            for path, entry in self.current.items()
        ]
        self.index["historical_maps"] = [
            {
                "path": self.map_path,
                "blob": self.current[self.map_path]["blob"],
                "sha256": self.current_hash(self.map_path),
            }
        ]
        return validator.validate(self.index, self.objects)

    def test_provider_schema_checks_sources_and_preserves_unresolved_claims(self):
        result = self.result()
        self.assertEqual(result["invalid"], 0)
        self.assertEqual(result["supported_historical_maps"], 1)
        self.assertEqual(result["counts"]["unresolved:provider_nonliteral_symbol"], 2)
        self.assertIn(
            "unresolved:provider_historical_evidence_unverified", result["counts"]
        )
        self.assertEqual(result["status"], "unresolved")
        self.assertFalse(result["runtime_or_updater_acceptance"])

    def test_provider_formatted_hash_bytes_and_base_tampering_are_invalid(self):
        row = self.document["source_paths"][0]
        for key, replacement, code in (
            ("formatted_sha256", "f" * 64, "provider_formatted_source_mismatch"),
            (
                "formatted_bytes",
                row["formatted_bytes"] + 1,
                "provider_formatted_source_mismatch",
            ),
            ("base_sha256", "f" * 64, "provider_base_source_mismatch"),
            ("base_sha256", None, "provider_base_source_mismatch"),
        ):
            with self.subTest(key=key, replacement=replacement):
                old = row[key]
                row[key] = replacement
                self.assertIn("invalid:" + code, self.result()["counts"])
                row[key] = old

    def test_provider_new_file_absence_requires_available_parent(self):
        self.objects.commits.pop("6" * 40)
        result = self.result()
        self.assertEqual(result["invalid"], 0)
        self.assertIn("unresolved:provider_parent_tree_unavailable", result["counts"])
        self.objects.commits["6" * 40] = "5" * 40
        self.put(self.parent, self.test, b"already existed")
        self.assertIn("invalid:provider_base_source_mismatch", self.result()["counts"])

    def test_provider_reference_hash_and_unchanged_claim_are_checked(self):
        row = self.document["unchanged_reference_paths"][0]
        self.put(self.parent, row["path"], b"different prior reference")
        row["sha256"] = "f" * 64
        counts = self.result()["counts"]
        self.assertIn("invalid:provider_reference_hash_mismatch", counts)
        self.assertIn("invalid:provider_reference_changed", counts)

    def test_provider_path_coverage_and_duplicates_are_checked(self):
        for key, code in (
            ("source_paths", "provider_source_path_coverage"),
            ("symbol_mapping", "provider_symbol_path_coverage"),
            ("unchanged_reference_paths", "provider_reference_path_coverage"),
        ):
            with self.subTest(key=key):
                rows = self.document[key]
                self.document[key] = rows + [rows[0]]
                self.assertIn("invalid:" + code, self.result()["counts"])
                self.document[key] = rows[1:]
                self.assertIn("invalid:" + code, self.result()["counts"])
                self.document[key] = rows

    def test_provider_empty_symbol_lists_do_not_silently_weaken_checks(self):
        for field in ("existing_symbols", "new_symbols"):
            row = self.document["symbol_mapping"][0]
            old = row[field]
            row[field] = []
            self.assertIn(
                "invalid:provider_symbol_fields_mismatch", self.result()["counts"]
            )
            row[field] = old

    def test_provider_revision_binding_is_checked(self):
        for field, code in (
            ("upstream_revision", "provider_upstream_revision_mismatch"),
            ("publication_parent", "provider_parent_revision_mismatch"),
        ):
            old = self.document[field]
            self.document[field] = "f" * 40
            self.assertIn("invalid:" + code, self.result()["counts"])
            self.document[field] = old

    def test_provider_unknown_or_weakened_nested_shapes_are_not_supported(self):
        row = self.document["source_paths"][0]
        original = row["formatted_bytes"]
        for value in (True, "1"):
            row["formatted_bytes"] = value
            self.assertEqual(self.result()["supported_historical_maps"], 0)
            self.assertIn(
                "unresolved:provider_schema_unsupported", self.result()["counts"]
            )
        row["formatted_bytes"] = original
        row["invented_field"] = "unknown"
        self.assertEqual(self.result()["supported_historical_maps"], 0)
        del row["invented_field"]
        del self.document["adoption"]["receipt"]
        self.assertEqual(self.result()["supported_historical_maps"], 0)

    def test_provider_different_tested_bytes_remain_unverified(self):
        self.document["source_paths"][1]["adopted_and_tested_sha256"] = "f" * 64
        self.document["source_chain"]["formatted_changed_paths"] = [self.test]
        result = self.result()
        self.assertEqual(result["invalid"], 0)
        self.assertIn("unresolved:provider_tested_source_unavailable", result["counts"])
        self.document["source_chain"]["formatted_changed_paths"] = []
        self.assertIn("invalid:provider_format_path_mismatch", self.result()["counts"])


if __name__ == "__main__":
    unittest.main()
