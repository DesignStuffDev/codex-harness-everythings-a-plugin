"""Linux real Launch/GUI accepted-at-fixture slow-storage acceptance.

Requires a NEW exclusively assigned manual-migration fixture. No Rust builds.
Always run beneath the unchanged strict five-second subreaper runner.
"""

import argparse
import ctypes
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
from uuid import uuid4


def require(value, message):
    if not value:
        raise AssertionError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("sdk-tests", "host", "codex", "independent-report", "manual-migration-report", "work-dir", "source-receipt"):
        parser.add_argument("--" + name, type=Path, required=True)
    for name in ("host-sha256", "codex-sha256", "source-receipt-sha256"):
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--mode", choices=("normal", "forced"), required=True)
    args = parser.parse_args()
    require(sys.platform == "linux" and hasattr(signal, "pidfd_send_signal"), "Linux pidfds required")
    sdk_tests = args.sdk_tests.resolve(strict=True)
    sys.path.insert(0, str(sdk_tests))
    from harness_acceptance import fingerprint
    from p01_gui_acceptance import Browser, OwnedProcess, completed, screenshot, send_signal, stop_manager, submit, wait_for
    from reused_storage_acceptance import original_proof, package_fingerprints

    strict = sdk_tests / "subreaper_runner.py"
    require(fingerprint(strict)["sha256"] == "fe01097ae1741cbcb76e15fb07c0c936108a4dc35c08b4264c1caa9eb1e08875", "strict subreaper runner changed; root review required")

    host, codex = args.host.resolve(strict=True), args.codex.resolve(strict=True)
    source_receipt = args.source_receipt.resolve(strict=True)
    before = {"host": fingerprint(host), "codex": fingerprint(codex)}
    require(before["host"]["sha256"] == args.host_sha256 and before["codex"]["sha256"] == args.codex_sha256, "reviewed binary hashes differ")
    require(fingerprint(source_receipt)["sha256"] == args.source_receipt_sha256, "source/build receipt changed")
    original_path = args.independent_report.resolve(strict=True)
    original_before = fingerprint(original_path)
    original, package, native_manifest = original_proof(original_path)
    manual_path = args.manual_migration_report.resolve(strict=True)
    manual_before = fingerprint(manual_path)
    manual = json.loads(manual_path.read_text())
    require(manual["passed"] and manual["host_unchanged"] and manual["binaries_before"] == before == manual["binaries_after"], "fresh migration fixture must match actual binaries")
    reuse_path = Path(manual["independent_report"])
    require(fingerprint(reuse_path) == manual["independent_report_fingerprint"], "reuse evidence changed")
    reuse = json.loads(reuse_path.read_text())
    require(reuse["passed"] and reuse["binaries_after"] == before and reuse["package_files"] == original["package_files"], "reuse evidence does not bind package and new host")
    if "source_build_report" in reuse:
        require(Path(reuse["source_build_report"]).resolve() == original_path and reuse["source_build_report_fingerprint"] == original_before, "wrong independent build lineage")
    fixture = json.loads(Path(manual["gui_fixture"]).read_text())
    home, project = Path(fixture["codex_home"]), Path(fixture["cwd"])
    thread_id = manual["thread_id"]
    require(manual_path.parent in home.parents and manual_path.parent in project.parents, "fixture must belong to supplied migration run")
    work = args.work_dir.resolve()
    require(not os.path.lexists(work) and not any((p / ".git").exists() for p in [work, *work.parents]), "work directory must be new and outside checkouts")
    os.umask(0o077)
    work.mkdir(parents=True, mode=0o700)
    # Reject prior slow-test reuse. Root must assign this fixture exclusively;
    # the ordinary GUI driver does not consult this specialized claim file.
    with (manual_path.parent / "slow-storage-fixture-claim.json").open("x") as stream:
        json.dump({"work_dir": str(work), "mode": args.mode}, stream)
    gate = work / "gate"
    gate.mkdir(mode=0o700)
    prompt = "Slow installed storage " + uuid4().hex
    owners, browsers = [], []
    report = {
        "passed": False, "mode": args.mode, "started_utc": datetime.now(timezone.utc).isoformat(),
        "admission_boundary": "installed_fixture_before_native_forward",
        "selected_history_event": "ItemCompleted(UserMessage)",
        "response_only_interrupted_history_gap_fixed": False,
        "native_internal_admission_before_release_proven": False,
        "browser": "Chromium/Playwright fallback; not in-app Browser",
        "inference": "existing deterministic installed fixture; no live provider",
        "binaries_before": before, "source_receipt": str(source_receipt),
        "source_receipt_fingerprint": fingerprint(source_receipt),
        "source_receipt_interpretation": "Root-reviewed build/source binding; this driver verifies immutable receipt and binary hashes, not its semantic correctness.",
        "independent_report": str(original_path), "independent_report_fingerprint": original_before,
        "source_removed_before_install": original["source_removed_before_install"],
        "manual_report": str(manual_path), "manual_report_fingerprint": manual_before,
        "script_fingerprints": {str(p): fingerprint(p) for p in [Path(__file__), Path(__file__).with_name("slow_store_relay.py"), sdk_tests / "p01_gui_acceptance.py", sdk_tests / "gui_browser_driver.cjs", sdk_tests / "reused_storage_acceptance.py", sdk_tests / "subreaper_runner.py"]},
        "commands": [], "processes": [], "tracking_limit": "sampled descendant identities can miss very short-lived children; unchanged external subreaper remains mandatory",
    }
    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
    libc.prctl.restype = ctypes.c_int
    require(libc.prctl(36, 1, 0, 0, 0) == 0, "cannot enable child subreaper")
    environment = dict(os.environ, CODEX_SQLITE_HOME=str(home), PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1")

    def command(label, argv):
        with (work / (label + ".stdout")).open("w") as stdout, (work / (label + ".stderr")).open("w") as stderr:
            result = subprocess.run(list(map(str, argv)), cwd=project, env=environment, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, timeout=60)
        report["commands"].append({"label": label, "argv": list(map(str, argv)), "status": result.returncode})
        require(result.returncode == 0, "component management failed; see private artifacts")

    def launch(cycle):
        manager = OwnedProcess([host, "--codex-home", home, "launch", "desktop", "--codex-bin", codex, "--port", "0"], project, work / (cycle + "-manager"), environment)
        owners.append(manager)
        def ready():
            require(manager.process.poll() is None, "Launch exited before readiness")
            try:
                return any(json.loads(line).get("method") == "presentation/ready" for line in manager.stdout_path.read_text().splitlines())
            except ValueError:
                return False
        wait_for(ready, "presentation ready", 125)
        browser = Browser(manager.stdout_path, project, work / (cycle + "-browser"), environment, browsers.append)
        browser.command("await page.waitForFunction(() => document.querySelector('#connection').classList.contains('connected')); return {connected:true};")
        selector = '.thread[data-thread-id=' + json.dumps(thread_id) + ']'
        browser.command(f"await page.locator({json.dumps(selector)}).click(); await page.waitForFunction(() => document.querySelector('#messages').textContent.includes('Preserve this real legacy conversation through manual migration.')); return {{history:true}};")
        return manager, browser

    try:
        settings_path = home / "components/config.json"
        settings = json.loads(settings_path.read_text())
        require(settings["selections"]["thread_store:default"] == native_manifest["id"], "native storage is not selected before fixture installation")
        installed = home / "components/objects"
        original_objects = {str(p.relative_to(installed)): fingerprint(p) for p in installed.rglob("*") if p.is_file()}
        fixture_package = work / "package"
        shutil.copytree(package, fixture_package)
        relay = fixture_package / "slow_store_relay.py"
        shutil.copy2(Path(__file__).with_name("slow_store_relay.py"), relay)
        relay.chmod(0o700)
        native_entry = native_manifest["entrypoint"]
        native_hash = original["package_files"][native_entry]["sha256"]
        (fixture_package / "relay-config.json").write_text(json.dumps({"native_entrypoint": native_entry, "native_sha256": native_hash, "gate_directory": str(gate), "prompt": prompt, "thread_id": thread_id}) + "\n")
        manifest = dict(native_manifest, id="fixture.slow-thread-store", entrypoint=relay.name, args=[], version="0.1.0")
        (fixture_package / "codex-component.json").write_text(json.dumps(manifest, indent=2) + "\n")
        package_before = package_fingerprints(fixture_package)
        manager_argv = [host, "--codex-home", home]
        command("install-relay", manager_argv + ["install", fixture_package])
        command("select-relay", manager_argv + ["select", "thread_store", "default", manifest["id"]])
        settings = json.loads(settings_path.read_text())
        installed_relay = installed / settings["installed"][manifest["id"]]
        require(fingerprint(installed_relay / native_entry)["sha256"] == native_hash, "installed native binary changed")
        report["fixture_package_files"] = package_before
        report["native_fingerprint"] = fingerprint(installed_relay / native_entry)
        manager, browser = launch("held")
        browser.command(f"await page.locator('#prompt').fill({json.dumps(prompt)}); await page.locator('#send').click(); return {{submitted:true}};")
        wait_for(lambda: (gate / "accepted.json").exists(), "installed fixture accepted target append", 60)
        accepted = json.loads((gate / "accepted.json").read_text())
        require(accepted["thread_id"] == thread_id and not (gate / "forwarded.json").exists(), "write admission evidence invalid")
        require(accepted.get("canonical_event") == "ItemCompleted(UserMessage)"
                and isinstance(accepted.get("turn_id"), str) and accepted["turn_id"]
                and isinstance(accepted.get("item_id"), str) and accepted["item_id"],
                "gate did not admit the exact canonical UI-history event")
        report["accepted_history_event"] = {key: accepted[key] for key in
                                            ("canonical_event", "turn_id", "item_id")}
        wait_for(lambda: any(p.get("executable") == str(installed_relay / native_entry) for p in manager.snapshot()), "installed native worker observed")
        signal_time = time.monotonic_ns()
        require(accepted["monotonic_ns"] <= signal_time, "gate must precede first SIGINT")
        send_signal(manager.root, signal.SIGINT)
        report["first_sigint_monotonic_ns"] = signal_time
        acknowledgment = "Stopping presentation; allowing up to"

        def shutdown_acknowledged():
            require(manager.process.poll() is None, "manager exited before first-interrupt acknowledgment")
            require(not (gate / "forwarded.json").exists(), "write escaped before first-interrupt acknowledgment")
            return acknowledgment in manager.stderr_path.read_text()

        wait_for(shutdown_acknowledged, "manager first-interrupt acknowledgment", 15)
        acknowledgment_time = time.monotonic_ns()
        report["first_sigint_acknowledgment_observed_monotonic_ns"] = acknowledgment_time
        report["first_sigint_acknowledgment"] = acknowledgment
        hold_seconds = 46 if args.mode == "normal" else 2
        deadline_ns = acknowledgment_time + hold_seconds * 1_000_000_000
        while time.monotonic_ns() < deadline_ns:
            require(manager.process.poll() is None and not (gate / "forwarded.json").exists(), "manager exited or write escaped gate during hold")
            time.sleep(0.025)
        if args.mode == "normal":
            (gate / "release").touch(exist_ok=False)
            status = manager.process.wait(timeout=170)
            require(status == 0, "normal Launch shutdown did not confirm cleanup")
            for name in ("accepted", "release_observed", "forwarded", "native_append_reply", "native_shutdown_complete", "finished"):
                report.setdefault("gate_receipts", {})[name] = json.loads((gate / (name + ".json")).read_text())
            require(report["gate_receipts"]["release_observed"]["monotonic_ns"] - acknowledgment_time > 45_000_000_000, "hold after acknowledged shutdown did not exceed old 45-second deadline")
            require(report["gate_receipts"]["finished"]["held_append_acknowledged"], "native append was never acknowledged")
            require(report["gate_receipts"]["finished"]["all_directional_results_observed"], "relay did not observe every directional task result")
            report["durability_classification"] = "native_shutdown_confirmed_pending_cold_recovery"
        else:
            send_signal(manager.root, signal.SIGINT)
            status = manager.process.wait(timeout=20)
            require(status != 0, "forced Launch shutdown incorrectly reported success")
            require("durability is unknown" in manager.stderr_path.read_text(), "forced shutdown omitted durability uncertainty")
            require(not (gate / "forwarded.json").exists(), "forced fixture write unexpectedly forwarded")
            report["durability_classification"] = "forced_shutdown_explicitly_unknown"
        report["manager_status"] = status
        report["shutdown_seconds"] = (time.monotonic_ns() - signal_time) / 1e9
        manager.reap_and_check()  # Includes zombies, not merely running PIDs.
        report["processes"].append(manager.snapshot())
        browser.close()
        report["held_runtime_all_tracked_processes_absent"] = True
        command("restore-native-selection", manager_argv + ["select", "thread_store", "default", native_manifest["id"]])
        command("remove-relay", manager_argv + ["remove", manifest["id"]])
        recovered, cold = launch("cold-native")
        visible = cold.command(f"return {{present:(await page.locator('#messages').innerText()).includes({json.dumps(prompt)})}};")["present"]
        require(visible == (args.mode == "normal"), "cold recovered history differs from observed gate outcome")
        report["cold_recovered_held_prompt"] = visible
        continuation = "Continue after slow installed storage " + uuid4().hex
        completed(cold, submit(cold, continuation), "GUI fixture response: " + continuation)
        screenshot(cold, work / "cold-recovery.png")
        report["cold_shutdown"] = {}
        stop_manager(recovered, report["cold_shutdown"])
        cold.close()
        report["processes"].append(recovered.snapshot())
        require(package_fingerprints(fixture_package) == package_before, "fixture package mutated")
        require(package_fingerprints(package) == original["package_files"], "independent native package mutated")
        require(all(fingerprint(installed / name) == expected for name, expected in original_objects.items()), "pre-existing installed package changed")
        require(fingerprint(original_path) == original_before and fingerprint(manual_path) == manual_before, "input evidence changed")
        report["binaries_after"] = {"host": fingerprint(host), "codex": fingerprint(codex)}
        require(before == report["binaries_after"], "host binaries changed")
        if args.mode == "normal":
            report["durability_classification"] = "native_cleanup_and_cold_recovery_confirmed"
        report["passed"] = True
    except BaseException as error:
        report["error_type"] = type(error).__name__
        report["error"] = str(error)
    finally:
        for browser in browsers:
            try:
                if browser.owner.process.poll() is None:
                    browser.close()
                browser.owner.reap_and_check()
            except BaseException:
                report.setdefault("emergency_cleanup", []).append("browser")
                try:
                    browser.owner.emergency_cleanup()
                except BaseException:
                    report.setdefault("cleanup_errors", []).append("browser reap unconfirmed")
            browser.reader.join(timeout=5)
            if browser.reader.is_alive():
                report.setdefault("emergency_cleanup", []).append("unjoined browser reader")
            browser.owner.finish()
        for owner in owners:
            try:
                if owner.process.poll() is None:
                    stop_manager(owner, {})
                owner.reap_and_check()
            except BaseException:
                report.setdefault("emergency_cleanup", []).append("manager")
                try:
                    owner.emergency_cleanup()
                except BaseException:
                    report.setdefault("cleanup_errors", []).append("manager reap unconfirmed")
            owner.finish()
        report["passed"] = report["passed"] and not report.get("emergency_cleanup")
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        (work / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    require(report["passed"], "acceptance failed; inspect private report")
    print(work / "report.json")


if __name__ == "__main__":
    main()
