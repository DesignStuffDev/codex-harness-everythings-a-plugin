"""Retained, isolated real-manager acceptance; no repository fetch or source edits."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

SDK = Path(__file__).resolve().parents[1]
PLUGIN_ID = "codex.maintenance.upstream-review"
NAME = "upstream_impact_review"


def fingerprint(path):
    before = path.stat()
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    after = path.stat()
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    assert all(getattr(before, field) == getattr(after, field) for field in fields)
    return {
        "sha256": digest.hexdigest(),
        "bytes": after.st_size,
        "identity": [getattr(after, field) for field in fields],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True, type=Path)
    parser.add_argument("--sdk-python", required=True, type=Path)
    parser.add_argument("--request", required=True, type=Path)
    parser.add_argument("--work-dir", required=True, type=Path)
    args = parser.parse_args()
    host = args.host.resolve(strict=True)
    sdk_python = args.sdk_python.absolute()
    work = args.work_dir.absolute()
    assert SDK.parent != work and SDK.parent not in work.parents, (
        "work must be outside harness source"
    )
    work.mkdir(mode=0o700, parents=True, exist_ok=False)
    with args.request.open("rb") as stream:
        request_raw = stream.read(32 * 1024 + 1)
    assert len(request_raw) <= 32 * 1024
    request = json.loads(request_raw)
    before = fingerprint(host)
    commands = []
    environment = dict(os.environ, PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1")
    home = work / "isolated-component-home"
    base = [str(host), "--codex-home", str(home)]

    def run(argv, *, expected=0, env=environment):
        result = subprocess.run(
            [str(value) for value in argv],
            cwd=work,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=130,
            check=False,
        )
        # Commands are controlled here; do not retain raw stderr or ambient configuration.
        commands.append(
            {
                "ordinal": len(commands) + 1,
                "operation": str(argv[0]),
                "returncode": result.returncode,
                "stdout_sha256": hashlib.sha256(result.stdout).hexdigest(),
                "stderr_sha256": hashlib.sha256(result.stderr).hexdigest(),
                "stdout_bytes": len(result.stdout),
            }
        )
        assert len(result.stdout) <= 256 * 1024, "acceptance output limit"
        assert (
            result.returncode != 0
            if expected is None
            else result.returncode == expected
        )
        return result.stdout

    def invoke(value):
        params = {"call_id": "maintenance-acceptance", "name": NAME, "arguments": value}
        tool = json.loads(
            run(base + ["call", "tool", NAME, "invoke", json.dumps(params)])
        )
        report = json.loads(tool["text"])
        assert len(tool["text"].encode()) <= 8192
        assert report["update_allowed"] is False
        assert tool["success"] == (report["status"] == "review_required")
        return report

    try:
        sdk = json.loads(
            run(
                [
                    sdk_python,
                    "-I",
                    "-B",
                    "-c",
                    "import codex_component_sdk,importlib.metadata,json; print(json.dumps({'path':codex_component_sdk.__file__,'version':importlib.metadata.version('codex-component-sdk')}))",
                ]
            )
        )
        assert SDK.parent not in Path(sdk["path"]).resolve().parents, (
            "SDK must be an installed distribution"
        )
        packages = []
        for number, identity in enumerate((PLUGIN_ID, PLUGIN_ID + ".replacement")):
            project = work / f"external-project-{number}"
            package = work / f"package-{number}"
            shutil.copytree(SDK / "examples" / "upstream-maintenance", project)
            manifest_path = project / "codex-component.json"
            manifest = json.loads(manifest_path.read_text())
            manifest["id"] = identity
            manifest_path.write_text(json.dumps(manifest) + "\n")
            if number == 1:
                # A test-only capability marker proves the selected executable
                # ran; analysis remains identical after removing this marker.
                plugin_path = project / "plugin.py"
                source = plugin_path.read_text()
                anchor = "    # success describes report computation, never permission to update."
                assert source.count(anchor) == 1
                plugin_path.write_text(
                    source.replace(
                        anchor, '    report["acceptance_marker"] = 2\n' + anchor
                    )
                )
            run(
                [
                    sdk_python,
                    "-I",
                    "-B",
                    project / "build_package.py",
                    "--output",
                    package,
                ]
            )
            # Only this helper's newly created disposable project copies are removed.
            shutil.rmtree(project)
            assert not project.exists()
            packages.append(package)

        assert run(base + ["install", packages[0]]).decode().strip() == PLUGIN_ID
        assert PLUGIN_ID in json.loads(run(base + ["list"]))["enabled"]
        report = invoke(request)
        assert report["status"] == "review_required"
        assert report["candidate_revision"] == request["candidate_revision"]
        assert report["composition_revision"] == request["composition_revision"]
        assert report["index_sha256"] == request["lineage_sha256"]
        assert report.get("report", {}).get("unresolved") or report.get(
            "report_unresolved"
        ), "review blockers must remain visible"
        wrong_digest = dict(request, lineage_sha256="0" * 64)
        assert invoke(wrong_digest)["diagnostic_code"] == "input_digest_mismatch"
        unsupported = invoke(dict(request, contract_version=2))
        assert unsupported["status"] == "invalid"
        missing = invoke(dict(request, candidate_revision="0" * 40))
        assert missing["candidate_revision"] == "0" * 40
        missing_findings = missing.get("report", {}).get(
            "unresolved", missing.get("report_unresolved", [])
        )
        assert "input_or_local_object_unavailable_or_unsupported" in missing_findings
        assert missing["update_allowed"] is False
        incompatible = work / "package-incompatible"
        shutil.copytree(packages[0], incompatible)
        incompatible_manifest = json.loads(
            (incompatible / "codex-component.json").read_text()
        )
        incompatible_manifest.update(id=PLUGIN_ID + ".incompatible", api_version=2)
        incompatible_manifest["components"][0]["name"] = NAME + "_incompatible"
        (incompatible / "codex-component.json").write_text(
            json.dumps(incompatible_manifest) + "\n"
        )
        run(base + ["install", incompatible], expected=None)
        assert json.loads(run(base + ["list"]))["enabled"] == [PLUGIN_ID]
        # No Codex executable is discoverable in this bootstrap process's PATH.
        # Absolute binaries elsewhere in the VM remain intact.
        only = work / "bootstrap-path"
        only.mkdir(mode=0o700)
        git = shutil.which("git")
        assert git
        (only / "git").symlink_to(Path(git).resolve(strict=True))
        assert shutil.which("codex-component", path=str(only)) is None
        request_file = work / "request.json"
        request_file.write_bytes(request_raw)
        bootstrap = json.loads(
            run(
                [
                    sys.executable,
                    "-I",
                    "-B",
                    packages[0] / "bootstrap.py",
                    request_file,
                ],
                expected=2,
                env=dict(environment, PATH=str(only)),
            )
        )
        assert bootstrap == report
        # Catalog rejects an ambiguous install before activation. Select the
        # existing provider first, then explicitly switch to the new identity.
        run(base + ["install", packages[1]], expected=None)
        assert json.loads(run(base + ["list"]))["enabled"] == [PLUGIN_ID]
        run(base + ["select", "tool", NAME, PLUGIN_ID])
        assert (
            run(base + ["install", packages[1]]).decode().strip()
            == PLUGIN_ID + ".replacement"
        )
        params = json.dumps(
            {"call_id": "ambiguous", "name": NAME, "arguments": request}
        )
        run(base + ["select", "tool", NAME, PLUGIN_ID + ".replacement"])
        assert (
            json.loads(run(base + ["list"]))["selections"]["tool:" + NAME]
            == PLUGIN_ID + ".replacement"
        )
        replacement = invoke(request)
        assert replacement.pop("acceptance_marker") == 2
        assert replacement == report
        run(base + ["remove", PLUGIN_ID + ".replacement"])
        run(base + ["remove", PLUGIN_ID])
        assert json.loads(run(base + ["list"]))["enabled"] == []
        run(base + ["call", "tool", NAME, "invoke", params], expected=None)
        after = fingerprint(host)
        assert before == after, "host changed during separately built plugin acceptance"
        result = {
            "passed": True,
            "host_before": before,
            "host_after": after,
            "sdk_distribution": sdk,
            "request_sha256": hashlib.sha256(request_raw).hexdigest(),
            "independent_project_copies_removed_before_install": True,
            "bootstrap_host_absent_from_PATH": True,
            "new_host_build_performed": False,
            "initial_report": report,
            "missing_revision_report": missing,
            "replacement_marker_observed": True,
            "replacement_analysis_identical": True,
            "commands": commands,
            "scope": "real manager install/invoke/selection/removal and packaged bootstrap; no engine-session, upstream integration, or rollback proof",
        }
        (work / "acceptance.json").write_text(json.dumps(result, indent=2) + "\n")
        print(work / "acceptance.json")
    except Exception:
        (work / "acceptance-failure.json").write_text(
            json.dumps({"passed": False, "commands": commands}, indent=2) + "\n"
        )
        raise


if __name__ == "__main__":
    main()
