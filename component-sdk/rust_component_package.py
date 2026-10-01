"""Plan/export a native component's production source closure, or assemble its binary.

Requires Python 3.11+ (tomllib). Does not build Rust or vendor registry/git sources.
"""

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
from pathlib import PurePosixPath
from pathlib import PureWindowsPath
import re
import shutil
import tempfile
import tomllib

EXCLUDED = {".git", "target", "__pycache__"}
DEPENDENCIES = ("dependencies", "build-dependencies")
LITERAL_INCLUDE = re.compile(
    r'(?:include_str|include_bytes|include|include_dir)!\s*\(\s*"([^"\n]+)"'
)
MANIFEST_INCLUDE = re.compile(
    r'(?:include_str|include_bytes|include|include_dir)!\s*\(\s*concat!\s*\(\s*env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*"([^"\n]+)"'
)


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for data in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(data)
    return value.hexdigest()


def toml_key(value):
    return value if re.fullmatch(r"[A-Za-z0-9_-]+", value) else json.dumps(value)


def toml_value(value):
    if isinstance(value, dict):
        return (
            "{ "
            + ", ".join(
                toml_key(key) + " = " + toml_value(item) for key, item in value.items()
            )
            + " }"
        )
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(item) for item in value) + "]"
    if isinstance(value, (str, bool, int, float)):
        return json.dumps(value, ensure_ascii=False, allow_nan=False)
    raise ValueError(f"unsupported Cargo TOML value type: {type(value).__name__}")


def dump_toml(document):
    """Serialize Cargo's TOML data model without changing dependency semantics."""
    lines = []

    def emit(table, path):
        for name, value in table.items():
            if not isinstance(value, dict):
                lines.append(toml_key(name) + " = " + toml_value(value))
        for name, value in table.items():
            if isinstance(value, dict):
                child = path + [name]
                lines.extend(["", "[" + ".".join(map(toml_key, child)) + "]"])
                emit(value, child)

    emit(document, [])
    result = "\n".join(lines) + "\n"
    if tomllib.loads(result) != document:
        raise ValueError("Cargo manifest serialization changed its data")
    return result


def production_manifest(document):
    value = copy.deepcopy(document)
    value.pop("dev-dependencies", None)
    for target in value.get("target", {}).values():
        target.pop("dev-dependencies", None)
    return value


def dependency_groups(document):
    for group in [document, *document.get("target", {}).values()]:
        for name in DEPENDENCIES:
            yield group.get(name, {})


