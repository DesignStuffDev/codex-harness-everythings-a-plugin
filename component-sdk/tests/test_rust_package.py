"""Exercise source closure/export without compiling or touching the Rust workspace."""

import importlib.util
import json
from pathlib import Path
import tempfile
import tomllib
import unittest

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


if __name__ == "__main__":
    unittest.main()
