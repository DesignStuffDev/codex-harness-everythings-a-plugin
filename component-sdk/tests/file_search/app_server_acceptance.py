#!/usr/bin/env python3
"""Real frozen App Server + separately built/installed native-search acceptance.

No compilation, model service, fake backend, or search protocol stub is used.
Run inside the unchanged SDK Linux subreaper. Runtime execution belongs to root.
"""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import sys

from acceptance_support import (
    Runner,
    fingerprint,
    inventory,
    require,
    same_content,
    write_report,
)
from rpc_support import RpcServer
from search_cases import (
    PLUGIN,
    assert_resource_cause,
    exhaust_real_snapshot,
    one_shots,
    prepare_fixture,
    streams,
)


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--server", type=Path, required=True)
    parser.add_argument("--server-sha256", required=True)
    parser.add_argument("--server-build-report", type=Path, required=True)
    parser.add_argument(
        "--server-mode", choices=("standalone", "codex"), default="standalone"
    )
    parser.add_argument("--manager", type=Path, required=True)
    parser.add_argument("--build-report", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    return parser.parse_args()


def main():
    args = arguments()
    repo = args.repo.resolve(strict=True)
    server = args.server.resolve(strict=True)
    manager = args.manager.resolve(strict=True)
    server_proof_path = args.server_build_report.resolve(strict=True)
    server_proof = json.loads(server_proof_path.read_text())
    require(
        server_proof.get("returncode") == 0
        and server_proof.get("scoped_source_unchanged") is True,
        "App Server needs a successful source-unchanged root build report",
    )
    before = {"server": fingerprint(server), "manager": fingerprint(manager)}
    require(
        before["server"]["sha256"] == args.server_sha256,
        "App Server differs from supplied frozen digest",
    )
    proof_path = args.build_report.resolve(strict=True)
    proof = json.loads(proof_path.read_text())
    require(
        proof.get("passed") is True
        and proof.get("evidence_kind") == "independent_source_build",
        "worker input is not an accepted independent source build",
    )
    require(
        proof["build_artifact"]["separate_initially_empty_target"] is True
        and proof["build_artifact"]["fresh"] is False,
        "worker was not independently compiled",
    )
    require(
        proof["source_resolution"]["all_local_paths_inside_export"] is True
        and proof["source_resolution"]["lock_new_identities"] == 0,
        "worker dependency/source proof missing",
    )
    require(
        proof["source_parked_before_runtime"] is True
        and not (Path(proof["artifact_directory"]) / "source").exists(),
        "worker build source was not parked",
    )
    source_inventory = Path(proof["artifact_directory"]) / "source-inventory.json"
    require(
        fingerprint(source_inventory) == proof["exported_source_inventory"],
        "source inventory changed",
    )
    require(
        proof["frozen_binaries_before"]["manager"]
        == proof["frozen_binaries_after"]["manager"]
        == before["manager"],
        "manager differs from independent build proof",
    )
    package = Path(proof["package_path"]).resolve(strict=True)
    package_files = inventory(package)
    require(
        package_files == proof["package_files"], "independently built package changed"
    )
    work = args.work_dir.resolve()
    require(
        not work.exists() and work != repo and repo not in work.parents,
        "acceptance artifacts must be new and outside checkout",
    )
    work.mkdir(parents=True, mode=0o700)
    report_path = work / "app-server-acceptance.json"
    tooling = {p.name: fingerprint(p) for p in Path(__file__).parent.glob("*.py")}
    helper = repo / "component-sdk/tests/file_search/acceptance_support.py"
    require(
        fingerprint(helper)["sha256"] == tooling["acceptance_support.py"]["sha256"],
        "copied strict SDK helper differs from repository helper",
    )
    subreaper = repo / "component-sdk/tests/subreaper_runner.py"
    report = {
        "passed": False,
        "evidence_kind": "real_app_server_independent_native_search",
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "artifact_directory": str(work),
        "commands": [],
        "rpc_processes": [],
        "binaries_before": before,
        "app_server_build_report": {
            "path": str(server_proof_path),
            **fingerprint(server_proof_path),
        },
        "worker_build_report": {"path": str(proof_path), **fingerprint(proof_path)},
        "independent_package": {"path": str(package), "files": package_files},
        "tooling": tooling,
        "subreaper": {"path": str(subreaper), **fingerprint(subreaper)},
        "scope": "Real App Server stdio search with installed native worker. No model turns, TUI, GUI, Browser, or multiple-connection proof.",
        "parity": "Exact ordered JSON values including root/path/file_name/match_type/score/highlight indices; no normalization.",
        "pid_policy": "All observed App Server/worker/descendant PIDs must be absent, including zombies. No retry or forced-cleanup pass.",
    }
    runner = Runner(work, report, report_path)
    home, base = work / "home", work / "fixture"
    home.mkdir()
    base.mkdir()
    (home / "config.toml").write_text("[features]\nshell_snapshot = false\n")
    environment = dict(
        os.environ,
        CODEX_HOME=str(home),
        RUST_LOG="warn",
        CODEX_APP_SERVER_MANAGED_CONFIG_PATH=str(home / "managed_config.toml"),
    )
    environment.pop("CODEX_INTERNAL_ORIGINATOR_OVERRIDE", None)
    server_command = [
        server,
        *(["app-server"] if args.server_mode == "codex" else []),
        "--listen",
        "stdio://",
    ]
    management = [manager, "--codex-home", home]
    active = None
    installed = worker = None
    try:
        prepare_fixture(base)
        report["fixture_files"] = inventory(base)
        active = RpcServer(
            "native-baseline",
            server_command,
            base,
            environment,
            work,
            report,
            report_path,
        )
        active.initialize()
        baseline = one_shots(active)
        report["native_baseline"] = baseline
        active.finish()
        active = None

        install_input = work / "install-input"
        shutil.copytree(package, install_input)
        require(
            same_content(inventory(install_input), package_files),
            "package copy differs",
        )
        runner.run(
            "install",
            [*management, "install", install_input],
            cwd=base,
            environment=environment,
        )
        runner.run(
            "select",
            [*management, "select", "file_search", "default", PLUGIN],
            cwd=base,
            environment=environment,
        )
        output, _, _ = runner.run(
            "list-selected", [*management, "list"], cwd=base, environment=environment
        )
        settings = json.loads(output)
        require(
            settings["selections"].get("file_search:default") == PLUGIN,
            "catalog selection missing",
        )
        installed = home / "components/objects" / settings["installed"][PLUGIN]
        require(
            same_content(inventory(installed), package_files),
            "installed object differs from separate build",
        )
        manifest = json.loads((installed / "codex-component.json").read_text())
        require(
            manifest["id"] == PLUGIN
            and manifest["api_version"] == 1
            and any(
                item["kind"] == "file_search"
                and item["name"] == "default"
                and item["contract_version"] == 1
                for item in manifest["components"]
            ),
            "installed contract incompatible",
        )
        worker = (installed / manifest["entrypoint"]).resolve(strict=True)
        require(
            installed in worker.parents, "worker escapes installed immutable package"
        )
        report["installed_worker"] = {"path": str(worker), **fingerprint(worker)}
        report["installed_package_files"] = inventory(installed)
        install_input.rename(work / "install-input.parked")
        report["install_input_parked_before_execution"] = True

        active = RpcServer(
            "installed-parity-and-streams",
            server_command,
            base,
            environment,
            work,
            report,
            report_path,
        )
        active.initialize()
        active.require_worker(worker)
        one_shots(active, baseline)
        streams(active, baseline)
        active.finish()  # One live sibling lease must be closed by actual EOF shutdown.
        active = None

        settings_path = home / "components/config.json"
        good_settings = settings_path.read_text()
        small_snapshot = json.loads(good_settings)
        small_snapshot.setdefault("config", {})[PLUGIN] = {
            "native_snapshot_bytes": 65536
        }
        settings_path.write_text(json.dumps(small_snapshot, indent=2) + "\n")
        active = RpcServer(
            "installed-resource-failure",
            server_command,
            base,
            environment,
            work,
            report,
            report_path,
        )
        active.initialize()
        active.require_worker(worker)
        exhaust_real_snapshot(active, baseline)
        active.finish(expected_status="nonzero")
        assert_resource_cause(active.error_path.read_text(), "normal EOF shutdown")
        active = None

        invalid_settings = json.loads(good_settings)
        invalid_settings.setdefault("config", {})[PLUGIN] = {
            "unsupported_acceptance_option": True
        }
        settings_path.write_text(json.dumps(invalid_settings, indent=2) + "\n")
        output, errors, entry = runner.run(
            "selected-failure-no-fallback",
            server_command,
            cwd=base,
            environment=environment,
            expected_status="nonzero",
        )
        require(
            not output.strip(),
            "invalid selection served protocol output instead of failing startup",
        )
        require(
            "unsupported field or type" in errors,
            "selected native config rejection was lost",
        )
        report["selected_failure_no_fallback"] = {
            "status": entry["status"],
            "before_initialize": True,
        }
        settings_path.write_text(good_settings)
        runner.run(
            "remove", [*management, "remove", PLUGIN], cwd=base, environment=environment
        )
        output, _, _ = runner.run(
            "list-removed", [*management, "list"], cwd=base, environment=environment
        )
        removed = json.loads(output)
        require(
            PLUGIN not in removed["installed"]
            and PLUGIN not in removed["enabled"]
            and "file_search:default" not in removed["selections"],
            "removal left selection active",
        )
        require(
            inventory(installed) == report["installed_package_files"],
            "removal changed preserved immutable object",
        )
        active = RpcServer(
            "removed-native-restored",
            server_command,
            base,
            environment,
            work,
            report,
            report_path,
        )
        active.initialize()
        one_shots(active, baseline)
        streams(active, baseline)
        active.finish()
        require(
            not any(
                p.get("executable") == str(worker)
                for p in active.entry["observed_processes"]
            ),
            "removed plugin executed during native restoration",
        )
        active = None
        report["passed"] = True
    except BaseException as error:
        report["failure"] = f"{type(error).__name__}: {error}"
        if active is not None:
            try:
                active.abort()
            except BaseException as cleanup:
                report["failure_cleanup_error"] = f"{type(cleanup).__name__}: {cleanup}"
        raise
    finally:
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        report["binaries_after"] = {
            "server": fingerprint(server),
            "manager": fingerprint(manager),
        }
        report["frozen_binaries_unchanged"] = report["binaries_after"] == before
        report["independent_package_unchanged"] = inventory(package) == package_files
        report["build_proof_unchanged"] = fingerprint(proof_path) == {
            k: v for k, v in report["worker_build_report"].items() if k != "path"
        }
        report["server_proof_unchanged"] = fingerprint(server_proof_path) == {
            k: v for k, v in report["app_server_build_report"].items() if k != "path"
        }
        report["tooling_unchanged"] = {
            p.name: fingerprint(p) for p in Path(__file__).parent.glob("*.py")
        } == tooling
        report["subreaper_unchanged"] = fingerprint(subreaper) == {
            k: v for k, v in report["subreaper"].items() if k != "path"
        }
        report["installed_package_unchanged"] = installed is None or inventory(
            installed
        ) == report.get("installed_package_files")
        for key in (
            "frozen_binaries_unchanged",
            "independent_package_unchanged",
            "build_proof_unchanged",
            "server_proof_unchanged",
            "tooling_unchanged",
            "subreaper_unchanged",
            "installed_package_unchanged",
        ):
            if not report[key]:
                report["passed"] = False
        write_report(report_path, report)
    require(report["passed"], "acceptance did not pass; see report")
    print(report_path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