class ExportPlan:
    """A conservative all-target/all-optional closure of one virtual workspace."""

    def __init__(self, repo, workspace, package, resources=()):
        self.repo = repo.resolve(strict=True)
        self.workspace = self.source_path(self.repo / workspace)
        self.workspace.relative_to(self.repo)
        self.root = tomllib.loads((self.workspace / "Cargo.toml").read_text())
        if "package" in self.root or "workspace" not in self.root:
            raise ValueError("export currently requires a virtual Cargo workspace")
        manifests = {}
        for member in self.root["workspace"]["members"]:
            for path in self.workspace.glob(member):
                if (path / "Cargo.toml").is_file():
                    value = tomllib.loads((path / "Cargo.toml").read_text())
                    manifests[value["package"]["name"]] = self.source_path(path)
        if package not in manifests:
            raise ValueError(f"unknown workspace package: {package}")
        self.package = package
        self.entry = manifests[package]
        self.crates = {}
        self.workspace_owners = {}
        self.nested_workspaces = {}
        self.workspace_owner_retentions = {}
        self.aliases = set()
        self.edges = []
        self.files = {}
        self.warnings = []
        pending = [self.entry]
        overrides = [*self.root.get("patch", {}).values(), self.root.get("replace", {})]
        for source in overrides:
            for value in source.values():
                if isinstance(value, dict) and "path" in value:
                    if Path(value["path"]).is_absolute():
                        raise ValueError("absolute Cargo override path is not portable")
                    pending.append(self.workspace / value["path"])
        while pending:
            path = self.source_path(pending.pop())
            path.relative_to(self.workspace)
            if path in self.crates:
                continue
            document = production_manifest(
                tomllib.loads((path / "Cargo.toml").read_text())
            )
            owner = self.workspace_owner(path)
            self.workspace_owners[path] = owner
            if owner != self.workspace:
                self.validate_nested_manifest(path, document)
                # A member can be selected directly or by a root path patch.
                # Retain its declaring package/workspace and dependencies even
                # when that package was not itself a dependency of the entry.
                pending.append(owner)
                if path != owner:
                    self.workspace_owner_retentions.setdefault(owner, set()).add(path)
            self.crates[path] = document
            for group in dependency_groups(document):
                for alias, original in group.items():
                    value, base = original, path
                    if isinstance(value, dict) and value.get("workspace"):
                        if owner != self.workspace:
                            raise ValueError(
                                f"nested inherited dependency is unsupported: {path}: {alias}"
                            )
                        self.aliases.add(alias)
                        value = self.root["workspace"]["dependencies"][alias]
                        base = self.workspace
                    if isinstance(value, dict) and "path" in value:
                        if Path(value["path"]).is_absolute():
                            raise ValueError(
                                f"absolute dependency path is not portable: {alias}"
                            )
                        destination = self.source_path(base / value["path"])
                        destination.relative_to(self.workspace)
                        pending.append(destination)
                        self.edges.append(
                            {
                                "from": document["package"]["name"],
                                "alias": alias,
                                "to": str(destination.relative_to(self.workspace)),
                                "optional": bool(
                                    isinstance(original, dict)
                                    and original.get("optional")
                                ),
                            }
                        )
        self.export_root = copy.deepcopy(self.root)
        target = self.export_root["workspace"]
        target["members"] = sorted(
            str(path.relative_to(self.workspace))
            for path in self.crates
            if self.workspace_owners[path] == self.workspace
        )
        target["default-members"] = [str(self.entry.relative_to(self.workspace))]
        excludes = set(target.get("exclude", []))
        excludes.update(
            str(path.relative_to(self.workspace)) for path in self.nested_workspaces
        )
        if excludes:
            target["exclude"] = sorted(excludes)
        target["dependencies"] = {
            alias: self.root["workspace"]["dependencies"][alias]
            for alias in sorted(self.aliases)
        }
        for owner in self.nested_workspaces:
            if owner not in self.crates:
                raise ValueError(f"nested workspace root was not retained: {owner}")
            nested = self.crates[owner]["workspace"]
            selected = {
                path for path in self.crates if self.workspace_owners[path] == owner
            }
            members = sorted(
                str(path.relative_to(owner)) for path in selected if path != owner
            )
            nested["members"] = members
            if "default-members" in nested:
                defaults = set()
                for pattern in nested["default-members"]:
                    if Path(pattern).is_absolute() or ".." in Path(pattern).parts:
                        raise ValueError(
                            f"nested workspace default member is not portable: {pattern}"
                        )
                    matches = [owner] if pattern == "." else owner.glob(pattern)
                    defaults.update(
                        str(path.relative_to(owner))
                        for path in matches
                        if path in selected
                    )
                if defaults:
                    nested["default-members"] = sorted(defaults)
                else:
                    nested.pop("default-members")
        for path in self.crates:
            self.add_tree(path)
        # Keep root and workspace Cargo configuration and toolchain resolution.
        for base in dict.fromkeys([self.repo, self.workspace]):
            for name in (
                "LICENSE",
                "NOTICE",
                "README.md",
                "UPSTREAM_PROVENANCE.md",
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain",
                "rust-toolchain.toml",
                ".cargo/config",
                ".cargo/config.toml",
            ):
                if (base / name).is_file():
                    self.add_file(base / name)
        for resource in resources:
            self.add_tree(self.source_path(self.repo / resource))
        self.audit_includes()

    def workspace_owner(self, path):
        """Find a local package's declared workspace without absorbing vendor roots."""
        owners = []
        current = path
        while current != self.workspace:
            manifest = current / "Cargo.toml"
            if manifest.is_file():
                document = tomllib.loads(manifest.read_text())
                if "workspace" in document:
                    owners.append((current, document))
            current = current.parent
        if len(owners) > 1:
            raise ValueError(
                f"multiple nested workspace levels are unsupported: {path}"
            )
        if not owners:
            return self.workspace
        owner, document = owners[0]
        if "package" not in document:
            raise ValueError(f"virtual nested workspaces are unsupported: {owner}")
        if document["workspace"].get("exclude"):
            raise ValueError(
                f"nested workspace exclusions require explicit ownership resolution: {owner}"
            )
        self.nested_workspaces[owner] = document
        return owner

    def validate_nested_manifest(self, path, document):
        """Only self-contained nested manifests are supported; never inherit outer values."""
        if "workspace" in document["package"] or any(
            isinstance(value, dict) and value.get("workspace")
            for value in document["package"].values()
        ):
            raise ValueError(
                f"nested inherited package settings are unsupported: {path}"
            )
        if document.get("lints", {}).get("workspace"):
            raise ValueError(f"nested inherited lint settings are unsupported: {path}")

    def source_path(self, path, strict=True):
        for ancestor in (path, *path.parents):
            if ancestor.is_symlink():
                raise ValueError(
                    f"source symlinks require explicit materialization: {ancestor}"
                )
            if ancestor == self.repo:
                break
        resolved = path.resolve(strict=strict)
        resolved.relative_to(self.repo)
        return resolved

    def add_file(self, path):
        relative = self.source_path(path).relative_to(self.repo)
        self.files[relative] = path

    def add_tree(self, directory):
        directory = self.source_path(directory)
        if directory.is_file():
            self.add_file(directory)
            return
        directory.relative_to(self.repo)
        if directory in (self.repo, self.workspace):
            raise ValueError("resource cannot export the entire repository/workspace")
        for current, names, filenames in os.walk(directory, followlinks=False):
            current = Path(current)
            for name in list(names):
                child = current / name
                if name in EXCLUDED or (
                    (child / "Cargo.toml").is_file()
                    and child.resolve() not in self.crates
                ):
                    names.remove(name)
                elif child.is_symlink():
                    raise ValueError(f"source directory symlink: {child}")
            for name in filenames:
                self.add_file(current / name)

    def audit_includes(self):
        """Copy direct include resources; report nonliteral/generated reads honestly."""
        inspected = set()
        while True:
            pending = [
                path
                for path in self.files.values()
                if path.suffix == ".rs" and path not in inspected
            ]
            if not pending:
                break
            for path in pending:
                inspected.add(path)
                text = path.read_text()
                owner = next(
                    (
                        crate
                        for crate in sorted(
                            self.crates, key=lambda p: len(p.parts), reverse=True
                        )
                        if crate == path.parent or crate in path.parents
                    ),
                    None,
                )
                paths = [
                    (path.parent, value) for value in LITERAL_INCLUDE.findall(text)
                ]
                if owner:
                    paths.extend(
                        (owner, value.lstrip("/"))
                        for value in MANIFEST_INCLUDE.findall(text)
                    )
                for base, resource in paths:
                    if "$CARGO_MANIFEST_DIR" in resource and owner:
                        base, resource = (
                            owner,
                            resource.replace("$CARGO_MANIFEST_DIR", "").lstrip("/"),
                        )
                    if "$" in resource:
                        self.warnings.append(
                            f"dynamic include: {path.relative_to(self.repo)}: {resource}"
                        )
                        continue
                    resolved = self.source_path(base / resource, strict=False)
                    if resolved.exists():
                        self.add_tree(resolved)
                    else:
                        self.warnings.append(
                            f"missing/generated include: {path.relative_to(self.repo)}: {resource}"
                        )
        self.warnings.append(
            "Arbitrary build-script/runtime file reads require separate build verification; literal include scanning is not a compiler."
        )

    def describe(self):
        return {
            "package": self.package,
            "workspace": str(self.workspace.relative_to(self.repo)),
            "local_crates": [
                {
                    "name": value["package"]["name"],
                    "path": str(path.relative_to(self.repo)),
                }
                for path, value in sorted(self.crates.items())
            ],
            "local_edges": sorted(
                self.edges, key=lambda edge: (edge["from"], edge["alias"])
            ),
            "files": len(self.files),
            "source_bytes": sum(path.stat().st_size for path in self.files.values()),
            "dev_dependencies_removed": True,
            "all_targets_and_optional_dependencies_included": True,
            "nested_workspaces": [
                str(path.relative_to(self.workspace))
                for path in sorted(self.nested_workspaces)
            ],
            "nested_workspace_owner_retention": [
                {
                    "path": str(owner.relative_to(self.repo)),
                    "selected_members": [
                        str(path.relative_to(self.repo)) for path in sorted(members)
                    ],
                    "reason": "Retain the declaring workspace package and its source closure; this is not a dependency edge from the export entry.",
                }
                for owner, members in sorted(self.workspace_owner_retentions.items())
            ],
            "warnings": sorted(set(self.warnings)),
            "independent_build_verified": False,
        }

    def export(self, output):
        output = output.resolve()
        if output.exists() or self.repo == output or self.repo in output.parents:
            raise ValueError(
                "output must be a new directory outside the source repository"
            )
        output.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(
            prefix=".component-export-", dir=output.parent
        ) as temporary:
            temporary = Path(temporary)
            inventory = []
            for relative, source in sorted(self.files.items()):
                destination = temporary / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)
                inventory.append(
                    {"path": str(relative), "source_sha256": digest(source)}
                )
            workspace = temporary / self.workspace.relative_to(self.repo)
            (workspace / "Cargo.toml").write_text(dump_toml(self.export_root))
            for crate, document in self.crates.items():
                manifest = temporary / crate.relative_to(self.repo) / "Cargo.toml"
                # Preserve upstream manifest bytes unless export actually changes
                # its dependency/workspace metadata (for example pruning tests).
                if tomllib.loads(manifest.read_text()) != document:
                    manifest.write_text(dump_toml(document))
            for entry in inventory:
                entry["export_sha256"] = digest(temporary / entry["path"])
            report = self.describe()
            report["source_hashes"] = inventory
            report["build_command"] = [
                "cargo",
                "build",
                "--offline",
                "--manifest-path",
                str(self.workspace.relative_to(self.repo) / "Cargo.toml"),
                "-p",
                self.package,
            ]
            report["registry_and_git_sources_vendored"] = False
            (temporary / "COMPONENT_SOURCE_EXPORT.json").write_text(
                json.dumps(report, indent=2) + "\n"
            )
            # Rename the staged directory atomically; TemporaryDirectory cleanup then becomes a no-op.
            temporary.rename(output)


