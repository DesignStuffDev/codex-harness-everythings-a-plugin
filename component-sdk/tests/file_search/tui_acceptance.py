#!/usr/bin/env python3
"""Drive the real frozen full Codex CLI and independently built search package.

Run only under the unchanged SDK subreaper_runner.py. No Rust, model server,
protocol stub, desktop viewer, or build command is invoked by this gate.
"""

import argparse
from datetime import datetime, timezone
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import time

PLUGIN = "native.file-search-local"
ALPHA = "p02alpha_visible_in_a.rs"
BETA = "p02beta_after_resume_in_a.md"
ROOT_A = "p02root_only_a.rs"
ROOT_B = "p02root_only_b.rs"


def load_support(repo):
    path = repo / "component-sdk/tests/file_search/acceptance_support.py"
    spec = importlib.util.spec_from_file_location("acceptance_support", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module, path


def session_records(path):
    if not path.exists():
        return []
    # The TUI logger flushes each bounded JSON line; ignore only an unfinished tail.
    data = path.read_text()
    if not data.endswith("\n"):
        data = data.rsplit("\n", 1)[0] if "\n" in data else ""
    return [json.loads(line) for line in data.splitlines()]


def config(home, roots):
    text = """model = "gpt-6-sol"
model_provider = "search_acceptance_offline"
cli_auth_credentials_store = "file"
analytics.enabled = false
check_for_update_on_startup = false
suppress_unstable_features_warning = true
features.daemon_auto_start = false
features.api_key_model_discovery = false
tui.disable_paste_burst = true
[model_providers.search_acceptance_offline]
name = "Search acceptance (no model turn)"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
"""
    for root in roots:
        text += "\n[projects." + json.dumps(str(root)) + ']\ntrust_level = "trusted"\n'
    (home / "config.toml").write_text(text)


def environment(home, log):
    # Deliberate allowlist: no inherited model keys, cloud tokens or user config.
    # This is a private fixture CODEX_HOME, not a rewrite of HOME.
    return {
        "PATH": os.defpath,
        "LANG": "C.UTF-8",
        "TERM": "xterm-256color",
        "COLORTERM": "truecolor",
        "CODEX_HOME": str(home),
        "CODEX_TUI_RECORD_SESSION": "1",
        "CODEX_TUI_SESSION_LOG_PATH": str(log),
        "RUST_LOG": "warn",
        "NO_PROXY": "127.0.0.1,localhost",
    }


def clear_draft(case, draft):
    # No Enter while the @ popup has a draft: Enter could start an actual model
    # turn when there is no selection. The runner owns these ASCII keystrokes.
    case.write(b"\x7f" * len(draft), label="erase-known-draft")
    case.wait(
        "draft-cleared-" + str(len(case.entry["steps"])),
        lambda: draft not in case.screen.text(),
    )
    case.hold(0.12, lambda: True, "")


def type_query(case, query, expected, forbidden=()):
    case.write(("@" + query).encode(), label="type-at-query-" + query)
    text = case.wait("query-" + query, lambda: expected in case.screen.text())
    for value in forbidden:
        require(value not in text, f"{query}: forbidden root/ignored filename appeared")
    return "@" + query


def exercise(case, root_b, log, causal):
    case.wait(
        "ready",
        lambda: (
            "›" in case.screen.text()
            and any(
                row.get("variant") == "StartupThreadStarted"
                for row in session_records(log)
            )
        ),
    )
    if case.worker:
        original = case.installed_worker()
        case.entry["worker_before_query"] = dict(original)
    draft = type_query(case, "p02alpha", ALPHA, ("p02alpha_ignored.rs", ROOT_B))
    if case.worker:
        require(
            case.installed_worker()["pid"] == original["pid"], "worker identity changed"
        )
    clear_draft(case, draft)
    if causal:
        case.pause_worker()
        try:
            case.write(b"@p02beta", label="type-fresh-query-while-worker-stopped")
            case.wait("stopped-query-draft", lambda: "@p02beta" in case.screen.text())
            case.hold(
                0.6,
                lambda: BETA not in case.screen.text(),
                "fresh result appeared while selected worker was stopped",
            )
            case.capture("stopped-worker-no-result")
        finally:
            case.resume_workers()
        case.wait("resumed-worker-result", lambda: BETA in case.screen.text())
        require(
            case.installed_worker()["pid"] == original["pid"],
            "fresh query replaced selected worker",
        )
        case.entry["causal_worker_pause_query"] = {
            "hold_seconds": 0.6,
            "fresh_filename": BETA,
            "no_result_while_stopped": True,
            "result_after_resume": True,
            "same_worker": True,
        }
        draft = "@p02beta"
    else:
        draft = type_query(case, "p02beta", BETA)
    clear_draft(case, draft)
    # /cd is an actual supported TUI command. Wait for its completion message;
    # do not infer a root transition from the typed command or directory header.
    command = "/cd " + str(root_b)
    case.write(command.encode() + b"\r", label="normal-slash-cd")
    case.wait(
        "working-directory-changed",
        lambda: (
            "Working directory changed to:" in case.screen.text()
            and root_b.name in case.screen.text()
        ),
        seconds=60,
    )
    draft = type_query(case, "p02root", ROOT_B, (ROOT_A,))
    if case.worker:
        require(
            case.installed_worker()["pid"] == original["pid"],
            "/cd selected another process instead of retaining the shared provider",
        )
        case.entry["same_worker_after_cd"] = True
    clear_draft(case, draft)
    case.write(b"/quit\r", label="normal-slash-quit")
    case.await_exit(0)
    records = session_records(log)
    require(
        not any(
            row.get("dir") == "from_tui"
            and row.get("kind") == "op"
            and isinstance(row.get("payload"), dict)
            and "UserTurn" in row["payload"]
            for row in records
        ),
        "PTY keyboard sequence accidentally submitted a model turn",
    )
    case.entry["no_user_turn_in_recorded_commands"] = True
    require(
        any(
            row.get("kind") == "file_search_start" and row.get("query") == "p02alpha"
            for row in records
        ),
        "real TUI query admission was not recorded",
    )
    require(
        any(row.get("kind") == "file_search_ready" for row in records),
        "real mailbox delivery was not recorded",
    )
    require(
        any(row.get("kind") == "session_end" for row in records),
        "normal TUI session end was not recorded",
    )
    case.entry["session_log_fingerprint"] = fingerprint(log)


def run_tui(runner, label, cli, home, roots, worker=None, failure=False):
    from pty_case import PtyCase

    log = runner.directory / (label + ".session.jsonl")
    case = PtyCase(
        runner,
        label,
        [cli, "--no-daemon", "--no-alt-screen", "--cd", roots[0]],
        roots[0],
        environment(home, log),
        worker,
    )
    error = None
    try:
        if failure:
            case.await_exit("nonzero", seconds=60)
            raw = case.transcript.read_bytes().decode("utf-8", errors="replace")
            require(
                "unsupported field or type" in raw,
                "selected native worker rejection cause missing",
            )
            require(
                ALPHA not in raw and BETA not in raw,
                "selected failure produced successful filenames",
            )
            require(
                not any(
                    row.get("variant") == "StartupThreadStarted"
                    for row in session_records(log)
                ),
                "selected failure unexpectedly reached interactive session",
            )
            case.capture("selected-startup-rejection")
        else:
            exercise(case, roots[1], log, causal=worker is not None)
    except BaseException as caught:
        error = caught
        case.capture("failure")
        raise
    finally:
        case.finish(error)
    if worker is None:
        require(
            not any(
                Path(item.get("executable", "")).name
                == "codex-file-search-local-plugin"
                for item in case.tracked.values()
            ),
            "unselected/native case unexpectedly started external worker",
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument(
        "--cli",
        type=Path,
        required=True,
        help="frozen FULL codex binary, not codex-file-search",
    )
    parser.add_argument("--cli-sha256", required=True)
    parser.add_argument(
        "--host-source-report",
        type=Path,
        required=True,
        help="root full-CLI successful build source manifest/evidence",
    )
    parser.add_argument(
        "--host-build-binding",
        type=Path,
        required=True,
        help="root frozen-artifact envelope binding this CLI to that successful build",
    )
    parser.add_argument("--manager", type=Path, required=True)
    parser.add_argument("--manager-sha256", required=True)
    parser.add_argument(
        "--build-report",
        type=Path,
        required=True,
        help="successful independent03 worker build proof",
    )
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()
    repo = args.repo.resolve(strict=True)
    support, support_path = load_support(repo)
    global require, fingerprint
    require, fingerprint = support.require, support.fingerprint
    cli, manager = args.cli.resolve(strict=True), args.manager.resolve(strict=True)
    before = {"cli": fingerprint(cli), "manager": fingerprint(manager)}
    require(
        before["cli"]["sha256"] == args.cli_sha256
        and before["manager"]["sha256"] == args.manager_sha256,
        "supplied frozen host hash mismatch",
    )
    host_source = args.host_source_report.resolve(strict=True)
    binding_path = args.host_build_binding.resolve(strict=True)
    binding = json.loads(binding_path.read_text())
    require(
        binding.get("schema_version") == 1
        and binding.get("artifact_kind") == "codex-cli",
        "full CLI build binding is missing or wrong kind",
    )
    require(
        Path(binding["binary"]["path"]).resolve(strict=True) == cli
        and binding["binary"]["sha256"] == before["cli"]["sha256"],
        "build binding names a different frozen executable",
    )
    require(
        Path(binding["source_report"]["path"]).resolve(strict=True) == host_source
        and binding["source_report"]["sha256"] == fingerprint(host_source)["sha256"],
        "build binding names a different source report",
    )
    built = json.loads(host_source.read_text())
    require(
        built.get("status") == "finished"
        and built.get("returncode") == 0
        and built.get("scoped_source_unchanged") is True,
        "full CLI build was not successful on unchanged source",
    )
    require(
        built.get("source_before")
        and built["source_before"] == built.get("source_after"),
        "full CLI build source fingerprints differ",
    )
    require(
        "build" in built.get("command", [])
        and any(
            built["command"][i : i + 2] == ["-p", "codex-cli"]
            for i in range(len(built["command"]))
        ),
        "source evidence did not build the full codex-cli package",
    )
    for path in (
        "codex-rs/tui/src/file_search.rs",
        "codex-rs/tui/src/file_search_startup.rs",
        "codex-rs/app-server/src/file_search_services.rs",
    ):
        require(
            path in built["source_before"],
            "full CLI build evidence omitted consumer composition source: " + path,
        )
    proof_path = args.build_report.resolve(strict=True)
    proof = json.loads(proof_path.read_text())
    require(
        proof.get("passed") is True
        and proof.get("evidence_kind") == "independent_source_build",
        "worker input is not verified independent-source evidence",
    )
    require(
        proof["build_artifact"]["separate_initially_empty_target"]
        and proof["build_artifact"]["fresh"] is False,
        "worker was not actually compiled separately",
    )
    require(
        proof["source_resolution"]["all_local_paths_inside_export"]
        and proof["source_resolution"]["lock_new_identities"] == 0,
        "worker source/lock provenance failed",
    )
    require(
        proof["source_parked_before_runtime"]
        and not (Path(proof["artifact_directory"]) / "source").exists(),
        "worker source was not parked",
    )
    require(
        fingerprint(Path(proof["artifact_directory"]) / "source-inventory.json")
        == proof["exported_source_inventory"],
        "worker source inventory changed",
    )
    require(
        proof["frozen_binaries_before"] == proof["frozen_binaries_after"],
        "original independent-build host freeze failed",
    )
    # That proof's CLI is the older standalone search executable. This gate's
    # later full CLI has its own explicit hash/source-report; never relabel them.
    package = Path(proof["package_path"]).resolve(strict=True)
    package_files = support.inventory(package)
    require(package_files == proof["package_files"], "verified package changed")
    work = args.work_dir.resolve()
    require(
        not work.exists() and work != repo and repo not in work.parents,
        "work must be new and outside checkout",
    )
    work.mkdir(parents=True, mode=0o700)
    report_path = work / "tui-acceptance.json"
    report = {
        "passed": False,
        "evidence_kind": "real_full_cli_tui_independent_search",
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "commands": [],
        "pty_cases": [],
        "binaries_before": before,
        "host_source_report": str(host_source),
        "host_source_report_fingerprint": fingerprint(host_source),
        "host_build_binding": str(binding_path),
        "host_build_binding_fingerprint": fingerprint(binding_path),
        "worker_build_report": str(proof_path),
        "worker_build_report_fingerprint": fingerprint(proof_path),
        "original_worker_build_frozen_binaries": proof["frozen_binaries_before"],
        "tooling": {p.name: fingerprint(p) for p in Path(__file__).parent.glob("*.py")},
        "reused_sdk_support": {
            "path": str(support_path),
            "fingerprint": fingerprint(support_path),
        },
        "limitations": [
            "No remote transport reconnect/daemon/shared-socket gate.",
            "No model turn, auth/network/model-stream, desktop GUI or Browser proof.",
            "Same worker identity plus pause/resume query dependence supports shared selection; does not inspect in-memory provider identity.",
            "Subreaper report must independently pass; this report alone is insufficient.",
        ],
    }
    runner = support.Runner(work, report, report_path)
    home = work / "home"
    home.mkdir()
    roots = [work / "root_a", work / "root_b"]
    for root in roots:
        root.mkdir()
        (root / ".git").mkdir()
    for name in (ALPHA, BETA, ROOT_A, "p02alpha_ignored.rs"):
        (roots[0] / name).write_text("real picker fixture\n")
    (roots[0] / ".gitignore").write_text("p02alpha_ignored.rs\n")
    (roots[1] / ROOT_B).write_text("second root fixture\n")
    config(home, roots)
    management = [manager, "--codex-home", home]
    env = environment(home, work / "manager-unused-log.jsonl")
    try:
        report["fixture_files"] = {str(root): support.inventory(root) for root in roots}
        report["config_fingerprint"] = fingerprint(home / "config.toml")
        run_tui(runner, "native", cli, home, roots)
        source_package = work / "install-input"
        shutil.copytree(package, source_package)
        require(
            support.same_content(support.inventory(source_package), package_files),
            "install-input copy differs",
        )
        runner.run(
            "install",
            [*management, "install", source_package],
            cwd=work,
            environment=env,
        )
        runner.run(
            "select",
            [*management, "select", "file_search", "default", PLUGIN],
            cwd=work,
            environment=env,
        )
        output, _, _ = runner.run(
            "selected-list", [*management, "list"], cwd=work, environment=env
        )
        settings = json.loads(output)
        require(
            settings["selections"].get("file_search:default") == PLUGIN,
            "manager did not select file_search/default",
        )
        installed = home / "components/objects" / settings["installed"][PLUGIN]
        require(
            support.same_content(support.inventory(installed), package_files),
            "immutable installed content differs",
        )
        manifest = json.loads((installed / "codex-component.json").read_text())
        require(
            manifest["id"] == PLUGIN
            and any(
                item["kind"] == "file_search" and item["contract_version"] == 1
                for item in manifest["components"]
            ),
            "invalid installed manifest",
        )
        worker = (installed / manifest["entrypoint"]).resolve(strict=True)
        require(installed in worker.parents, "worker escapes installed object")
        installed_before = support.inventory(installed)
        report["installed_worker"] = {
            "path": str(worker),
            "fingerprint": fingerprint(worker),
        }
        source_package.rename(work / "install-input.parked")
        require(
            {"cli": fingerprint(cli), "manager": fingerprint(manager)} == before,
            "host changed during install",
        )
        run_tui(runner, "installed", cli, home, roots, worker=worker)
        settings_path = home / "components/config.json"
        good = settings_path.read_text()
        changed = json.loads(good)
        changed.setdefault("config", {})[PLUGIN] = {
            "unsupported_acceptance_option": True
        }
        settings_path.write_text(json.dumps(changed, indent=2) + "\n")
        try:
            run_tui(
                runner,
                "selected-failure",
                cli,
                home,
                roots,
                worker=worker,
                failure=True,
            )
        finally:
            settings_path.write_text(good)
        runner.run("remove", [*management, "remove", PLUGIN], cwd=work, environment=env)
        output, _, _ = runner.run(
            "removed-list", [*management, "list"], cwd=work, environment=env
        )
        settings = json.loads(output)
        require(
            PLUGIN not in settings["installed"]
            and PLUGIN not in settings["enabled"]
            and "file_search:default" not in settings["selections"],
            "removal left activation behind",
        )
        run_tui(runner, "removed-native", cli, home, roots)
        require(
            support.inventory(installed) == installed_before,
            "installed immutable object changed",
        )
        require(
            support.inventory(package) == package_files,
            "original independent package changed",
        )
        report["binaries_after"] = {
            "cli": fingerprint(cli),
            "manager": fingerprint(manager),
        }
        require(
            report["binaries_after"] == before,
            "frozen host changed during runtime acceptance",
        )
        require(
            fingerprint(host_source) == report["host_source_report_fingerprint"],
            "full CLI source evidence changed during acceptance",
        )
        require(
            fingerprint(binding_path) == report["host_build_binding_fingerprint"],
            "full CLI artifact binding changed during acceptance",
        )
        require(
            fingerprint(proof_path) == report["worker_build_report_fingerprint"],
            "worker build proof changed during acceptance",
        )
        report["passed"] = True
    except BaseException as error:
        report["failure"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        support.write_report(report_path, report)
    print(
        json.dumps(
            {
                "passed": True,
                "report": str(report_path),
                "pty_cases": len(report["pty_cases"]),
            }
        )
    )


if __name__ == "__main__":
    main()
