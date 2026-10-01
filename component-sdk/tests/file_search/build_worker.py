#!/usr/bin/env python3
"""Export/build only the native search worker in a new external source/target.

--plan performs filesystem/resource inspection only. Actual builds are an
explicit separate invocation, serialized by the owning task, under the unchanged
subreaper runner. This script never builds the CLI or component manager.
"""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import runpy
import shutil
import sys
import tomllib

from acceptance_support import Runner, fingerprint, inventory, require, write_report
from lock_patch_normalization import (
    assert_package_records_unchanged,
    prepare_normalization,
)

WORKER = "codex-file-search-local-plugin"
PLUGIN = "native.file-search-local"
EXCLUDED_ENGINES = {
    "codex-core",
    "codex-cli",
    "codex-tui",
    "codex-app-server",
    "codex-file-search-runtime",
    "core_test_support",
}


def lock_identities(path):
    return {
        (p["name"], p["version"], p.get("source"), p.get("checksum"))
        for p in tomllib.loads(path.read_text())["package"]
    }


def validate_metadata(metadata, source, target):
    local = [package for package in metadata["packages"] if package["source"] is None]
    require(
        all(
            source in Path(package["manifest_path"]).resolve().parents
            for package in local
        ),
        "a local dependency resolves outside the source export",
    )
    require(
        not EXCLUDED_ENGINES.intersection(
            package["name"] for package in metadata["packages"]
        ),
        "worker unexpectedly depends on host/runtime/UI engine",
    )
    require(
        Path(metadata["target_directory"]).resolve() == target,
        "Cargo metadata did not use the independent target",
    )
    return local


def existing_parent(path):
    while not path.exists():
        path = path.parent
    return path


