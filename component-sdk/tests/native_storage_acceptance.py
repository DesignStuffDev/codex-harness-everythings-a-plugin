"""Build storage outside the checkout, remove its build tree, then exercise the CLI.

Run with the repository's Rust verification environment already sourced. Third-party
caches may be shared; every local Cargo package must resolve inside the export.
The host executables must already exist and are never built by this script.
Bundled MPL-covered vendor source intentionally remains in the installed package.
"""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import tomllib

from harness_acceptance import fingerprint


def lock_identities(path):
    return {
        (p["name"], p["version"], p.get("source"), p.get("checksum"))
        for p in tomllib.loads(path.read_text())["package"]
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--codex", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True)
    parser.add_argument("--contract-version", type=int, required=True)
    parser.add_argument("--package-version", default="0.1.0")
    args = parser.parse_args()
    repo = args.repo.resolve(strict=True)
    host, codex = args.host.resolve(strict=True), args.codex.resolve(strict=True)
    work = args.work_dir.resolve()
    if work.exists() or work == repo or repo in work.parents:
        parser.error("work directory must be new and outside the source checkout")
    work.mkdir(parents=True)
    source = work / "source"
    workspace = source / "codex-rs"
    package = work / "package"
    exporter = repo / "component-sdk/rust_component_package.py"
    report = {
        "passed": False,
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "contract_version": args.contract_version,
        "package_version": args.package_version,
        "commands": [],
        "binaries_before": {"host": fingerprint(host), "codex": fingerprint(codex)},
        "artifact_directory": str(work),
    }

    def run(label, argv, *, cwd=repo, environment=None):
        out, err = work / f"{label}.stdout", work / f"{label}.stderr"
        command = {"argv": list(map(str, argv)), "cwd": str(cwd)}
        report["commands"].append(command)
        started = time.monotonic()
        with out.open("w") as stdout, err.open("w") as stderr:
            result = subprocess.run(
                command["argv"],
                cwd=cwd,
                env=environment,
                stdin=subprocess.DEVNULL,
                stdout=stdout,
                stderr=stderr,
            )
        command.update(
            status=result.returncode,
            elapsed_seconds=time.monotonic() - started,
            stdout=str(out),
            stderr=str(err),
        )
        if result.returncode:
            raise RuntimeError(f"{label} failed ({result.returncode}); see {err}")
        return out

    try:
        run(
            "export",
            [
                sys.executable,
                exporter,
                "export",
                "--repo",
                repo,
                "--package",
                "codex-thread-store-local-plugin",
                "--output",
                source,
            ],
        )
        shutil.copy2(
            source / "COMPONENT_SOURCE_EXPORT.json", work / "source-inventory.json"
        )
        exported = json.loads((work / "source-inventory.json").read_text())
        before = lock_identities(workspace / "Cargo.lock")
        compiler = run("compiler", ["rustc", "-vV"], cwd=workspace).read_text()
        target = next(
            line.removeprefix("host: ")
            for line in compiler.splitlines()
            if line.startswith("host: ")
        )
        report["build_target"] = target
        metadata = json.loads(
            run(
                "metadata",
                [
                    "cargo",
                    "metadata",
                    "--offline",
                    "--filter-platform",
                    target,
                    "--format-version",
                    "1",
                ],
                cwd=workspace,
            ).read_text()
        )
        after = lock_identities(workspace / "Cargo.lock")
        assert not after - before, "export resolution changed dependency identities"
        local = [p for p in metadata["packages"] if p["source"] is None]
        assert all(source in Path(p["manifest_path"]).resolve().parents for p in local)
        excluded = {
            "codex-core",
            "codex-cli",
            "codex-tui",
            "codex-app-server",
            "core_test_support",
        }
        assert not excluded.intersection(p["name"] for p in metadata["packages"])
        report["source_export"] = {
            "local_crates": len(exported["local_crates"]),
            "resolved_packages": len(metadata["packages"]),
            "all_local_paths_inside_export": True,
            "lock_identities_before": len(before),
            "lock_identities_after": len(after),
            "lock_new_identities": 0,
            "excluded_engines": sorted(excluded),
            "inventory": fingerprint(work / "source-inventory.json"),
            "warnings": exported["warnings"],
        }
        build_env = dict(os.environ, CARGO_TARGET_DIR=str(args.target_dir.resolve()))
        build = run(
            "build",
            [
                "cargo",
                "build",
                "--offline",
                "--locked",
                "-j",
                "1",
                "-p",
                "codex-thread-store-local-plugin",
                "--bin",
                "codex-thread-store-local-plugin",
                "--message-format=json-render-diagnostics",
            ],
            cwd=workspace,
            environment=build_env,
        )
        artifacts = [
            json.loads(line)
            for line in build.read_text().splitlines()
            if line.startswith("{")
        ]
        artifact = next(
            a
            for a in artifacts
            if a.get("reason") == "compiler-artifact"
            and a.get("executable")
            and a["target"]["name"] == "codex-thread-store-local-plugin"
        )
        assert source in Path(artifact["target"]["src_path"]).resolve().parents
        binary = Path(artifact["executable"]).resolve(strict=True)
        report["build_artifact"] = {
            "package_id": artifact["package_id"],
            "target_source": artifact["target"]["src_path"],
            "fresh": artifact["fresh"],
            "fingerprint": fingerprint(binary),
            "shared_target": str(args.target_dir.resolve()),
            "third_party_cache_reused": any(
                a.get("reason") == "compiler-artifact"
                and a.get("fresh")
                and a.get("package_id", "").startswith(("registry+", "git+"))
                for a in artifacts
            ),
        }
        run(
            "assemble",
            [
                sys.executable,
                exporter,
                "assemble",
                "--repo",
                source,
                "--binary",
                binary,
                "--output",
                package,
                "--id",
                "native.thread-store-local",
                "--kind",
                "thread_store",
                "--name",
                "default",
                "--contract-version",
                args.contract_version,
                "--version",
                args.package_version,
            ],
        )
        run("strip", ["strip", "--strip-unneeded", package / binary.name])
        libraries = run("libraries", ["ldd", package / binary.name]).read_text()
        assert "not found" not in libraries, (
            "native package has unresolved runtime libraries"
        )
        report["runtime_libraries"] = libraries
        report["package_files"] = {
            p.relative_to(package).as_posix(): fingerprint(p)
            for p in sorted(package.rglob("*")) if p.is_file()
        }
        # This directory was created exclusively by this run; keep its inventory.
        shutil.rmtree(source)
        assert not source.exists()
        report["source_removed_before_install"] = True
        report["source_removal_scope"] = (
            "Exported build tree removed; any bundled MPL-covered vendor source intentionally travels with the package."
        )
        run(
            "runtime",
            [
                sys.executable,
                repo / "component-sdk/tests/harness_acceptance.py",
                "--host",
                host,
                "--codex",
                codex,
                "--work-dir",
                work / "runtime",
                "--thread-store-package",
                package,
            ],
        )
        report["runtime_report"] = str(work / "runtime/acceptance.json")
        assert json.loads(Path(report["runtime_report"]).read_text())["passed"] is True
        report["binaries_after"] = {
            "host": fingerprint(host),
            "codex": fingerprint(codex),
        }
        assert report["binaries_before"] == report["binaries_after"], (
            "host changed during acceptance"
        )
        report["passed"] = True
    finally:
        report["binaries_after"] = {
            "host": fingerprint(host),
            "codex": fingerprint(codex),
        }
        (work / "independent-report.json").write_text(
            json.dumps(report, indent=2) + "\n"
        )
    print(work / "independent-report.json")


if __name__ == "__main__":
    main()