def assemble(
    repo, binary, output, plugin_id, kind, name, *, contract_version=1, version="0.1.0"
):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", plugin_id):
        raise ValueError("invalid plugin ID")
    if not kind or not name or len(kind.encode()) > 128 or len(name.encode()) > 128:
        raise ValueError("component kind/name must contain 1–128 UTF-8 bytes")
    if type(contract_version) is not int or not 1 <= contract_version <= 0xFFFFFFFF:
        raise ValueError("component contract version must be a positive 32-bit integer")
    semantic_version = (
        re.fullmatch(
            r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
            r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
            r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?",
            version,
        )
        if isinstance(version, str)
        else None
    )
    # Match the host's semver parser: numeric prerelease identifiers cannot have
    # leading zeroes, and its three numeric version fields fit in u64.
    if (
        semantic_version is None
        or any(int(part) > 0xFFFFFFFFFFFFFFFF for part in semantic_version.groups()[:3])
        or any(
            part.isdigit() and len(part) > 1 and part.startswith("0")
            for part in (semantic_version.group(4) or "").split(".")
        )
    ):
        raise ValueError("package version must be semantic versioning")
    if not binary.is_file():
        raise ValueError("binary must be a regular file")
    covered_source = nucleo_covered_source(repo)
    output.mkdir(parents=True, exist_ok=False)
    shutil.copy2(binary, output / binary.name)
    for notice in ("LICENSE", "NOTICE"):
        shutil.copyfile(repo / notice, output / notice)
    if covered_source:
        for entry in covered_source["files"]:
            source = repo / entry["source_path"]
            destination = output / entry["package_path"]
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
            entry["package_sha256"] = digest(destination)
            if entry["package_sha256"] != entry["export_sha256"]:
                raise ValueError(f"covered source changed during assembly: {source}")
        (output / "THIRD_PARTY_NOTICES.json").write_text(
            json.dumps(covered_source, indent=2) + "\n"
        )
    manifest = {
        "api_version": 1,
        "id": plugin_id,
        "version": version,
        "entrypoint": binary.name,
        "args": [],
        "dependencies": {},
        "components": [
            {
                "kind": kind,
                "name": name,
                "contract_version": contract_version,
                "metadata": {},
            }
        ],
    }
    (output / "codex-component.json").write_text(json.dumps(manifest, indent=2) + "\n")