def resources(work, target):
    result = {"work_filesystem": {}, "target_filesystem": {}}
    for label, directory in (("work_filesystem", work), ("target_filesystem", target)):
        parent = existing_parent(directory)
        usage = shutil.disk_usage(parent)
        result[label] = {
            "path": str(directory),
            "device": parent.stat().st_dev,
            "free_bytes": usage.free,
            "total_bytes": usage.total,
        }
    for name in ("memory.current", "memory.max", "memory.events"):
        path = Path("/sys/fs/cgroup") / name
        if path.is_file():
            result[name] = path.read_text().strip()
    result["policy"] = {
        "minimum_target_free_bytes": 2 * 1024**3,
        "minimum_work_free_bytes": 256 * 1024**2,
        "minimum_cgroup_memory_headroom_bytes": 768 * 1024**2,
        "profile": "dev-small",
        "jobs": 1,
        "incremental": False,
        "fresh_target_required": True,
        "registry_toolchain_cache_reuse": True,
        "estimate_caveat": "Conservative admission threshold, not a measured peak guarantee.",
    }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--manager", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True)
    parser.add_argument("--plan", action="store_true")
    parser.add_argument("--package-version", default="0.1.0")
    parser.add_argument(
        "--component-metadata",
        action="append",
        default=[],
        help="Bounded inline component JSON object; declaration is not runtime proof",
    )
    args = parser.parse_args()
    repo = args.repo.resolve(strict=True)
    packager = runpy.run_path(str(repo / "component-sdk/rust_component_package.py"))
    if len(args.component_metadata) > 1:
        parser.error("component metadata may be supplied only once")
    try:
        version = packager["validate_package_version"](args.package_version)
        component_metadata = (
            packager["parse_component_metadata"](args.component_metadata[0])
            if args.component_metadata
            else {}
        )
    except ValueError as error:
        parser.error(str(error))
    declaration = {
        "version": version,
        "contract_version": 1,
        "metadata": component_metadata,
        "capabilities_are_declared_not_runtime_proof": True,
    }
    cli, manager = args.cli.resolve(strict=True), args.manager.resolve(strict=True)
    work, target = args.work_dir.resolve(), args.target_dir.resolve()
    for directory in (work, target):
        require(not directory.exists(), "source/build work and target must both be new")
        require(
            directory != repo and repo not in directory.parents,
            "source/build artifacts must be outside the harness checkout",
        )
    require(
        work != target and work not in target.parents and target not in work.parents,
        "work directory and independent target must be disjoint",
    )
    plan = resources(work, target)
    plan["package_declaration"] = declaration
    plan["commands"] = {
        "export": [
            sys.executable,
            str(repo / "component-sdk/rust_component_package.py"),
            "export",
            "--repo",
            str(repo),
            "--package",
            WORKER,
            "--output",
            str(work / "source"),
        ],
        "metadata": [
            "cargo",
            "metadata",
            "--offline",
            "--filter-platform",
            "<rustc-host-triple>",
            "--format-version",
            "1",
        ],
        "build": [
            "cargo",
            "build",
            "--offline",
            "--locked",
            "--profile",
            "dev-small",
            "-j",
            "1",
            "-p",
            WORKER,
            "--bin",
            WORKER,
            "--message-format=json-render-diagnostics",
        ],
        "build_cwd": str(work / "source/codex-rs"),
        "target_override": str(target),
    }
    if args.plan:
        print(json.dumps(plan, indent=2))
        return 0
    require(
        plan["target_filesystem"]["free_bytes"] >= 2 * 1024**3,
        "insufficient target filesystem headroom; owner must reclaim approved generated caches first",
    )
    require(
        plan["work_filesystem"]["free_bytes"] >= 256 * 1024**2,
        "insufficient work artifact filesystem headroom",
    )
    maximum = plan.get("memory.max", "max")
    if maximum.isdecimal():
        require(
            int(maximum) - int(plan["memory.current"]) >= 768 * 1024**2,
            "insufficient current cgroup headroom; serialize with other work",
        )
    work.mkdir(parents=True)
    target.mkdir(parents=True)
    os.chmod(work, 0o700)
    report_path = work / "independent-build.json"
    report = {
        "passed": False,
        "evidence_kind": "independent_source_build",
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "artifact_directory": str(work),
        "target_directory": str(target),
        "resources_before": plan,
        "package_declaration": declaration,
        "commands": [],
        "frozen_binaries_before": {
            "cli": fingerprint(cli),
            "manager": fingerprint(manager),
        },
        "tooling": {
            name: fingerprint(Path(__file__).with_name(name))
            for name in (
                "build_worker.py",
                "acceptance_support.py",
                "lock_patch_normalization.py",
            )
        },
        "exporter": fingerprint(repo / "component-sdk/rust_component_package.py"),
    }
    runner = Runner(work, report, report_path)
    source, workspace, package = (
        work / "source",
        work / "source/codex-rs",
        work / "package",
    )
    environment = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL="0")
    try:
        runner.run("export", plan["commands"]["export"], cwd=repo)
        shutil.copy2(
            source / "COMPONENT_SOURCE_EXPORT.json", work / "source-inventory.json"
        )
        exported = json.loads((work / "source-inventory.json").read_text())
        report["exported_source_inventory"] = fingerprint(
            work / "source-inventory.json"
        )
        before = lock_identities(workspace / "Cargo.lock")
        shutil.copy2(workspace / "Cargo.lock", work / "original-export.lock")
        report["original_export_lock"] = fingerprint(work / "original-export.lock")
        text, _, _ = runner.run(
            "compiler", ["rustc", "-vV"], cwd=workspace, environment=environment
        )
        triple = next(
            line.removeprefix("host: ")
            for line in text.splitlines()
            if line.startswith("host: ")
        )
        metadata_args = [
            "cargo",
            "metadata",
            "--offline",
            "--filter-platform",
            triple,
            "--format-version",
            "1",
        ]
        text, _, _ = runner.run(
            "metadata",
            metadata_args,
            cwd=workspace,
            environment=environment,
            timeout=600,
        )
        metadata = json.loads(text)
        after = lock_identities(workspace / "Cargo.lock")
        require(
            not after - before,
            "export dependency resolution introduced changed/new identities",
        )
        local = validate_metadata(metadata, source, target)
        # Restrict normalization to this isolated export. Original root patches
        # stay untouched; full-lock records protect optional/foreign-target deps.
        serializer = runpy.run_path(
            str(repo / "component-sdk/rust_component_package.py")
        )["dump_toml"]
        normalization, resolved_before = prepare_normalization(source, work, serializer)
        report["unused_patch_normalization"] = normalization
        write_report(report_path, report)
        if normalization["applied"]:
            text, _, _ = runner.run(
                "metadata-normalized",
                metadata_args,
                cwd=workspace,
                environment=environment,
                timeout=600,
            )
            normalized_metadata = json.loads(text)
            resolved_after = tomllib.loads((workspace / "Cargo.lock").read_text())
            assert_package_records_unchanged(resolved_before, resolved_after)
            require(
                not lock_identities(workspace / "Cargo.lock") - before,
                "normalized resolution introduced changed/new identities",
            )
            local = validate_metadata(normalized_metadata, source, target)
            for key in (
                "packages",
                "resolve",
                "workspace_members",
                "workspace_default_members",
            ):
                require(
                    metadata.get(key) == normalized_metadata.get(key),
                    f"unused-patch normalization changed metadata {key}",
                )
            metadata = normalized_metadata
            normalization["exact_full_package_records_unchanged"] = True
            normalization["local_paths_and_closure_revalidated"] = True
            normalization["lock_after"] = fingerprint(workspace / "Cargo.lock")
            shutil.copy2(
                source / "COMPONENT_SOURCE_EXPORT.json", work / "source-inventory.json"
            )
            report["exported_source_inventory"] = fingerprint(
                work / "source-inventory.json"
            )
        locked_bytes = (workspace / "Cargo.lock").read_bytes()
        report["source_resolution"] = {
            "host_triple": triple,
            "local_crates": len(local),
            "resolved_packages": len(metadata["packages"]),
            "all_local_paths_inside_export": True,
            "lock_new_identities": 0,
            "lock_identities_before": len(before),
            "lock_identities_after": len(after),
            "excluded_packages": sorted(EXCLUDED_ENGINES),
            "warnings": exported["warnings"],
            "resolved_lock": fingerprint(workspace / "Cargo.lock"),
        }
        text, _, _ = runner.run(
            "build",
            plan["commands"]["build"],
            cwd=workspace,
            environment=environment,
            timeout=3600,
        )
        require(
            (workspace / "Cargo.lock").read_bytes() == locked_bytes,
            "locked build changed the normalized lockfile",
        )
        report["final_locked_build_preserved_lock_bytes"] = True
        artifacts = [
            json.loads(line) for line in text.splitlines() if line.startswith("{")
        ]
        matching = [
            a
            for a in artifacts
            if a.get("reason") == "compiler-artifact"
            and a.get("executable")
            and a["target"]["name"] == WORKER
        ]
        require(len(matching) == 1, "worker artifact missing or ambiguous")
        artifact = matching[0]
        require(
            source in Path(artifact["target"]["src_path"]).resolve().parents,
            "worker artifact source is outside export",
        )
        require(
            artifact["fresh"] is False, "worker compilation reused a prior artifact"
        )
        binary = Path(artifact["executable"]).resolve(strict=True)
        require(target in binary.parents, "worker binary is outside independent target")
        report["build_artifact"] = {
            "binary": str(binary),
            "fingerprint": fingerprint(binary),
            "target_source": artifact["target"]["src_path"],
            "fresh": artifact["fresh"],
            "package_id": artifact["package_id"],
            "separate_initially_empty_target": True,
            "profile": "dev-small",
            "incremental": False,
        }
        runner.run(
            "assemble",
            [
                sys.executable,
                repo / "component-sdk/rust_component_package.py",
                "assemble",
                "--repo",
                source,
                "--binary",
                binary,
                "--output",
                package,
                "--id",
                PLUGIN,
                "--kind",
                "file_search",
                "--name",
                "default",
                "--contract-version",
                "1",
                "--version",
                version,
                "--metadata",
                json.dumps(
                    component_metadata,
                    ensure_ascii=False,
                    allow_nan=False,
                    separators=(",", ":"),
                ),
            ],
            cwd=repo,
        )
        libraries, _, _ = runner.run(
            "libraries", ["ldd", package / binary.name], cwd=work
        )
        require("not found" not in libraries, "worker has unresolved runtime libraries")
        report["package_files"] = inventory(package)
        require(
            "THIRD_PARTY_NOTICES.json" in report["package_files"],
            "MPL source notice missing",
        )
        report["package_path"] = str(package)
        # Preserve all work, but make original source paths unavailable for runtime.
        source.rename(work / "source.parked")
        report["source_parked_before_runtime"] = True
        report["source_preserved_at"] = str(work / "source.parked")
        report["resources_after"] = resources(work, target)
        report["passed"] = True
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        report["frozen_binaries_after"] = {
            "cli": fingerprint(cli),
            "manager": fingerprint(manager),
        }
        if report["frozen_binaries_before"] != report["frozen_binaries_after"]:
            report["passed"] = False
            report["error"] = "CLI or manager changed during independent build"
        write_report(report_path, report)
    print(str(report_path))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
