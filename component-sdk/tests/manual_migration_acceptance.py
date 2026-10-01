"""Use an independently built storage worker to migrate and resume a real session.

Run native_storage_acceptance.py first. This adds the P01 migration-to-engine
check to that helper's source export/build/install proof. Inference uses the
existing deterministic GUI model fixture; Codex, App Server and storage are real.
The prepared desktop home is retained for the separate browser/manager regression.
This script does not build Rust, use a live model, or claim a browser check ran.
"""

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time
from unittest.mock import patch

from harness_acceptance import SDK, fingerprint

sys.path.insert(0, str(SDK / "examples" / "desktop"))
from desktop_ui.gateway import Bridge


class LegacyBridge(Bridge):
    """Opt into the pinned App Server's explicit legacy-history test input."""

    def rpc(self, method, params, timeout=90):
        if method == "initialize":
            params = dict(params, capabilities={"experimentalApi": True})
        return super().rpc(method, params, timeout)


def result(bridge, method, params):
    response = bridge.rpc(method, params)
    assert "error" not in response, response
    return response["result"]


def completed(bridge, thread_id, turn_id):
    def terminal():
        for _, encoded in bridge.events:
            event = json.loads(encoded)
            params = event.get("params", {})
            if (
                event.get("method") == "turn/completed"
                and params.get("threadId") == thread_id
                and params.get("turn", {}).get("id") == turn_id
            ):
                return params["turn"]
        return None

    with bridge.condition:
        assert bridge.condition.wait_for(terminal, 60), "turn did not complete"
        return terminal()


