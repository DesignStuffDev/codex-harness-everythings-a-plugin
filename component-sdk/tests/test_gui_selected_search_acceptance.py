"""Provenance rejection tests; synthetic files, not native/runtime acceptance."""

import json
from pathlib import Path
import tempfile
import unittest

import gui_selected_search_acceptance as selected
from file_search.acceptance_support import fingerprint, inventory


class SearchReuseProofTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.old_host = self.root / "original-manager"
        self.old_host.write_bytes(b"original manager")
        self.host = self.root / "current-manager"
        self.host.write_bytes(b"new current manager")
        self.artifacts = self.root / "independent"
        self.package = self.artifacts / "package"
        self.package.mkdir(parents=True)
        self.worker = self.package / "worker"
        self.worker.write_bytes(b"independently built worker")
        self.worker.chmod(0o700)
        self.source_inventory = self.artifacts / "source-inventory.json"
        self.source_inventory.write_text('{"revision":"fixture-only"}\n')
        manifest = {
            "id": selected.PLUGIN,
            "api_version": 1,
            "entrypoint": "worker",
            "components": [
                {"kind": "file_search", "name": "default", "contract_version": 1}
            ],
        }
        (self.package / "codex-component.json").write_text(json.dumps(manifest))
        original_binaries = {"manager": fingerprint(self.old_host)}
        self.proof = {
            "passed": True,
            "evidence_kind": "independent_source_build",
            "artifact_directory": str(self.artifacts),
            "package_path": str(self.package),
            "package_files": inventory(self.package),
            "build_artifact": {
                "separate_initially_empty_target": True,
                "fresh": False,
                "fingerprint": fingerprint(self.worker),
            },
            "source_resolution": {
                "all_local_paths_inside_export": True,
                "lock_new_identities": 0,
            },
            "final_locked_build_preserved_lock_bytes": True,
            "source_parked_before_runtime": True,
            "exported_source_inventory": fingerprint(self.source_inventory),
            "frozen_binaries_before": original_binaries,
            "frozen_binaries_after": dict(original_binaries),
        }
        self.proof_path = self.artifacts / "independent-build.json"
        self.write_proof()
        self.pin = fingerprint(self.proof_path)["sha256"]

    def write_proof(self):
        self.proof_path.write_text(json.dumps(self.proof) + "\n")

    def validate_reuse(self):
        return selected.validate_build(
            self.proof_path, self.host, reuse_proof_sha256=self.pin
        )

    def test_original_host_mode_still_rejects_new_manager(self):
        with self.assertRaisesRegex(AssertionError, "manager differs"):
            selected.validate_build(self.proof_path, self.host)
        selected.validate_build(self.proof_path, self.old_host)

    def test_explicit_reuse_keeps_original_proof_and_package_unchanged(self):
        before = fingerprint(self.proof_path), inventory(self.package)
        self.old_host.unlink()  # Historical host need not remain installed.
        proof_path, proof, package, _ = self.validate_reuse()
        self.assertEqual(
            (proof_path, proof, package), (self.proof_path, self.proof, self.package)
        )
        self.assertEqual(
            before, (fingerprint(self.proof_path), inventory(self.package))
        )

    def test_modified_package_is_rejected(self):
        self.worker.write_bytes(b"substituted worker")
        with self.assertRaisesRegex(AssertionError, "search package changed"):
            self.validate_reuse()

    def test_extra_package_file_is_rejected(self):
        (self.package / "unreviewed").write_text("extra")
        with self.assertRaisesRegex(AssertionError, "search package changed"):
            self.validate_reuse()

    def test_package_symlink_is_rejected(self):
        self.worker.unlink()
        self.worker.symlink_to(self.host)
        with self.assertRaisesRegex(AssertionError, "symlink or special file"):
            self.validate_reuse()

    def test_report_substitution_is_rejected_by_exact_pin(self):
        self.proof["source_resolution"]["lock_new_identities"] = 1
        self.write_proof()
        with self.assertRaisesRegex(AssertionError, "reviewed SHA-256"):
            self.validate_reuse()

    def test_modified_source_inventory_is_rejected(self):
        self.source_inventory.write_text('{"revision":"substituted"}\n')
        with self.assertRaisesRegex(
            AssertionError, "source paths or inventory changed"
        ):
            self.validate_reuse()

    def test_restored_original_source_path_is_rejected(self):
        (self.artifacts / "source").mkdir()
        with self.assertRaisesRegex(
            AssertionError, "source paths or inventory changed"
        ):
            self.validate_reuse()

    def test_explicit_pin_does_not_waive_original_build_consistency(self):
        self.proof["frozen_binaries_after"] = {"manager": fingerprint(self.host)}
        self.write_proof()
        self.pin = fingerprint(self.proof_path)["sha256"]
        with self.assertRaisesRegex(
            AssertionError, "original independent-build binaries changed"
        ):
            self.validate_reuse()

    def test_explicit_pin_does_not_waive_source_provenance(self):
        self.proof["source_resolution"]["all_local_paths_inside_export"] = False
        self.write_proof()
        self.pin = fingerprint(self.proof_path)["sha256"]
        with self.assertRaisesRegex(AssertionError, "compilation/provenance"):
            self.validate_reuse()

    def test_final_recheck_rejects_manager_or_proof_or_package_change(self):
        evidence = {
            "frozen_manager": fingerprint(self.host),
            "build_report": str(self.proof_path),
            "build_report_fingerprint": fingerprint(self.proof_path),
            "reuse_proof_sha256": self.pin,
        }
        selected.verify_inputs(self.host, evidence)
        self.host.write_bytes(b"changed during runtime")
        with self.assertRaisesRegex(AssertionError, "runtime manager changed"):
            selected.verify_inputs(self.host, evidence)
        evidence["frozen_manager"] = fingerprint(self.host)
        self.worker.write_bytes(b"changed after installation")
        with self.assertRaisesRegex(AssertionError, "search package changed"):
            selected.verify_inputs(self.host, evidence)
        self.proof_path.write_text(self.proof_path.read_text() + " ")
        with self.assertRaisesRegex(AssertionError, "build report changed during GUI"):
            selected.verify_inputs(self.host, evidence)


if __name__ == "__main__":
    unittest.main()
