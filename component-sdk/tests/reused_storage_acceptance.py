"""Exercise an unchanged independently built storage package with new host binaries.

The input must be an original native_storage_acceptance.py build proof, not a
previous reuse report. This helper does not export or compile Rust. It checks
the old proof/package, runs the real harness acceptance with the supplied host
and CLI, and emits evidence_kind=reused_package_new_host. Its compatible
independent-report.json shape lets manual migration and GUI acceptance verify
the current candidate while source_build_report identifies the original build.
Original artifacts and reports are read only; no command from a report executes.
"""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path, PurePosixPath
import stat
import subprocess
import sys
import time

from harness_acceptance import SDK, fingerprint


def require(condition, message):
    if not condition:
        raise ValueError(message)


def checked_fingerprint(value):
    require(isinstance(value, dict), "missing fingerprint object")
    require(set(value) == {"sha256", "mtime_ns", "bytes"}, "invalid fingerprint fields")
    digest = value["sha256"]
    require(
        isinstance(digest, str)
        and len(digest) == 64
        and all(char in "0123456789abcdef" for char in digest),
        "invalid SHA-256 fingerprint",
    )
    for key in ("mtime_ns", "bytes"):
        require(type(value[key]) is int and value[key] >= 0, f"invalid {key}")


def package_fingerprints(package):
    """Reject symlinks/special files and fingerprint the complete regular-file set."""
    require(
        package.is_dir() and not package.is_symlink(),
        "package directory missing or linked",
    )
    result = {}
    for path in sorted(package.rglob("*")):
        mode = path.lstat().st_mode
        require(
            stat.S_ISDIR(mode) or stat.S_ISREG(mode),
            f"package contains a symlink or special file: {path.relative_to(package)}",
        )
        if stat.S_ISREG(mode):
            result[path.relative_to(package).as_posix()] = fingerprint(path)
    return result


def option(argv, name):
    require(argv.count(name) == 1, f"original command must specify {name} once")
    position = argv.index(name)
    require(position + 1 < len(argv), f"missing original {name} value")
    return argv[position + 1]