def storage_snapshot(home):
    # Config/telemetry/component registry state is outside this storage-byte check.
    paths = []
    for name in (
        "sessions",
        "archived_sessions",
        "shared_histories",
        "rollout-migrations",
    ):
        paths.extend(path for path in (home / name).rglob("*") if path.is_file())
    for name in ("state_", "thread_history_", "goals_", "memories_", "queue_"):
        paths.extend(path for path in home.glob(name + "*.sqlite*") if path.is_file())
    return {str(path.relative_to(home)): fingerprint(path) for path in sorted(paths)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--codex", type=Path, required=True)
    parser.add_argument("--independent-report", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.error("this acceptance records Linux /proc process-reaping evidence")
    host, codex = args.host.resolve(strict=True), args.codex.resolve(strict=True)
    work = args.work_dir.resolve()
    if work.exists() or work == SDK.parent or SDK.parent in work.parents:
        parser.error("work directory must be new and outside the source checkout")
    independent_path = args.independent_report.resolve(strict=True)
    independent = json.loads(independent_path.read_text())
    assert independent["passed"] is True and independent["contract_version"] == 2
    assert independent["source_removed_before_install"] is True
    package = Path(independent["artifact_directory"]) / "package"
    assert not (package.parent / "source").exists()
    before = {"host": fingerprint(host), "codex": fingerprint(codex)}
    assert before == independent["binaries_before"] == independent["binaries_after"]
    for name, expected in independent["package_files"].items():
        assert fingerprint(package / name) == expected, name
    manifest = json.loads((package / "codex-component.json").read_text())
    storage_id = manifest["id"]
    work.mkdir(parents=True)
    gui = work / "gui"
    home, project = gui / "codex-home", gui / "project"
    report = {
        "passed": False,
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "independent_report": str(independent_path),
        "independent_report_fingerprint": fingerprint(independent_path),
        "binaries_before": before,
        "inference": "deterministic external GUI fixture, not a live provider",
        "commands": [],
        "storage_processes": {},
        "browser_manager_regression": "pending; use the retained GUI fixture and runbook",
    }
    stopped = threading.Event()

    def monitor():
        prefix = str(home / "components" / "objects") + os.sep
        while not stopped.is_set():
            for process in Path("/proc").glob("[0-9]*"):
                try:
                    executable = os.readlink(process / "exe")
                    if executable.startswith(prefix):
                        report["storage_processes"][int(process.name)] = executable
                except OSError:
                    pass
            stopped.wait(0.005)

    def reaped():
        deadline = time.monotonic() + 5
        while any(
            Path(f"/proc/{pid}").exists() for pid in list(report["storage_processes"])
        ):
            assert time.monotonic() < deadline, (
                "storage process remains alive or unreaped"
            )
            time.sleep(0.025)

    def run(label, command, *, cwd=None):
        prior_processes = set(report["storage_processes"])
        stdout, stderr = work / f"{label}.stdout", work / f"{label}.stderr"
        environment = dict(
            os.environ, CODEX_HOME=str(home), CODEX_SQLITE_HOME=str(home)
        )
        entry = {
            "label": label,
            "argv": list(map(str, command)),
            "stdout": str(stdout),
            "stderr": str(stderr),
        }
        report["commands"].append(entry)
        with stdout.open("w") as out, stderr.open("w") as err:
            process = subprocess.Popen(
                entry["argv"],
                cwd=cwd or SDK.parent,
                env=environment,
                stdin=subprocess.DEVNULL,
                stdout=out,
                stderr=err,
                start_new_session=True,
            )
            try:
                status = process.wait(timeout=240)
            except subprocess.TimeoutExpired:
                process.send_signal(signal.SIGINT)
                try:
                    process.wait(timeout=220)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                    entry["cleanup"] = "forced; durability unknown"
                raise RuntimeError(f"{label} exceeded acceptance deadline") from None
        entry["status"] = status
        entry["storage_pids"] = sorted(
            set(report["storage_processes"]) - prior_processes
        )
        assert status == 0, f"{label} failed; see {stderr}"
        if label in (
            "selected-dry-run",
            "selected-apply",
            "selected-repeated",
            "cli-resume",
        ):
            assert entry["storage_pids"], f"{label}: selected worker was not observed"
        reaped()
        return stdout.read_text()

    @contextmanager
    def engine():
        with patch.dict(os.environ, {"CODEX_SQLITE_HOME": str(home)}):
            bridge = LegacyBridge(str(codex), str(home), str(project))
        try:
            yield bridge
        finally:
            bridge.close()
            assert not Path(f"/proc/{bridge.child.pid}").exists()
            reaped()

    watcher = threading.Thread(target=monitor, daemon=True)
    watcher.start()
    try:
        run(
            "prepare-gui",
            [
                sys.executable,
                SDK / "tests/gui_harness_fixture.py",
                "--host",
                host,
                "--codex",
                codex,
                "--work-dir",
                gui,
            ],
        )
        config = home / "config.toml"
        config.write_text(
            config.read_text().replace(
                "[features]\n",
                "[features]\nbackground_paginated_rollout_migration = false\nlocal_thread_store_compression = false\n",
            )
        )
        manager = [host, "--codex-home", home]
        run("install-native", manager + ["install", package])
        run(
            "select-native", manager + ["select", "thread_store", "default", storage_id]
        )
        seed_prompt = "Preserve this real legacy conversation through manual migration."
        with engine() as bridge:
            thread = result(
                bridge,
                "thread/start",
                {"cwd": str(project), "ephemeral": False, "historyMode": "legacy"},
            )["thread"]
            assert thread["historyMode"] == "legacy", thread
            thread_id = thread["id"]
            turn = result(
                bridge,
                "turn/start",
                {
                    "threadId": thread_id,
                    "input": [
                        {"type": "text", "text": seed_prompt, "text_elements": []}
                    ],
                },
            )["turn"]
            assert completed(bridge, thread_id, turn["id"])["status"] == "completed"
        report["thread_id"] = thread_id
        assert report["storage_processes"], "selected storage process was not observed"
        original = storage_snapshot(home)
        migration = [codex, "migrate-rollouts", "--json", "--thread", thread_id]
        selected = json.loads(run("selected-dry-run", migration, cwd=project))
        assert [item["status"] for item in selected["outcomes"]] == ["eligible"], (
            selected
        )
        assert storage_snapshot(home) == original, "selected dry-run mutated storage"
        run("reset-native", manager + ["reset", "thread_store", "default"])
        native = json.loads(run("native-dry-run", migration, cwd=project))
        assert native == selected
        assert storage_snapshot(home) == original, "native dry-run mutated storage"
        run(
            "reselect-native",
            manager + ["select", "thread_store", "default", storage_id],
        )
        applied = json.loads(
            run("selected-apply", migration + ["--apply"], cwd=project)
        )
        assert [item["status"] for item in applied["outcomes"]] == ["migrated"], applied
        repeated = json.loads(
            run("selected-repeated", migration + ["--apply"], cwd=project)
        )
        assert [item["status"] for item in repeated["outcomes"]] == [
            "already_paginated"
        ], repeated
        resumed = run(
            "cli-resume",
            [
                codex,
                "exec",
                "resume",
                "--skip-git-repo-check",
                "--json",
                thread_id,
                "Continue the migrated acceptance conversation.",
            ],
            cwd=project,
        )
        assert any(
            json.loads(line).get("type") == "turn.completed"
            for line in resumed.splitlines()
        )
        state = home / "components" / "state" / "test.gui-model"
        requests = [
            json.loads(path.read_text()) for path in state.glob("request-*.json")
        ]
        assert any(
            seed_prompt in json.dumps(item["request"]["input"])
            and "GUI fixture response: " + seed_prompt
            in json.dumps(item["request"]["input"])
            for item in requests
        ), "cold CLI resume lost legacy turn input/output"
        with engine() as bridge:
            # `exec resume` intentionally persists its noninteractive Never policy.
            # Restore this fixture's on-request policy through the public API so
            # the later GUI approval check actually requires a user decision.
            recovered = result(
                bridge,
                "thread/resume",
                {
                    "threadId": thread_id,
                    "excludeTurns": True,
                    "approvalPolicy": "on-request",
                },
            )
            assert recovered["approvalPolicy"] == "on-request"
            report["gui_approval_policy"] = {
                "value": recovered["approvalPolicy"],
                "set_by": "public thread/resume after noninteractive CLI resume",
            }
            assert recovered["thread"]["historyMode"] == "paginated"
            history = result(
                bridge,
                "thread/turns/list",
                {
                    "threadId": thread_id,
                    "limit": 20,
                    "sortDirection": "desc",
                    "itemsView": "full",
                },
            )
            assert seed_prompt in json.dumps(history)
            turn = result(
                bridge,
                "turn/start",
                {
                    "threadId": thread_id,
                    "input": [
                        {
                            "type": "text",
                            "text": "Exercise cancel after migration",
                            "text_elements": [],
                        }
                    ],
                },
            )["turn"]
            blocked = state / f"blocked-{thread_id}.json"
            deadline = time.monotonic() + 60
            while not blocked.exists():
                assert time.monotonic() < deadline, "model never entered blocked stream"
                time.sleep(0.025)
            model_pid = json.loads(blocked.read_text())["pid"]
            result(
                bridge, "turn/interrupt", {"threadId": thread_id, "turnId": turn["id"]}
            )
            assert completed(bridge, thread_id, turn["id"])["status"] == "interrupted"
        assert not Path(f"/proc/{model_pid}").exists(), (
            "cancelled model remained alive or unreaped"
        )
        report["verified"] = [
            "matching independent-source package and unchanged host",
            "real legacy App Server conversation",
            "native/selected dry-run parity and unchanged storage bytes",
            "selected CLI migration and idempotent repeated apply",
            "cold CLI resume retains user/assistant history",
            "cold App Server paginated history",
            "post-migration turn cancellation and process reaping",
        ]
        report["gui_fixture"] = str(gui / "fixture-ready.json")
        report["passed"] = True
    finally:
        stopped.set()
        watcher.join()
        report["binaries_after"] = {
            "host": fingerprint(host),
            "codex": fingerprint(codex),
        }
        report["host_unchanged"] = before == report["binaries_after"]
        report["passed"] = report["passed"] and report["host_unchanged"]
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        (work / "manual-migration-report.json").write_text(
            json.dumps(report, indent=2) + "\n"
        )
    assert report["passed"], "acceptance failed; inspect retained report"
    print(work / "manual-migration-report.json")


if __name__ == "__main__":
    main()
