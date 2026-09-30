"""Verify a wheel-installed SDK and plugin against a frozen, separately copied host."""

import argparse
from datetime import datetime, timezone
from email.parser import BytesParser
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile


SDK = Path(__file__).resolve().parents[1]


def fingerprint(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return {
        "sha256": digest.hexdigest(),
        "bytes": path.stat().st_size,
        "mtime_ns": path.stat().st_mtime_ns,
    }


def inventory(directory):
    return {
        path.relative_to(directory).as_posix(): fingerprint(path)
        for path in sorted(directory.rglob("*"))
        if path.is_file() and "__pycache__" not in path.parts
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.error("this acceptance script currently verifies Linux")
    work = args.work_dir.resolve()
    if work == SDK.parent or SDK.parent in work.parents:
        parser.error("work directory must be outside the harness source")
    work.mkdir(parents=True, exist_ok=False)
    original_host = args.host.resolve(strict=True)
    frozen_host = work / "frozen-bin" / "codex-component"
    frozen_host.parent.mkdir()
    original_fingerprint = fingerprint(original_host)
    shutil.copy2(original_host, frozen_host)
    before = fingerprint(frozen_host)
    assert before == original_fingerprint
    environment = dict(os.environ)
    for variable in ("PYTHONPATH", "PYTHONHOME"):
        environment.pop(variable, None)
    environment["PYTHONNOUSERSITE"] = "1"
    environment["PIP_DISABLE_PIP_VERSION_CHECK"] = "1"
    environment["PIP_NO_CACHE_DIR"] = "1"
    commands = []
    report = {
        "passed": False,
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "work_dir": str(work),
        "source_host": str(original_host),
        "frozen_host": str(frozen_host),
        "host_before": before,
        "python": sys.version,
        "build_tools": {
            name: importlib.metadata.version(name)
            for name in ("pip", "setuptools", "wheel")
        },
        "scope": "Python SDK distribution and actual package-manager/tool invocation; no new native extraction or whole-engine claim",
    }

    def run(command, *, expected=0):
        command = [str(argument) for argument in command]
        result = subprocess.run(
            command,
            cwd=work,
            env=environment,
            text=True,
            capture_output=True,
            timeout=120,
        )
        commands.append(
            {
                "command": command,
                "cwd": str(work),
                "status": result.returncode,
                "stdout": result.stdout,
                "stderr": result.stderr,
            }
        )
        assert result.returncode == expected, commands[-1]
        return result.stdout

    try:
        # Export just the SDK distribution. No native workspace or example source
        # can enter the wheel through automatic package discovery.
        source = work / "sdk-build-source"
        source.mkdir()
        for name in ("pyproject.toml", "README.md"):
            shutil.copy2(SDK / name, source / name)
        shutil.copytree(
            SDK / "codex_component_sdk",
            source / "codex_component_sdk",
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )
        report["sdk_source_inventory"] = inventory(source)
        expected_notices = {
            name: (source / "codex_component_sdk" / name).read_bytes()
            for name in ("LICENSE", "NOTICE")
        }
        run(
            [
                sys.executable,
                "-m",
                "pip",
                "wheel",
                "--no-index",
                "--no-deps",
                "--no-build-isolation",
                "--wheel-dir",
                work / "wheels",
                source,
            ]
        )
        wheels = list((work / "wheels").glob("*.whl"))
        assert len(wheels) == 1, wheels
        wheel = wheels[0]
        with zipfile.ZipFile(wheel) as archive:
            entries = sorted(archive.namelist())
            metadata_path = next(
                p for p in entries if p.endswith(".dist-info/METADATA")
            )
            metadata = BytesParser().parsebytes(archive.read(metadata_path))
            assert metadata["Name"] == "codex-component-sdk", metadata
            assert metadata["Version"] == "0.1.0", metadata
            assert metadata["Requires-Python"] == ">=3.10", metadata
            assert metadata["License-Expression"] == "Apache-2.0", metadata
            assert metadata.get_all("Requires-Dist") is None, metadata
            assert set(metadata.get_all("License-File")) == {
                "codex_component_sdk/LICENSE",
                "codex_component_sdk/NOTICE",
            }, metadata
            info_prefix = metadata_path.rsplit("/", 1)[0]
            for name, contents in expected_notices.items():
                assert archive.read(f"codex_component_sdk/{name}") == contents
                assert (
                    archive.read(f"{info_prefix}/licenses/codex_component_sdk/{name}")
                    == contents
                )
            assert all(
                p.startswith("codex_component_sdk/") or p.startswith(f"{info_prefix}/")
                for p in entries
            ), entries
        report["wheel"] = {"path": str(wheel), **fingerprint(wheel), "entries": entries}
        shutil.rmtree(source)
        report["sdk_build_source_removed_before_install"] = not source.exists()

        venv = work / "venv"
        run([sys.executable, "-m", "venv", str(venv)])
        python = venv / "bin" / "python"
        entrypoint = venv / "bin" / "codex-component-sdk"
        run([python, "-m", "pip", "install", "--no-index", "--no-deps", wheel])
        installation = json.loads(
            run(
                [
                    python,
                    "-I",
                    "-c",
                    "import importlib.metadata as m, json, codex_component_sdk as s; "
                    "d=m.distribution('codex-component-sdk'); "
                    "print(json.dumps({'module':s.__file__,'version':d.version,"
                    "'scripts':{e.name:e.value for e in d.entry_points if e.group=='console_scripts'}}))",
                ]
            )
        )
        assert Path(installation["module"]).is_relative_to(venv), installation
        assert installation["version"] == "0.1.0", installation
        assert installation["scripts"] == {
            "codex-component-sdk": "codex_component_sdk.__main__:main"
        }, installation
        report["installed_distribution"] = installation
        environment["PATH"] = str(venv / "bin") + os.pathsep + environment["PATH"]
        project = work / "custom-plugin-source"
        plugin_id = "acceptance.wheel-greeting"
        run([entrypoint, "init", project, "--id", plugin_id])
        (project / "plugin.py").write_text(
            "from importlib.resources import files\n"
            "from codex_component_sdk import Plugin\n"
            "app = Plugin()\n"
            '@app.handle("tool", "greeting", "invoke")\n'
            "def greet(params, context):\n"
            '    name = params["arguments"]["name"]\n'
            '    label = files("greeter_assets").joinpath("static/label.txt").read_text().strip()\n'
            '    return {"text": f"{label}: {name.upper()} ({len(name)} characters)", "success": True}\n'
        )
        (project / "greeter_assets" / "static").mkdir(parents=True)
        (project / "greeter_assets" / "__init__.py").write_text("")
        (project / "greeter_assets" / "static" / "label.txt").write_text(
            "Wheel-built greeting\n"
        )
        report["plugin_source_inventory"] = inventory(project)
        package = work / "package"
        run([entrypoint, "build", project, "--output", package])
        # The module entry point remains usable from the installed distribution.
        run([python, "-I", "-m", "codex_component_sdk", "--help"])
        with zipfile.ZipFile(package / "plugin.pyz") as archive:
            for name, contents in expected_notices.items():
                assert archive.read(f"codex_component_sdk/{name}") == contents
                assert (package / f"SDK-{name}").read_bytes() == contents
            assert (
                archive.read("greeter_assets/static/label.txt")
                == b"Wheel-built greeting\n"
            )
            assert "codex_component_sdk/__main__.py" in archive.namelist()
        assert os.access(package / "plugin.pyz", os.X_OK)
        report["package_inventory"] = inventory(package)
        shutil.rmtree(project)
        run([python, "-m", "pip", "uninstall", "--yes", "codex-component-sdk"])
        run(
            [
                python,
                "-I",
                "-c",
                "import importlib.util; assert importlib.util.find_spec('codex_component_sdk') is None",
            ]
        )
        report["plugin_source_removed_before_host_install"] = not project.exists()
        report["sdk_uninstalled_before_host_install"] = True
        report["pythonpath_unset"] = "PYTHONPATH" not in environment

        host_command = [frozen_host, "--codex-home", work / "codex-home"]
        assert run(host_command + ["install", package]).strip() == plugin_id
        settings = json.loads(run(host_command + ["list"]))
        assert plugin_id in settings["enabled"], settings
        invocation = host_command + [
            "call",
            "tool",
            "greeting",
            "invoke",
            json.dumps(
                {
                    "call_id": "wheel-acceptance",
                    "name": "greeting",
                    "arguments": {"name": "Ada Lovelace"},
                }
            ),
        ]
        actual = json.loads(run(invocation))
        assert actual == {
            "text": "Wheel-built greeting: ADA LOVELACE (12 characters)",
            "success": True,
        }, actual
        report["tool_result"] = actual
        run(host_command + ["remove", plugin_id])
        settings = json.loads(run(host_command + ["list"]))
        assert settings["enabled"] == [], settings
        run(invocation, expected=1)
        report["removed_plugin_unavailable"] = True
        report["host_after"] = fingerprint(frozen_host)
        assert report["host_after"] == before
        report["passed"] = True
    finally:
        (work / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
        (work / "acceptance.json").write_text(json.dumps(report, indent=2) + "\n")
    print(work / "acceptance.json")


if __name__ == "__main__":
    main()
