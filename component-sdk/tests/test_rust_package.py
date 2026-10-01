"""Exercise source closure/export without compiling or touching the Rust workspace."""

import importlib.util
import contextlib
import io
import json
from pathlib import Path
import tempfile
import shutil
import subprocess
import sys
import tomllib
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "rust_component_package.py"
SPEC = importlib.util.spec_from_file_location("rust_component_package", SCRIPT)
packaging = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(packaging)


class NativeSourceExportTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.repo = self.base / "repository"
        self.workspace = self.repo / "rust"
        self.workspace.mkdir(parents=True)
        self.write("LICENSE", "license")
        self.write("NOTICE", "notice")
        self.write(".cargo/config.toml", '[env]\nA = "value"\n')
        self.write("rust/rust-toolchain.toml", '[toolchain]\nchannel = "1.95.0"\n')
        self.write("rust/Cargo.lock", "version = 4\n")
        self.write(
            "rust/Cargo.toml",
            """[workspace]
members = ["*"]
resolver = "2"
[workspace.package]
version = "0.1.0"
edition = "2024"
[workspace.dependencies]
optional_alias = { package = "optional", path = "optional" }
dev_alias = { package = "dev", path = "dev" }
unused_alias = { path = "missing" }
[patch.crates-io]
patched = { path = "patched" }
[profile.dev]
debug = 1
""",
        )
        for name in (
            "plugin",
            "ordinary",
            "builder",
            "platform",
            "optional",
            "dev",
            "patched",
        ):
            self.write(
                f"rust/{name}/Cargo.toml",
                f'''[package]
name = "{name}"
version.workspace = true
edition.workspace = true
''',
            )
            self.write(f"rust/{name}/src/lib.rs", "pub fn value() {}\n")
        with (self.workspace / "plugin/Cargo.toml").open("a") as manifest:
            manifest.write("""[dependencies]
ordinary = { path = "../ordinary" }
optional_alias = { workspace = true, optional = true }
[build-dependencies]
builder = { path = "../builder" }
[target.'cfg(windows)'.dependencies]
platform = { path = "../platform" }
[dev-dependencies]
dev_alias.workspace = true
[target.'cfg(unix)'.dev-dependencies]
dev_alias.workspace = true
""")
        self.write("shared/prompt.txt", "shared prompt")
        self.write(
            "rust/plugin/src/lib.rs",
            'const PROMPT: &str = include_str!("../../../shared/prompt.txt");\n',
        )
        self.write("rust/plugin/assets/schema.json", '{"schema": 1}')
        self.write("rust/plugin/target/generated.bin", "not source")
        self.write(
            "rust/plugin/test-support/Cargo.toml", '[package]\nname="nested-dev"\n'
        )
        self.write("rust/plugin/test-support/src/lib.rs", "not production")

    def write(self, relative, text):
        path = self.repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def plan(self):
        return packaging.ExportPlan(self.repo, "rust", "plugin")

    def nested_vendor(self, *, include_unused=False):
        with (self.workspace / "plugin/Cargo.toml").open("a") as manifest:
            manifest.write('\n[dependencies.nucleo]\npath = "../vendor/nucleo"\n')
        members = '["matcher", "unused"]' if include_unused else '["matcher"]'
        self.write(
            "rust/vendor/nucleo/Cargo.toml",
            f"""# Keep upstream formatting when unchanged.
[package]
name = "nucleo"
version = "0.5.0"
edition = "2021"
license = "MPL-2.0"
[workspace]
members = {members}
[dependencies]
nucleo-matcher = {{ path = "matcher", version = "0.3.1" }}
""",
        )
        self.write(
            "rust/vendor/nucleo/src/lib.rs", "// Covered source\npub fn search() {}\n"
        )
        self.write("rust/vendor/nucleo/LICENSE", "Mozilla Public License Version 2.0\n")
        self.write(
            "rust/vendor/nucleo/PROVENANCE.md",
            "Pinned vendor revision and local changes.\n",
        )
        self.write(
            "rust/vendor/nucleo/matcher/Cargo.toml",
            """# Separate workspace member.
[package]
name = "nucleo-matcher"
version = "0.3.1"
edition = "2021"
license = "MPL-2.0"
[dependencies]
ordinary = { path = "../../../ordinary" }
""",
        )
        self.write(
            "rust/vendor/nucleo/matcher/src/lib.rs",
            "// Matcher source\npub fn score() {}\n",
        )
        self.write(
            "rust/vendor/nucleo/matcher/LICENSE", "Mozilla Public License Version 2.0\n"
        )
        self.write(
            "rust/vendor/nucleo/unused/Cargo.toml",
            '[package]\nname="unused"\nversion="0.1.0"\n',
        )
        self.write(
            "rust/vendor/nucleo/unused/src/lib.rs", "not a production dependency"
        )
        self.write(
            "rust/vendor/nucleo/matcher/fuzz/Cargo.toml",
            '[package]\nname="fuzz"\nversion="0.1.0"\n',
        )
        self.write(
            "rust/vendor/nucleo/matcher/fuzz/src/lib.rs", "not a production dependency"
        )
        return self.workspace / "vendor/nucleo"

    def test_nested_dependency_export_preserves_separate_workspace_and_covered_source(
        self,
    ):
        vendor = self.nested_vendor(include_unused=True)
        original = {
            path: path.read_bytes() for path in vendor.rglob("*") if path.is_file()
        }
        output = self.base / "nested-export"
        self.plan().export(output)
        root = tomllib.loads((output / "rust/Cargo.toml").read_text())["workspace"]
        self.assertEqual(root["exclude"], ["vendor/nucleo"])
        self.assertEqual(
            root["members"],
            ["builder", "optional", "ordinary", "patched", "platform", "plugin"],
        )
        exported_vendor = output / "rust/vendor/nucleo"
        nested = tomllib.loads((exported_vendor / "Cargo.toml").read_text())
        self.assertEqual(nested["workspace"]["members"], ["matcher"])
        self.assertFalse((exported_vendor / "unused").exists())
        self.assertFalse((exported_vendor / "matcher/fuzz").exists())
        for name in (
            "src/lib.rs",
            "LICENSE",
            "PROVENANCE.md",
            "matcher/src/lib.rs",
            "matcher/LICENSE",
            "matcher/Cargo.toml",
        ):
            self.assertEqual(
                (exported_vendor / name).read_bytes(), original[vendor / name]
            )
        self.assertEqual({path: path.read_bytes() for path in original}, original)
        inventory = json.loads((output / "COMPONENT_SOURCE_EXPORT.json").read_text())
        self.assertEqual(inventory["nested_workspaces"], ["vendor/nucleo"])
        self.assertTrue(
            any(
                edge["from"] == "nucleo" and edge["to"] == "vendor/nucleo/matcher"
                for edge in inventory["local_edges"]
            )
        )
        self.assertTrue(
            any(
                edge["from"] == "nucleo-matcher" and edge["to"] == "ordinary"
                for edge in inventory["local_edges"]
            )
        )

    def test_unchanged_nested_manifests_keep_exact_upstream_bytes(self):
        vendor = self.nested_vendor()
        root_path = self.workspace / "Cargo.toml"
        root = tomllib.loads(root_path.read_text())
        root["workspace"]["exclude"] = ["vendor/nucleo", "unrelated-exclusion"]
        root_path.write_text(packaging.dump_toml(root))
        output = self.base / "unchanged-export"
        self.plan().export(output)
        for name in ("Cargo.toml", "matcher/Cargo.toml"):
            self.assertEqual(
                (output / "rust/vendor/nucleo" / name).read_bytes(),
                (vendor / name).read_bytes(),
            )
        exported = tomllib.loads((output / "rust/Cargo.toml").read_text())["workspace"]
        self.assertEqual(exported["exclude"], ["unrelated-exclusion", "vendor/nucleo"])

    def test_nested_member_selected_without_root_retains_declaring_package(self):
        vendor = self.nested_vendor()
        plugin_path = self.workspace / "plugin/Cargo.toml"
        plugin = tomllib.loads(plugin_path.read_text())
        plugin["dependencies"].pop("nucleo")
        root_path = self.workspace / "Cargo.toml"
        root = tomllib.loads(root_path.read_text())
        for selection in ("path_patch", "direct_member"):
            with self.subTest(selection=selection):
                if selection == "path_patch":
                    root["patch"]["crates-io"]["nucleo-matcher"] = {
                        "path": "vendor/nucleo/matcher"
                    }
                else:
                    root["patch"]["crates-io"].pop("nucleo-matcher")
                    plugin["dependencies"]["nucleo-matcher"] = {
                        "path": "../vendor/nucleo/matcher"
                    }
                plugin_path.write_text(packaging.dump_toml(plugin))
                root_path.write_text(packaging.dump_toml(root))
                plan = self.plan()
                self.assertIn(vendor, plan.crates)
                self.assertIn(vendor / "matcher", plan.crates)
                retention = plan.describe()["nested_workspace_owner_retention"]
                self.assertEqual(
                    [(item["path"], item["selected_members"]) for item in retention],
                    [("rust/vendor/nucleo", ["rust/vendor/nucleo/matcher"])],
                )
                self.assertFalse(
                    any(
                        edge["from"] == "plugin" and edge["to"] == "vendor/nucleo"
                        for edge in plan.edges
                    )
                )
                self.assertTrue(
                    any(
                        edge["from"] == "nucleo"
                        and edge["to"] == "vendor/nucleo/matcher"
                        for edge in plan.edges
                    )
                )
                output = self.base / selection
                plan.export(output)
                exported_root = tomllib.loads((output / "rust/Cargo.toml").read_text())
                self.assertEqual(
                    exported_root["workspace"]["default-members"], ["plugin"]
                )
                self.assertEqual(
                    exported_root["workspace"]["exclude"], ["vendor/nucleo"]
                )
                self.assertFalse(
                    any(
                        member.startswith("vendor/")
                        for member in exported_root["workspace"]["members"]
                    )
                )
                self.assertEqual(
                    (output / "rust/vendor/nucleo/Cargo.toml").read_bytes(),
                    (vendor / "Cargo.toml").read_bytes(),
                )
                self.assertEqual(
                    "nucleo-matcher" in exported_root["patch"]["crates-io"],
                    selection == "path_patch",
                )

    def test_nested_inheritance_is_rejected_instead_of_using_outer_workspace(self):
        vendor = self.nested_vendor()
        manifest = vendor / "matcher/Cargo.toml"
        original = manifest.read_text()
        cases = [
            (
                original.replace('version = "0.3.1"', "version.workspace = true"),
                "inherited package",
            ),
            (
                original + "\n[dependencies.optional_alias]\nworkspace = true\n",
                "inherited dependency",
            ),
            (original + "\n[lints]\nworkspace = true\n", "inherited lint"),
        ]
        for content, error in cases:
            with self.subTest(error=error):
                manifest.write_text(content)
                with self.assertRaisesRegex(ValueError, error):
                    self.plan()
        manifest.write_text(original)

    def test_nested_dependency_paths_keep_portability_guards(self):
        vendor = self.nested_vendor()
        manifest = vendor / "matcher/Cargo.toml"
        original = manifest.read_text()
        for dependency in (str(self.workspace / "ordinary"), "../../../../outside"):
            with self.subTest(dependency=dependency):
                manifest.write_text(
                    original.replace('"../../../ordinary"', json.dumps(dependency))
                )
                with self.assertRaises((ValueError, FileNotFoundError)):
                    self.plan()

    def covered_source_export(self):
        vendor = self.nested_vendor()
        (vendor / "README.md").write_text("Nucleo upstream documentation.\n")
        (vendor / "PROVENANCE.md").write_text(
            "https://github.com/helix-editor/nucleo\n"
            "4253de9faabb4e5c6d81d946a5e35a90f87347ee\n"
            "Local covered changes retain MPL-2.0.\n"
        )
        destination = self.workspace / "third-party/nucleo"
        destination.parent.mkdir()
        vendor.rename(destination)
        manifest = self.workspace / "plugin/Cargo.toml"
        manifest.write_text(
            manifest.read_text().replace("../vendor/nucleo", "../third-party/nucleo")
        )
        exported = self.base / "covered-export"
        self.plan().export(exported)
        return exported

    def test_native_package_carries_inventoried_mpl_source_and_keeps_root_notices(self):
        exported = self.covered_source_export()
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        output = self.base / "covered-package"
        packaging.assemble(
            exported, binary, output, "example.native", "thread_store", "default"
        )
        notice = json.loads((output / "THIRD_PARTY_NOTICES.json").read_text())
        self.assertEqual(
            [
                (item["name"], item["version"], item["license"])
                for item in notice["components"]
            ],
            [("nucleo", "0.5.0", "MPL-2.0"), ("nucleo-matcher", "0.3.1", "MPL-2.0")],
        )
        self.assertEqual((output / "LICENSE").read_text(), "license")
        self.assertEqual((output / "NOTICE").read_text(), "notice")
        inventory = json.loads((exported / "COMPONENT_SOURCE_EXPORT.json").read_text())
        expected = {
            item["path"]: item
            for item in inventory["source_hashes"]
            if item["path"].startswith("rust/third-party/nucleo/")
        }
        self.assertEqual(
            {item["source_path"] for item in notice["files"]}, set(expected)
        )
        for item in notice["files"]:
            self.assertEqual(
                (output / item["package_path"]).read_bytes(),
                (exported / item["source_path"]).read_bytes(),
            )
            self.assertEqual(
                item["source_sha256"], expected[item["source_path"]]["source_sha256"]
            )
            self.assertEqual(item["package_sha256"], item["export_sha256"])
        self.assertIn("not a complete transitive-license", notice["scope"])

    def test_mpl_package_rejects_source_tampering_before_creating_output(self):
        exported = self.covered_source_export()
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        source = exported / "rust/third-party/nucleo/src/lib.rs"
        source.write_text(source.read_text() + "// Changed after export\n")
        output = self.base / "tampered-package"
        with self.assertRaisesRegex(ValueError, "hash does not match export"):
            packaging.assemble(
                exported, binary, output, "example.native", "thread_store", "default"
            )
        self.assertFalse(output.exists())

    def test_mpl_package_rejects_inventory_escape_and_missing_covered_files(self):
        exported = self.covered_source_export()
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        path = exported / "COMPONENT_SOURCE_EXPORT.json"
        original = path.read_text()
        for mode in ("escape", "missing", "omitted", "source_hash"):
            with self.subTest(mode=mode):
                inventory = json.loads(original)
                item = next(
                    entry
                    for entry in inventory["source_hashes"]
                    if entry["path"].endswith("nucleo/src/lib.rs")
                )
                if mode == "escape":
                    item["path"] = "../outside.rs"
                elif mode == "missing":
                    item["path"] = "rust/third-party/nucleo/src/missing.rs"
                elif mode == "omitted":
                    inventory["source_hashes"].remove(item)
                else:
                    item["source_sha256"] = "0" * 64
                path.write_text(json.dumps(inventory))
                output = self.base / "bad-inventory-package"
                with self.assertRaises(ValueError):
                    packaging.assemble(
                        exported,
                        binary,
                        output,
                        "example.native",
                        "thread_store",
                        "default",
                    )
                self.assertFalse(output.exists())

    def test_non_mpl_export_assembly_unchanged_and_vendor_checkout_requires_export(
        self,
    ):
        exported = self.base / "ordinary-export"
        self.plan().export(exported)
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        output = self.base / "ordinary-package"
        packaging.assemble(
            exported, binary, output, "example.native", "tool", "default"
        )
        self.assertEqual(
            {path.name for path in output.iterdir()},
            {"native-plugin", "LICENSE", "NOTICE", "codex-component.json"},
        )
        self.covered_source_export()
        output = self.base / "unexported-package"
        with self.assertRaisesRegex(ValueError, "requires a source export inventory"):
            packaging.assemble(
                self.repo, binary, output, "example.native", "tool", "default"
            )
        self.assertFalse(output.exists())

    def test_mpl_package_rejects_covered_symlink_and_cross_platform_escape_paths(self):
        exported = self.covered_source_export()
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        path = exported / "COMPONENT_SOURCE_EXPORT.json"
        original = path.read_text()
        for invalid in ("/absolute.rs", "C:\\outside.rs", "rust\\..\\outside.rs"):
            with self.subTest(path=invalid):
                inventory = json.loads(original)
                inventory["source_hashes"][0]["path"] = invalid
                path.write_text(json.dumps(inventory))
                with self.assertRaises(ValueError):
                    packaging.assemble(
                        exported,
                        binary,
                        self.base / "escape-package",
                        "example.native",
                        "tool",
                        "default",
                    )
        path.write_text(original)
        source = exported / "rust/third-party/nucleo/src/lib.rs"
        outside = self.base / "outside.rs"
        outside.write_bytes(source.read_bytes())
        source.unlink()
        source.symlink_to(outside)
        with self.assertRaisesRegex(ValueError, "symlinks"):
            packaging.assemble(
                exported,
                binary,
                self.base / "symlink-package",
                "example.native",
                "tool",
                "default",
            )
        self.assertFalse((self.base / "symlink-package").exists())

    def test_export_preserves_production_closure_and_resources(self):
        original = {
            path: packaging.digest(path) for path in self.repo.rglob("Cargo.toml")
        }
        output = self.base / "export"
        self.plan().export(output)
        root = tomllib.loads((output / "rust/Cargo.toml").read_text())
        self.assertEqual(
            root["workspace"]["members"],
            ["builder", "optional", "ordinary", "patched", "platform", "plugin"],
        )
        self.assertEqual(
            root["workspace"]["dependencies"],
            {"optional_alias": {"package": "optional", "path": "optional"}},
        )
        plugin = tomllib.loads((output / "rust/plugin/Cargo.toml").read_text())
        self.assertNotIn("dev-dependencies", plugin)
        self.assertNotIn("dev-dependencies", plugin["target"]["cfg(unix)"])
        self.assertEqual((output / "shared/prompt.txt").read_text(), "shared prompt")
        self.assertTrue((output / "rust/plugin/assets/schema.json").is_file())
        self.assertFalse((output / "rust/dev").exists())
        self.assertFalse((output / "rust/plugin/target").exists())
        self.assertFalse((output / "rust/plugin/test-support").exists())
        self.assertEqual({path: packaging.digest(path) for path in original}, original)
        report = json.loads((output / "COMPONENT_SOURCE_EXPORT.json").read_text())
        inventory = {entry["path"]: entry for entry in report["source_hashes"]}
        self.assertNotEqual(
            inventory["rust/Cargo.toml"]["source_sha256"],
            inventory["rust/Cargo.toml"]["export_sha256"],
        )
        self.assertEqual(
            inventory["shared/prompt.txt"]["source_sha256"],
            inventory["shared/prompt.txt"]["export_sha256"],
        )
        self.assertEqual(report["independent_build_verified"], False)
        self.assertEqual((output / "LICENSE").read_text(), "license")
        self.assertEqual((output / "NOTICE").read_text(), "notice")

    def test_paths_cannot_escape_or_retain_absolute_source_dependency(self):
        self.write(
            "outside/Cargo.toml", '[package]\nname = "outside"\nversion = "0.1.0"\n'
        )
        manifest = self.workspace / "ordinary/Cargo.toml"
        initial = manifest.read_text()
        for target in ("../../outside", str(self.workspace / "builder")):
            manifest.write_text(
                initial
                + "\n[dependencies]\nexternal = { path = "
                + json.dumps(target)
                + " }\n"
            )
            with self.assertRaises((ValueError, FileNotFoundError)):
                self.plan()

    def test_include_symlink_is_rejected_before_materialization(self):
        (self.repo / "shared/alias.txt").symlink_to("prompt.txt")
        self.write(
            "rust/plugin/src/lib.rs",
            'const PROMPT: &str = include_str!("../../../shared/alias.txt");\n',
        )
        with self.assertRaisesRegex(ValueError, "symlinks"):
            self.plan()

    def test_existing_or_in_repository_output_is_not_overwritten(self):
        plan = self.plan()
        for output in (self.repo / "export", self.base):
            with self.assertRaises(ValueError):
                plan.export(output)
        self.assertFalse((self.repo / "export").exists())

    def test_toml_round_trip_handles_targets_and_inline_arrays(self):
        value = {
            "bin": [{"name": "tool", "path": "src/main.rs"}],
            "target": {
                "cfg(windows)": {
                    "dependencies": {
                        "a.b": {"version": "1", "features": ["x"], "optional": True}
                    }
                }
            },
            "package": {
                "description": 'quotes " and \\ slash\nUnicode π',
                "version": "1.0.0",
            },
        }
        self.assertEqual(tomllib.loads(packaging.dump_toml(value)), value)

    def test_assemble_retains_executable_and_notices(self):
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        binary.chmod(0o755)
        output = self.base / "package"
        packaging.assemble(
            self.repo,
            binary,
            output,
            "example.native",
            "thread_store",
            "default",
            contract_version=2,
        )
        manifest = json.loads((output / "codex-component.json").read_text())
        self.assertEqual(manifest["version"], "0.1.0")
        self.assertEqual(manifest["entrypoint"], "native-plugin")
        self.assertEqual(
            manifest["components"],
            [
                {
                    "kind": "thread_store",
                    "name": "default",
                    "contract_version": 2,
                    "metadata": {},
                }
            ],
        )
        self.assertEqual((output / "native-plugin").stat().st_mode & 0o777, 0o755)
        self.assertEqual((output / "NOTICE").read_text(), "notice")

    def test_assemble_retains_explicit_package_version(self):
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        for index, version in enumerate(
            ("0.2.0", "1.2.3-rc.1+build.01", "1.0.0-0.a-1", "1.0.0+001")
        ):
            with self.subTest(version=version):
                output = self.base / f"package-{index}"
                packaging.assemble(
                    self.repo,
                    binary,
                    output,
                    "example.native",
                    "thread_store",
                    "default",
                    contract_version=2,
                    version=version,
                )
                manifest = json.loads((output / "codex-component.json").read_text())
                self.assertEqual(manifest["version"], version)
                self.assertEqual(manifest["components"][0]["contract_version"], 2)

    def test_invalid_version_rejected_before_package_directory_is_created(self):
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        for version in (
            None,
            True,
            2,
            "",
            "0.2",
            "v0.2.0",
            "01.2.3",
            "1.2.3-01",
            "1.2.3-a..b",
            "1.2.3-rc_1",
            "1.2.3-α",
            "1.2.3+",
            "1.2.3+a..b",
            "1.2.3\n",
            "18446744073709551616.0.0",
        ):
            with self.subTest(version=version):
                output = self.base / "invalid-package"
                with self.assertRaisesRegex(ValueError, "semantic versioning"):
                    packaging.assemble(
                        self.repo,
                        binary,
                        output,
                        "example.native",
                        "thread_store",
                        "default",
                        version=version,
                    )
                self.assertFalse(output.exists())

    def test_assemble_cli_forwards_package_version(self):
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        output = self.base / "cli-package"
        argv = [
            str(SCRIPT),
            "assemble",
            "--repo",
            str(self.repo),
            "--binary",
            str(binary),
            "--output",
            str(output),
            "--id",
            "example.native",
            "--kind",
            "thread_store",
            "--contract-version",
            "2",
            "--version",
            "0.2.0",
        ]
        with patch("sys.argv", argv), contextlib.redirect_stdout(io.StringIO()):
            packaging.main()
        manifest = json.loads((output / "codex-component.json").read_text())
        self.assertEqual(
            (manifest["version"], manifest["components"][0]["contract_version"]),
            ("0.2.0", 2),
        )

    def test_component_metadata_round_trip_and_default_are_independent(self):
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        metadata = {
            "preparing_cancel_receipt": 1,
            "details": {"label": 'λ / "quote"', "flags": [True, False, None]},
            "bounds": [-(2**63), 2**64 - 1, 1.25],
        }
        output = self.base / "capable-package"
        packaging.assemble(
            self.repo,
            binary,
            output,
            "example.native",
            "file_search",
            "default",
            version="0.2.0",
            metadata=metadata,
        )
        manifest = json.loads((output / "codex-component.json").read_text())
        self.assertEqual(manifest["version"], "0.2.0")
        self.assertEqual(
            manifest["components"][0],
            {
                "kind": "file_search",
                "name": "default",
                "contract_version": 1,
                "metadata": metadata,
            },
        )
        detached = packaging.validate_component_metadata(metadata)
        metadata["details"]["flags"].append("changed")
        self.assertEqual(detached["details"]["flags"], [True, False, None])
        legacy = self.base / "legacy-package"
        packaging.assemble(
            self.repo, binary, legacy, "example.native", "file_search", "default"
        )
        old = json.loads((legacy / "codex-component.json").read_text())
        self.assertEqual(
            (old["version"], old["components"][0]["metadata"]), ("0.1.0", {})
        )

    def test_invalid_component_metadata_rejected_before_any_output_mutation(self):
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        cycle = {}
        cycle["self"] = cycle
        deep = 1
        for _ in range(packaging.MAX_METADATA_DEPTH + 1):
            deep = [deep]
        invalid = [
            [],
            "object",
            True,
            {1: "key"},
            {"nested": {False: 1}},
            {"x": float("nan")},
            {"x": float("inf")},
            {"x": -float("inf")},
            {"x": 2**64},
            {"x": -(2**63) - 1},
            {"x": (1, 2)},
            {"x": b"bytes"},
            {"x": object()},
            {"x": "x" * packaging.MAX_METADATA_BYTES},
            {"x": "\x00" * (packaging.MAX_METADATA_BYTES // 2)},
            {"x": "λ" * packaging.MAX_METADATA_BYTES},
            {"x": deep},
            cycle,
            {"x": [None] * packaging.MAX_METADATA_NODES},
        ]
        original = (self.repo / "LICENSE").read_bytes()
        for index, metadata in enumerate(invalid):
            with self.subTest(case=index):
                parent = self.base / f"untouched-{index}"
                with self.assertRaises(ValueError):
                    packaging.assemble(
                        self.repo,
                        binary,
                        parent / "package",
                        "example.native",
                        "file_search",
                        "default",
                        metadata=metadata,
                    )
                self.assertFalse(parent.exists())
                self.assertEqual(binary.read_bytes(), b"native executable fixture")
                self.assertEqual((self.repo / "LICENSE").read_bytes(), original)

    def test_metadata_exact_size_depth_and_node_boundaries(self):
        exact = {"x": "a" * (packaging.MAX_METADATA_BYTES - 8)}
        self.assertEqual(packaging.validate_component_metadata(exact), exact)
        with self.assertRaisesRegex(ValueError, "UTF-8 bytes"):
            packaging.validate_component_metadata({"x": exact["x"] + "a"})
        value = 1
        for _ in range(packaging.MAX_METADATA_DEPTH - 1):
            value = [value]
        self.assertEqual(
            packaging.validate_component_metadata({"x": value}), {"x": value}
        )
        with self.assertRaisesRegex(ValueError, "depth"):
            packaging.validate_component_metadata({"x": [value]})
        nodes = {"x": [None] * (packaging.MAX_METADATA_NODES - 2)}
        self.assertEqual(packaging.validate_component_metadata(nodes), nodes)
        with self.assertRaisesRegex(ValueError, "node count"):
            packaging.validate_component_metadata({"x": [*nodes["x"], None]})

    def test_metadata_cli_round_trip_and_duplicate_rejection_before_output(self):
        binary = self.base / "native-plugin"
        binary.write_bytes(b"native executable fixture")
        cases = [
            ["--metadata", '{"preparing_cancel_receipt":1}'],
            ["--metadata", '{"x":1,"x":2}'],
            ["--metadata", '{"nested":{"x":1,"\\u0078":2}}'],
            ["--metadata", '{"x":NaN}'],
            ["--metadata", '{"x":1e309}'],
            ["--metadata", "null"],
            ["--metadata", "[]"],
            ["--metadata", "{}", "--metadata", "{}"],
            ["--metadata", " " * (packaging.MAX_METADATA_BYTES + 1)],
        ]
        for index, flags in enumerate(cases):
            output = self.base / f"metadata-cli-{index}"
            argv = [
                str(SCRIPT),
                "assemble",
                "--repo",
                str(self.repo),
                "--binary",
                str(binary),
                "--output",
                str(output),
                "--id",
                "example.native",
                "--kind",
                "file_search",
                "--version",
                "0.2.0",
                *flags,
            ]
            with (
                self.subTest(case=index),
                patch("sys.argv", argv),
                contextlib.redirect_stdout(io.StringIO()),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                if index == 0:
                    packaging.main()
                    manifest = json.loads((output / "codex-component.json").read_text())
                    self.assertEqual(
                        manifest["components"][0]["metadata"],
                        {"preparing_cancel_receipt": 1},
                    )
                else:
                    with self.assertRaises(SystemExit) as failure:
                        packaging.main()
                    self.assertEqual(failure.exception.code, 2)
                    self.assertFalse(output.exists())

    def test_worker_plan_preserves_default_and_explicit_declaration_without_building(
        self,
    ):
        worker = SCRIPT.parent / "tests/file_search/build_worker.py"
        sdk = self.repo / "component-sdk"
        sdk.mkdir()
        shutil.copy2(SCRIPT, sdk / SCRIPT.name)
        cli = self.base / "frozen-cli"
        cli.write_bytes(b"host fixture")
        manager = self.base / "frozen-manager"
        manager.write_bytes(b"manager fixture")
        cases = [
            ([], 0, "0.1.0", {}),
            (
                [
                    "--package-version",
                    "0.2.0",
                    "--component-metadata",
                    '{"preparing_cancel_receipt":1}',
                ],
                0,
                "0.2.0",
                {"preparing_cancel_receipt": 1},
            ),
            (["--component-metadata", '{"x":1,"x":2}'], 2, None, None),
            (
                ["--component-metadata", "{}", "--component-metadata", "{}"],
                2,
                None,
                None,
            ),
            (["--component-metadata", '{"x":1e309}'], 2, None, None),
            (["--package-version", "not-semver"], 2, None, None),
        ]
        for index, (flags, status, version, metadata) in enumerate(cases):
            work = self.base / f"worker-work-{index}"
            target = self.base / f"worker-target-{index}"
            # Invalid declarations also fail before mutation without --plan.
            args = [
                sys.executable,
                str(worker),
                "--repo",
                str(self.repo),
                "--cli",
                str(cli),
                "--manager",
                str(manager),
                "--work-dir",
                str(work),
                "--target-dir",
                str(target),
                *flags,
            ]
            if status == 0:
                args.append("--plan")
            with self.subTest(case=index):
                result = subprocess.run(
                    args, capture_output=True, text=True, timeout=10, check=False
                )
                self.assertEqual(result.returncode, status, result.stderr)
                self.assertFalse(work.exists())
                self.assertFalse(target.exists())
                if status == 0:
                    plan = json.loads(result.stdout)
                    self.assertEqual(
                        plan["package_declaration"],
                        {
                            "version": version,
                            "contract_version": 1,
                            "metadata": metadata,
                            "capabilities_are_declared_not_runtime_proof": True,
                        },
                    )
                    self.assertEqual(
                        plan["policy"]["minimum_target_free_bytes"], 2 * 1024**3
                    )
                    self.assertEqual(
                        plan["policy"]["minimum_cgroup_memory_headroom_bytes"],
                        768 * 1024**2,
                    )
                self.assertEqual(cli.read_bytes(), b"host fixture")
                self.assertEqual(manager.read_bytes(), b"manager fixture")


if __name__ == "__main__":
    unittest.main()
