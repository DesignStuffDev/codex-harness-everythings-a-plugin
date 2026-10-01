"""Fixtures for full-lock-only unused patch pruning; never run Cargo."""

import copy
import hashlib
import json
from pathlib import Path
import runpy
import tempfile
import unittest

from lock_patch_normalization import (
    assert_package_records_unchanged,
    prepare_normalization,
    unused_patch_plan,
)

REVISION = "a" * 40
URL = "https://example.invalid/unused"
SOURCE = f"git+{URL}?rev={REVISION}#{REVISION}"
DECLARATION = {"git": URL, "rev": REVISION}
UNUSED = {"name": "unused", "version": "1.0.0", "source": SOURCE}


def documents():
    return (
        {
            "workspace": {"members": ["worker"]},
            "patch": {
                "crates-io": {
                    "unused": DECLARATION.copy(),
                    "native": {"path": "native"},
                },
                "ssh://git@example.invalid/unused": {"unused": DECLARATION.copy()},
            },
        },
        {
            "version": 4,
            "package": [{"name": "worker", "version": "1.0.0"}],
            "patch": {"unused": [UNUSED.copy(), UNUSED.copy()]},
        },
    )


class UnusedPatches(unittest.TestCase):
    def test_duplicate_unused_origins_removed_but_path_patch_retained(self):
        manifest, lock = documents()
        original = copy.deepcopy(manifest)
        result, removals = unused_patch_plan(manifest, lock)
        self.assertEqual(result["patch"], {"crates-io": {"native": {"path": "native"}}})
        self.assertEqual(len(removals), 2)
        self.assertEqual(manifest, original)

    def test_full_lock_used_name_retains_patch_even_when_platform_metadata_omits_it(
        self,
    ):
        manifest, lock = documents()
        lock["package"].append(
            {
                "name": "unused",
                "version": "2.0.0",
                "source": "registry+https://example.invalid/index",
                "checksum": "preserved",
            }
        )
        self.assertEqual(unused_patch_plan(manifest, lock), (manifest, []))

    def test_unmatched_or_ambiguous_receipts_cannot_authorize_removal(self):
        manifest, lock = documents()
        lock["patch"]["unused"] = []
        self.assertEqual(unused_patch_plan(manifest, lock), (manifest, []))
        lock["patch"]["unused"] = [UNUSED, {**UNUSED, "version": "2.0.0"}]
        self.assertEqual(unused_patch_plan(manifest, lock), (manifest, []))

    def test_duplicate_declarations_require_matching_receipt_multiplicity(self):
        manifest, lock = documents()
        lock["patch"]["unused"].pop()
        result, removals = unused_patch_plan(manifest, lock)
        self.assertEqual(len(removals), 1)
        self.assertEqual(
            result["patch"]["ssh://git@example.invalid/unused"]["unused"], DECLARATION
        )

    def test_unpinned_or_extra_patch_options_are_retained(self):
        for extra in ({"rev": "main"}, {"branch": "main"}, {"version": "1"}):
            manifest, lock = documents()
            for patches in manifest["patch"].values():
                if "unused" in patches:
                    patches["unused"].update(extra)
            self.assertEqual(unused_patch_plan(manifest, lock), (manifest, []))

    def test_package_alias_uses_actual_package_identity(self):
        manifest, lock = documents()
        manifest["patch"] = {
            "crates-io": {"renamed": {**DECLARATION, "package": "unused"}}
        }
        result, removed = unused_patch_plan(manifest, lock)
        self.assertEqual(result["patch"], {})
        self.assertEqual(removed[0]["alias"], "renamed")

    def test_exact_graph_gate_rejects_dependency_checksum_and_package_changes(self):
        _, before = documents()
        for altered in (
            [{**before["package"][0], "dependencies": ["unexpected"]}],
            [{**before["package"][0], "checksum": "different"}],
            before["package"] + [{"name": "extra", "version": "1.0.0"}],
        ):
            with self.assertRaises(AssertionError):
                assert_package_records_unchanged(before, {**before, "package": altered})
        assert_package_records_unchanged(before, {**before, "patch": {"unused": []}})

    def test_isolated_rewrite_preserves_originals_and_updates_only_export_manifest_hash(
        self,
    ):
        serializer = runpy.run_path(
            str(Path(__file__).resolve().parents[2] / "rust_component_package.py")
        )["dump_toml"]
        with tempfile.TemporaryDirectory() as directory:
            artifacts = Path(directory)
            source = artifacts / "source"
            workspace = source / "codex-rs"
            workspace.mkdir(parents=True)
            manifest, lock = documents()
            manifest_path = workspace / "Cargo.toml"
            manifest_path.write_text(serializer(manifest))
            # Inline tables are valid TOML for these deliberately small fixtures.
            (workspace / "Cargo.lock").write_text(serializer(lock))
            digest = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
            initial = {
                "source_hashes": [
                    {
                        "path": "codex-rs/Cargo.toml",
                        "source_sha256": "upstream-original",
                        "export_sha256": digest,
                    }
                ]
            }
            (source / "COMPONENT_SOURCE_EXPORT.json").write_text(json.dumps(initial))
            report, prior_lock = prepare_normalization(source, artifacts, serializer)
            self.assertTrue(report["applied"])
            self.assertEqual(prior_lock, lock)
            self.assertEqual(
                json.loads((artifacts / "original-source-inventory.json").read_text()),
                initial,
            )
            self.assertEqual(
                hashlib.sha256(
                    (
                        artifacts / "manifest-before-patch-normalization.toml"
                    ).read_bytes()
                ).hexdigest(),
                digest,
            )
            updated = json.loads((source / "COMPONENT_SOURCE_EXPORT.json").read_text())
            self.assertEqual(
                updated["source_hashes"][0]["source_sha256"], "upstream-original"
            )
            self.assertEqual(
                updated["source_hashes"][0]["export_sha256"],
                hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
            )
            with self.assertRaises(AssertionError):
                prepare_normalization(workspace, artifacts, serializer)


if __name__ == "__main__":
    unittest.main()
