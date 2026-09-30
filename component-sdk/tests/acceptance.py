"""Build outside the harness, install, invoke, remove, and verify host immutability."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

SDK = Path(__file__).resolve().parents[1]


def fingerprint(path):
    digest = hashlib.sha256()
    with path.open("rb") as binary:
        for chunk in iter(lambda: binary.read(1024 * 1024), b""):
            digest.update(chunk)
    return {
        "sha256": digest.hexdigest(),
        "mtime_ns": path.stat().st_mtime_ns,
        "bytes": path.stat().st_size,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--host",
        type=Path,
        required=True,
        help="already-built codex-component executable",
    )
    parser.add_argument(
        "--work-dir", type=Path, help="new directory for retained evidence"
    )
    args = parser.parse_args()
    host = args.host.resolve(strict=True)
    before = fingerprint(host)
    work = (
        args.work_dir.absolute()
        if args.work_dir
        else Path(tempfile.mkdtemp(prefix="codex-component-acceptance-"))
    )
    if args.work_dir:
        work.mkdir(parents=True)
    if SDK.parent == work or SDK.parent in work.parents:
        parser.error("acceptance build directory must be outside the harness source")
    plugin_source = work / "external-plugin-source"
    package = work / "package"
    codex_home = work / "isolated-codex-home"
    shutil.copytree(SDK / "examples" / "calculator", plugin_source)
    commands = []

    def run(command, *, environment=None, expected=0):
        result = subprocess.run(
            command,
            cwd=work,
            env=environment,
            capture_output=True,
            text=True,
            timeout=30,
        )
        commands.append(
            {
                "command": [str(arg) for arg in command],
                "status": result.returncode,
                "stdout": result.stdout,
                "stderr": result.stderr,
            }
        )
        if result.returncode != expected:
            raise AssertionError(
                f"command exited {result.returncode}, expected {expected}: {command}\n{result.stderr}"
            )
        return result.stdout

    command = [str(host), "--codex-home", str(codex_home)]
    try:
        run(
            [
                sys.executable,
                "-m",
                "codex_component_sdk",
                "build",
                str(plugin_source),
                "--output",
                str(package),
            ],
            environment=dict(os.environ, PYTHONPATH=str(SDK)),
        )
        shutil.rmtree(plugin_source)
        # No SDK/harness source import path is present when the installed package runs.
        environment = dict(os.environ, PYTHONPATH="")
        installed = run(
            command + ["install", str(package)], environment=environment
        ).strip()
        assert installed == "example.calculator", installed
        tool = json.loads(
            run(
                command
                + [
                    "call",
                    "tool",
                    "calculator",
                    "invoke",
                    json.dumps(
                        {
                            "call_id": "acceptance-call",
                            "name": "calculator",
                            "arguments": {"operation": "product", "values": [6, 7]},
                        }
                    ),
                ],
                environment=environment,
            )
        )
        assert tool == {"text": "42", "success": True}, tool
        context = json.loads(
            run(
                command
                + [
                    "call",
                    "context",
                    "calculator_help",
                    "contribute",
                    json.dumps(
                        {
                            "session_id": "acceptance-session",
                            "thread_id": "acceptance-thread",
                            "turn_id": "acceptance-turn",
                        }
                    ),
                ],
                environment=environment,
            )
        )
        assert "calculator" in context["text"], context
        settings = json.loads(run(command + ["list"], environment=environment))
        assert "example.calculator" in settings["enabled"], settings
        run(command + ["remove", "example.calculator"], environment=environment)
        settings = json.loads(run(command + ["list"], environment=environment))
        assert settings["enabled"] == [], settings
        after = fingerprint(host)
        assert after == before, {"before": before, "after": after}
        report = {
            "passed": True,
            "host": str(host),
            "host_before": before,
            "host_after": after,
            "built_outside_harness": True,
            "source_removed_before_invocation": True,
            "scope": "component management and tool/context process contracts; full Codex engine separately verified",
            "commands": commands,
        }
    except Exception:
        (work / "acceptance-failure.json").write_text(
            json.dumps({"passed": False, "commands": commands}, indent=2) + "\n"
        )
        raise
    (work / "acceptance.json").write_text(json.dumps(report, indent=2) + "\n")
    print(work / "acceptance.json")


if __name__ == "__main__":
    main()
