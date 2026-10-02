#!/usr/bin/env python3
"""Isolated production in-process caller test. Run under the SDK strict subreaper.

Reuses only the existing curated fixture's setup/HTTP Git/cleanup; its three
original cases and assertions remain unchanged and must still pass separately.
"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time

sys.dont_write_bytecode = True

CASE = "in_process::featured_warmup_tests::production_featured_public_drop_child"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def require(value, message):
    if not value:
        raise RuntimeError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("test-elf", "fixture-source", "source-binding", "work-dir"):
        parser.add_argument("--" + name, type=Path, required=True)
    for name in (
        "test-elf-sha256",
        "fixture-source-sha256",
        "source-binding-sha256",
        "git-sha256",
    ):
        parser.add_argument("--" + name, required=True)
    args = parser.parse_args()
    require(sys.platform == "linux", "Linux child fixture required")
    for name in ("test_elf", "fixture_source", "source_binding"):
        path = getattr(args, name).resolve(strict=True)
        require(
            digest(path) == getattr(args, name + "_sha256"), name + " identity mismatch"
        )
        setattr(args, name, path)
    args.work_dir = args.work_dir.absolute()
    require(not args.work_dir.exists(), "fresh fixture directory required")
    spec = importlib.util.spec_from_file_location(
        "curated_caller_fixture", args.fixture_source
    )
    require(spec is not None and spec.loader is not None, "fixture import failed")
    fixture_module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixture_module)
    git = fixture_module.trusted_git()
    require(digest(git) == args.git_sha256, "trusted Git identity mismatch")
    args.work_dir.mkdir(parents=True)
    fixture = fixture_module.Fixture(
        args.work_dir, git, "featured-caller", time.monotonic() + 120
    )
    child = None
    report = {
        "passed": False,
        "whole_host_clean": False,
        "strict_runner_required": True,
    }
    try:
        fixture.prepare()
        # This case holds featured HTTP in Rust, not Git. Native Git still runs
        # through the exact existing smart-HTTP backend and normal URL rewrite.
        fixture.git_release.set()
        env = os.environ.copy()
        for name in ("CODEX_API_KEY", "OPENAI_API_KEY", "CHATGPT_API_KEY"):
            env.pop(name, None)
        env.update(
            {
                "CODEX_HOME": str(fixture.home),
                "HOME": str(fixture.home),
                "CODEX_SQLITE_HOME": str(fixture.home),
                "RUST_MIN_STACK": "8388608",
                "GIT_CONFIG_GLOBAL": str(fixture.git_config),
                "CODEX_TEST_FEATURED_WARMUP_HOME": str(fixture.home),
            }
        )
        with (
            (args.work_dir / "child.stdout").open("wb") as out,
            (args.work_dir / "child.stderr").open("wb") as err,
        ):
            child = subprocess.Popen(
                [
                    str(args.test_elf),
                    "--ignored",
                    "--exact",
                    CASE,
                    "--nocapture",
                    "--test-threads=1",
                ],
                cwd=fixture.home,
                env=env,
                stdout=out,
                stderr=err,
            )
            require(
                child.wait(timeout=fixture.remaining()) == 0,
                "native caller case failed",
            )
        receipt = json.loads((fixture.home / "featured-warmup-child.json").read_text())
        require(receipt.get("passed") is True, "native caller receipt did not pass")
        require(
            len(fixture.git_requests) >= 2, "native smart-HTTP Git was not exercised"
        )
        require(
            fixture.model_requests == 0 and not fixture.mcp_calls,
            "unexpected model/MCP use",
        )
        require(
            not fixture.errors and not fixture.forced, "fixture failed before cleanup"
        )
        require(digest(args.test_elf) == args.test_elf_sha256, "test ELF changed")
        require(
            digest(args.fixture_source) == args.fixture_source_sha256, "fixture changed"
        )
        require(
            digest(args.source_binding) == args.source_binding_sha256,
            "source binding changed",
        )
        require(digest(git) == args.git_sha256, "trusted Git changed")
        report.update(
            {"passed": True, "child": receipt, "identities": fixture.identities}
        )
    except Exception as error:
        report["failure"] = type(error).__name__ + ": " + str(error)[:500]
    finally:
        try:
            if child is not None and child.poll() is None:
                fixture.forced.append("child required failure cleanup")
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=5)
        except Exception as error:
            fixture.errors.append("child cleanup: " + type(error).__name__)
        try:
            fixture.close()
        except Exception as error:
            fixture.errors.append("fixture cleanup: " + type(error).__name__)
        report.update(
            {
                "child_returncode": child.returncode if child else None,
                "fixture_errors": fixture.errors,
                "forced_cleanup": fixture.forced,
                "git_requests": fixture.git_requests,
                "backend_returncodes": [p.returncode for p in fixture.backends],
                "source_binding_sha256": args.source_binding_sha256,
                "test_elf_sha256": args.test_elf_sha256,
                "fixture_source_sha256": args.fixture_source_sha256,
            }
        )
        if fixture.forced or fixture.errors:
            report["passed"] = False
        fixture_module.write_json(args.work_dir / "result.json", report)
    require(report["passed"], "caller acceptance failed; see preserved result.json")


if __name__ == "__main__":
    main()