def inventory_path(value):
    """Accept portable relative inventory paths, including Windows-produced exports."""
    if not isinstance(value, str) or not value or PureWindowsPath(value).drive:
        raise ValueError("source inventory path must be relative")
    normalized = value.replace("\\", "/")
    path = PurePosixPath(normalized)
    if path.is_absolute() or any(
        part in ("", ".", "..") for part in normalized.split("/")
    ):
        raise ValueError(f"unsafe source inventory path: {value}")
    return path


def nucleo_covered_source(repo):
    """Attest and bundle only this harness's inventoried, modified MPL dependency."""
    inventory_file = repo / "COMPONENT_SOURCE_EXPORT.json"
    if not inventory_file.is_file():
        if any(repo.glob("*/third-party/nucleo/Cargo.toml")):
            raise ValueError(
                "Nucleo source assembly requires a source export inventory"
            )
        return None
    if inventory_file.is_symlink():
        raise ValueError("source export inventory cannot be a symlink")
    inventory = json.loads(inventory_file.read_text())
    workspace = inventory_path(inventory["workspace"])
    prefix = workspace / "third-party/nucleo"
    entries = {}
    for entry in inventory["source_hashes"]:
        path = inventory_path(entry["path"])
        if path in entries:
            raise ValueError(f"duplicate source inventory path: {path}")
        entries[path] = entry
    selected = {
        path: entry for path, entry in entries.items() if prefix in path.parents
    }
    local = {
        item["name"]: inventory_path(item["path"]) for item in inventory["local_crates"]
    }
    if not selected:
        if (
            any(name in local for name in ("nucleo", "nucleo-matcher"))
            or (repo / prefix).exists()
        ):
            raise ValueError("Nucleo is present without inventoried covered source")
        return None
    if (
        local.get("nucleo") != prefix
        or local.get("nucleo-matcher") != prefix / "matcher"
    ):
        raise ValueError("Nucleo source inventory does not match its local crate paths")
    required = (
        "Cargo.toml",
        "LICENSE",
        "README.md",
        "PROVENANCE.md",
        "src/lib.rs",
        "matcher/Cargo.toml",
        "matcher/LICENSE",
        "matcher/src/lib.rs",
    )
    if any(prefix / name not in selected for name in required):
        raise ValueError("Nucleo covered source inventory is incomplete")
    for current in (repo / prefix).rglob("*"):
        if current.is_symlink():
            raise ValueError(f"covered source symlinks are unsupported: {current}")
        if (
            current.is_file()
            and PurePosixPath(current.relative_to(repo).as_posix()) not in selected
        ):
            raise ValueError(f"covered source file is absent from inventory: {current}")
    files = []
    for path, entry in sorted(selected.items()):
        source = repo / path
        current = source
        while current != repo:
            if current.is_symlink():
                raise ValueError(f"covered source symlinks are unsupported: {path}")
            current = current.parent
        if not source.is_file():
            raise ValueError(f"covered source file is missing: {path}")
        if any(
            not isinstance(entry.get(key), str)
            or not re.fullmatch(r"[a-f0-9]{64}", entry[key])
            for key in ("source_sha256", "export_sha256")
        ):
            raise ValueError(f"invalid covered source hash: {path}")
        if (
            entry["source_sha256"] != entry["export_sha256"]
            and path.name != "Cargo.toml"
        ):
            raise ValueError(
                f"non-manifest covered source changed during export: {path}"
            )
        if digest(source) != entry["export_sha256"]:
            raise ValueError(f"covered source hash does not match export: {path}")
        files.append(
            {
                "source_path": str(path),
                "package_path": str(
                    PurePosixPath("third-party/nucleo") / path.relative_to(prefix)
                ),
                "source_sha256": entry["source_sha256"],
                "export_sha256": entry["export_sha256"],
            }
        )
    pin = "4253de9faabb4e5c6d81d946a5e35a90f87347ee"
    upstream = "https://github.com/helix-editor/nucleo"
    provenance = (repo / prefix / "PROVENANCE.md").read_text()
    if pin not in provenance or upstream not in provenance:
        raise ValueError(
            "Nucleo provenance does not identify the supported upstream pin"
        )
    components = []
    for directory, name, version in (
        ("", "nucleo", "0.5.0"),
        ("matcher", "nucleo-matcher", "0.3.1"),
    ):
        package = tomllib.loads((repo / prefix / directory / "Cargo.toml").read_text())[
            "package"
        ]
        if (package["name"], package["version"], package["license"]) != (
            name,
            version,
            "MPL-2.0",
        ):
            raise ValueError(
                f"Nucleo package identity or MPL license does not match: {directory}"
            )
        components.append(
            {
                "name": name,
                "version": version,
                "license": "MPL-2.0",
                "upstream_repository": upstream,
                "upstream_revision": pin,
                "covered_source": str(PurePosixPath("third-party/nucleo") / directory),
                "license_file": str(
                    PurePosixPath("third-party/nucleo") / directory / "LICENSE"
                ),
                "provenance_file": "third-party/nucleo/PROVENANCE.md",
            }
        )
    return {
        "schema_version": 1,
        "scope": "Targeted Nucleo covered-source bundle; not a complete transitive-license compliance report.",
        "source_export_inventory_sha256": digest(inventory_file),
        "components": components,
        "files": files,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("plan", "export"):
        sub = commands.add_parser(command)
        sub.add_argument("--repo", type=Path, required=True)
        sub.add_argument("--workspace", default="codex-rs")
        sub.add_argument("--package", required=True)
        sub.add_argument("--resource", action="append", default=[])
        if command == "export":
            sub.add_argument("--output", type=Path, required=True)
    sub = commands.add_parser("assemble")
    sub.add_argument("--repo", type=Path, required=True)
    sub.add_argument("--binary", type=Path, required=True)
    sub.add_argument("--output", type=Path, required=True)
    sub.add_argument("--id", required=True)
    sub.add_argument("--kind", required=True)
    sub.add_argument("--name", default="default")
    sub.add_argument("--contract-version", type=int, default=1)
    sub.add_argument(
        "--version",
        default="0.1.0",
        help="Package semantic version (independent of the component contract)",
    )
    args = parser.parse_args()
    try:
        if args.command == "assemble":
            assemble(
                args.repo.resolve(strict=True),
                args.binary.resolve(strict=True),
                args.output.absolute(),
                args.id,
                args.kind,
                args.name,
                contract_version=args.contract_version,
                version=args.version,
            )
            print(args.output.absolute())
        else:
            plan = ExportPlan(args.repo, args.workspace, args.package, args.resource)
            if args.command == "export":
                plan.export(args.output)
                print(args.output.absolute())
            else:
                print(json.dumps(plan.describe(), indent=2))
    except (ValueError, OSError, KeyError) as error:
        parser.exit(2, f"native component packaging failed: {error}\n")


if __name__ == "__main__":
    main()