def original_proof(path):
    """Validate the original build record without requiring old host files to exist."""
    report = json.loads(path.read_text())
    require(
        report.get("passed") is True, "original independent acceptance did not pass"
    )
    require(report.get("contract_version") == 2, "original storage contract must be 2")
    require(
        report.get("evidence_kind") in (None, "independent_source_build")
        and "source_build_report" not in report,
        "expected original independent build proof, not nested reuse evidence",
    )
    require(
        report.get("source_removed_before_install") is True,
        "original proof must confirm source removal before install",
    )
    before = report["binaries_before"]
    require(set(before) == {"host", "codex"}, "original binary fingerprints missing")
    for value in before.values():
        checked_fingerprint(value)
    require(
        before == report["binaries_after"],
        "original binaries changed during acceptance",
    )

    directory = Path(report["artifact_directory"])
    require(directory.is_absolute(), "original artifact directory must be absolute")
    directory = directory.resolve(strict=True)
    source = directory / "source"
    require(not os.path.lexists(source), "original exported source tree still exists")
    package = directory / "package"
    require(
        package.is_dir() and not package.is_symlink(),
        "original package directory missing or linked",
    )
    expected = report["package_files"]
    require(
        isinstance(expected, dict) and expected, "original package inventory is empty"
    )
    for name, value in expected.items():
        relative = PurePosixPath(name)
        require(
            bool(name)
            and not relative.is_absolute()
            and name == relative.as_posix()
            and all(part not in (".", "..") for part in relative.parts)
            and "\\" not in name,
            f"invalid package inventory path: {name!r}",
        )
        checked_fingerprint(value)
    require(package_fingerprints(package) == expected, "original package files changed")
    manifest = json.loads((package / "codex-component.json").read_text())
    require(manifest["api_version"] == 1, "unsupported original component API")
    storage = [
        item
        for item in manifest["components"]
        if item["kind"] == "thread_store" and item["name"] == "default"
    ]
    require(
        len(storage) == 1 and storage[0]["contract_version"] == 2,
        "package is not storage contract 2",
    )
    require(
        manifest["entrypoint"] in expected, "package entrypoint is not fingerprinted"
    )
    require(
        os.access(package / manifest["entrypoint"], os.X_OK),
        "package entrypoint is not executable",
    )
    require(
        manifest["version"] == report["package_version"],
        "package version differs from proof",
    )

    exported = report["source_export"]
    require(
        exported["all_local_paths_inside_export"] is True,
        "original source paths escaped export",
    )
    require(
        exported["lock_new_identities"] == 0,
        "original build changed dependency identities",
    )
    require(exported["local_crates"] > 0, "original local source inventory missing")
    inventory = directory / "source-inventory.json"
    require(
        fingerprint(inventory) == exported["inventory"],
        "original source inventory changed",
    )
    artifact = report["build_artifact"]
    checked_fingerprint(artifact["fingerprint"])
    target_source = Path(artifact["target_source"])
    require(
        target_source.is_absolute() and source in target_source.parents,
        "original build artifact did not come from the exported tree",
    )

    commands = report["commands"]
    require(
        commands and all(command.get("status") == 0 for command in commands),
        "original command sequence did not pass",
    )
    builds = [
        command for command in commands if command["argv"][:2] == ["cargo", "build"]
    ]
    require(
        len(builds) == 1, "original independent Rust build command missing or ambiguous"
    )
    build = builds[0]
    require(
        Path(build["cwd"]) == source / "codex-rs",
        "original build was not inside export",
    )
    for flag in ("-p", "--bin"):
        require(
            option(build["argv"], flag) == "codex-thread-store-local-plugin",
            "original build target differs from native storage",
        )
    for verb, flag, destination in (
        ("export", "--output", source),
        ("assemble", "--repo", source),
        ("assemble", "--output", package),
    ):
        matches = [
            command
            for command in commands
            if len(command["argv"]) > 2
            and Path(command["argv"][1]).name == "rust_component_package.py"
            and command["argv"][2] == verb
        ]
        require(len(matches) == 1, f"original {verb} command missing or ambiguous")
        require(
            Path(option(matches[0]["argv"], flag)) == destination,
            f"original {verb} {flag} does not match preserved artifacts",
        )

    runtime_path = Path(report["runtime_report"])
    require(
        runtime_path == directory / "runtime/acceptance.json",
        "unexpected original runtime report",
    )
    runtime = json.loads(runtime_path.read_text())
    require(runtime["passed"] is True, "original runtime did not pass")
    require(
        runtime["binaries_before"] == before == runtime["binaries_after"],
        "original runtime used different binaries",
    )
    require(
        runtime["thread_store_plugin"] == manifest["id"],
        "original runtime used different storage",
    )
    return report, package, manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--codex", type=Path, required=True)
    parser.add_argument("--independent-report", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()
    host, codex = args.host.resolve(strict=True), args.codex.resolve(strict=True)
    proof_path = args.independent_report.resolve(strict=True)
    work = args.work_dir.resolve()
    if os.path.lexists(work) or work == SDK.parent or SDK.parent in work.parents:
        parser.error("work directory must be new and outside the source checkout")
    proof_before = fingerprint(proof_path)
    original, package, manifest = original_proof(proof_path)
    if package.parent == work or package.parent in work.parents:
        parser.error("work directory must be outside the original artifact directory")
    require(
        fingerprint(proof_path) == proof_before,
        "source build report changed during validation",
    )

    os.umask(0o077)
    work.mkdir(mode=0o700, parents=True)
    runner = SDK / "tests/harness_acceptance.py"
    before = {"host": fingerprint(host), "codex": fingerprint(codex)}
    report = {
        "passed": False,
        "evidence_kind": "reused_package_new_host",
        "new_independent_build": False,
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "contract_version": 2,
        "package_version": original["package_version"],
        "source_build_report": str(proof_path),
        "source_build_report_fingerprint": proof_before,
        "source_build_binaries": original["binaries_before"],
        "source_removed_before_install": True,
        "source_removal_scope": "Original exported build tree remains absent; this run performed no Rust export or build.",
        "reused_package": str(package),
        "package_files": original["package_files"],
        # Existing manual/GUI acceptance intentionally resolve this directory/package.
        "artifact_directory": str(package.parent),
        "reuse_artifact_directory": str(work),
        "binaries_before": before,
        "runtime_runner": str(runner),
        "runtime_runner_fingerprint": fingerprint(runner),
        "runtime_report": str(work / "runtime/acceptance.json"),
        "commands": [],
        "provenance_note": "Independent storage build belongs to source_build_report. This report proves reuse with the current binaries only; the harness also builds its small Python fixture plugins.",
    }
    try:
        argv = [
            sys.executable,
            str(runner),
            "--host",
            str(host),
            "--codex",
            str(codex),
            "--work-dir",
            str(work / "runtime"),
            "--thread-store-package",
            str(package),
        ]
        command = {
            "argv": argv,
            "cwd": str(SDK.parent),
            "stdout": str(work / "runtime.stdout"),
            "stderr": str(work / "runtime.stderr"),
        }
        report["commands"].append(command)
        started = time.monotonic()
        with (
            Path(command["stdout"]).open("w") as stdout,
            Path(command["stderr"]).open("w") as stderr,
        ):
            result = subprocess.run(
                argv,
                cwd=SDK.parent,
                stdin=subprocess.DEVNULL,
                stdout=stdout,
                stderr=stderr,
                env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
            )
        command.update(
            status=result.returncode, elapsed_seconds=time.monotonic() - started
        )
        require(
            result.returncode == 0,
            f"runtime failed ({result.returncode}); see {command['stderr']}",
        )
        runtime_path = Path(report["runtime_report"])
        runtime = json.loads(runtime_path.read_text())
        require(runtime["passed"] is True, "new-host runtime did not pass")
        require(
            runtime["binaries_before"] == before == runtime["binaries_after"],
            "runtime used or modified different candidate binaries",
        )
        require(
            runtime["thread_store_plugin"] == manifest["id"],
            "runtime did not select reused storage",
        )
        require(
            len(runtime["storage_children"]) >= 3
            and all(
                child["terminated"] is True for child in runtime["storage_children"]
            ),
            "runtime did not confirm repeated storage-process termination",
        )
        report["runtime_report_fingerprint"] = fingerprint(runtime_path)
        report["verified"] = [
            "original independent build provenance validated",
            "unchanged old package installed against current host and CLI",
            "real CLI turns, tool execution, streaming and cold session resume",
            "storage child termination and native restoration with retained history",
        ]
    except BaseException as error:
        report["error"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        integrity_errors = []
        checks = {
            "binaries_after": lambda: {
                "host": fingerprint(host),
                "codex": fingerprint(codex),
            },
            "package_files_after": lambda: package_fingerprints(package),
            "source_build_report_after": lambda: fingerprint(proof_path),
            "runtime_runner_after": lambda: fingerprint(runner),
        }
        expected = {
            "binaries_after": before,
            "package_files_after": original["package_files"],
            "source_build_report_after": proof_before,
            "runtime_runner_after": report["runtime_runner_fingerprint"],
        }
        for key, check in checks.items():
            try:
                report[key] = check()
                require(
                    report[key] == expected[key], f"{key} changed during acceptance"
                )
            except Exception as error:
                integrity_errors.append(f"{key}: {error}")
        report["source_export_absent_after"] = not os.path.lexists(
            package.parent / "source"
        )
        if not report["source_export_absent_after"]:
            integrity_errors.append("original exported source tree reappeared")
        report["integrity_errors"] = integrity_errors
        report["passed"] = "error" not in report and not integrity_errors
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        output = work / "independent-report.json"
        output.write_text(json.dumps(report, indent=2) + "\n")
        if integrity_errors and "error" not in report:
            raise RuntimeError(f"post-run integrity checks failed; see {output}")
    print(output)


if __name__ == "__main__":
    main()
