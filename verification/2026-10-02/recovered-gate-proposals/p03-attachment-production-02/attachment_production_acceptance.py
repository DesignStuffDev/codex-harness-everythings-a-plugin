#!/usr/bin/env python3
"""Fresh real-CLI image acceptance; independent native reuse and custom cases differ."""
import argparse
import collections
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import sys
import zlib

HERE = Path(__file__).resolve().parent
MODEL = "test.attachment-model-probe"
CUSTOM = "test.attachment-counted"
NATIVE = "codex.attachment-inline"
MARKER = b"failed to upload prepared image"
MAX_LOG = 8 * 1024 * 1024


class GateError(Exception):
    pass


def require(value, code):
    if not value:
        raise GateError(code)


def fingerprint(path):
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"sha256": digest, "bytes": path.stat().st_size}


def still_matches(path, expected):
    try:
        return fingerprint(path) == expected
    except OSError:
        return False


def same_sha(path, digest):
    try:
        return fingerprint(path)["sha256"] == digest
    except OSError:
        return False


def read_json(path):
    require(path.is_file() and not path.is_symlink(), "expected_regular_json_file")
    require(path.stat().st_size <= MAX_LOG, "json_size_limit")
    return json.loads(path.read_bytes())


def write_json(path, data):
    with path.open("x") as output:
        json.dump(data, output, indent=2, sort_keys=True)
        output.write("\n")


def inventory(path):
    result = {}
    for file in sorted(path.rglob("*")):
        require(not file.is_symlink(), "package_symlink")
        if file.is_file():
            result[str(file.relative_to(path))] = fingerprint(file)
    return result


def image_bytes():
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    scanline = b"\x00" + bytes((12, 34, 56, 255)) * 4
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 4, 4, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(scanline * 4)) + chunk(b"IEND", b"")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("repo", "host", "codex", "native-report", "work-dir"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--strace", type=Path, default=Path("/usr/bin/strace"))
    args = parser.parse_args()
    os.umask(0o077)
    repo, host, codex = args.repo.resolve(strict=True), args.host.resolve(strict=True), args.codex.resolve(strict=True)
    work = args.work_dir.resolve()
    require(not work.exists() and work != repo and repo not in work.parents, "fresh_external_work_required")
    pins = read_json(HERE / "INPUTS.json")
    require(fingerprint(host)["sha256"] == pins["manager_sha256"], "manager_pin")
    require(fingerprint(codex)["sha256"] == pins["CLI_sha256"], "CLI_pin")
    require(fingerprint(args.native_report)["sha256"] == pins["native_report_sha256"], "native_report_pin")
    for name, digest in pins["repo_source_sha256"].items():
        require(fingerprint(repo / name)["sha256"] == digest, "helper_source_pin")
    historical = read_json(args.native_report)
    require(historical["passed"] is True and historical["source_removed_before_install_and_execution"] is True
            and historical["exported_source_paths_self_contained"] is True, "independent_native_proof")
    package = Path(historical["package_directory"]).resolve(strict=True)
    expected = {entry["path"]: {key: entry[key] for key in ("sha256", "bytes")} for entry in historical["package_tree"]}
    require(inventory(package) == expected, "native_package_proof_changed")
    native_manifest = read_json(package / "codex-component.json")
    require(native_manifest["id"] == NATIVE and Path(native_manifest["entrypoint"]).name == native_manifest["entrypoint"], "native_manifest")
    work.mkdir(parents=True, exist_ok=False)
    home, project = work / "home", work / "project"
    home.mkdir(); project.mkdir()
    sdk = repo / "component-sdk"
    sys.path.insert(0, str(sdk / "tests"))
    from p01_gui_acceptance import OwnedProcess
    from trace_native_upload import TraceError, inspect_trace
    environment = dict(os.environ, HOME=str(home), CODEX_HOME=str(home), CODEX_SQLITE_HOME=str(home),
                       PYTHONPATH="", RUST_LOG="error,codex_core::image_preparation=warn")
    before = {"CLI": fingerprint(codex), "manager": fingerprint(host)}
    external_before = {str(path): fingerprint(path) for path in
                       [args.native_report, HERE / "INPUTS.json", HERE / "MANIFEST.json", *sorted(HERE.glob("*.py"))]}
    report = {"schema": "current-attachment-production-acceptance-v1", "passed": False,
              "native_package_build": "reused_exact_historical_independent_package_no_new_build",
              "inference": "deterministic_installed_model_plugin_no_live_provider", "commands": [], "cases": {},
              "host_before": before, "native_report_sha256": fingerprint(args.native_report)["sha256"],
              "native_package_before": expected, "resolve_production_caller_exercised": False,
              "raw_trace_private": True, "native_result_envelope_inspected": False, "whole_host_clean": False}

    def run(label, command, build=False):
        prefix = work / ("command-%02d" % len(report["commands"]))
        strict_path = prefix.with_suffix(".strict.json")
        require(not any(prefix.with_suffix(ext).exists() for ext in (".stdout", ".stderr", ".strict.json")), "command_output_exists")
        wrapper = [sys.executable, "-B", str(sdk / "tests/subreaper_runner.py"), "--report", str(strict_path), "--"] + list(map(str, command))
        entry = {"label": label, "passed": False, "forced_fixture_cleanup": False, "cleanup_confirmed": False}
        report["commands"].append(entry)
        owner = None
        try:
            owner = OwnedProcess(wrapper, project, prefix, dict(environment, PYTHONPATH=str(sdk)) if build else environment)
            status = owner.process.wait(timeout=300)
            owner.reap_and_check()
            strict = read_json(strict_path)
            entry["cleanup_confirmed"] = strict["subreaper"] is True and strict["runner_error"] is None
            entry.update(wrapper_returncode=status, command_returncode=strict["command_returncode"],
                         strict_exit_status=strict["exit_status"], strict_runner_error=strict["runner_error"],
                         adopted_status_histogram=dict(collections.Counter(str(row["returncode"]) for row in strict["reaped"])),
                         strict_report_sha256=fingerprint(strict_path)["sha256"], tracked_processes_absent=True)
            require(status == 0 and strict["subreaper"] is True and strict["command_returncode"] == 0
                    and strict["exit_status"] == 0 and strict["runner_error"] is None, "command_or_strict_gate_failed")
            require(owner.stdout_path.stat().st_size <= MAX_LOG and owner.stderr_path.stat().st_size <= MAX_LOG, "command_log_limit")
            output, errors = owner.stdout_path.read_bytes(), owner.stderr_path.read_bytes()
            entry.update(passed=True, stdout_sha256=hashlib.sha256(output).hexdigest(),
                         stderr_sha256=hashlib.sha256(errors).hexdigest(), fallback_warning_count=errors.count(MARKER))
            return output, entry
        finally:
            if owner is not None:
                try:
                    if owner.process.poll() is None or not entry["cleanup_confirmed"]:
                        entry["forced_fixture_cleanup"] = True
                        owner.emergency_cleanup()
                finally:
                    owner.finish()

    management = [host, "--codex-home", home]
    model_state = home / "components/state" / MODEL
    custom_state = home / "components/state" / CUSTOM

    def build_fixture(plugin_id, kind, source_name):
        source, artifact = work / (plugin_id + "-source"), work / (plugin_id + "-package")
        source.mkdir()
        shutil.copyfile(HERE / source_name, source / "plugin.py")
        for notice in ("LICENSE", "NOTICE"):
            shutil.copyfile(repo / notice, source / notice)
        write_json(source / "codex-component.json", {"api_version": 1, "id": plugin_id, "version": "0.1.0",
                   "entrypoint": "plugin.pyz", "components": [{"kind": kind, "name": "default", "contract_version": 1}]})
        run("build-" + plugin_id, [sys.executable, "-B", "-m", "codex_component_sdk", "build", source, "--output", artifact], build=True)
        source.rename(source.with_name(source.name + ".parked"))
        run("install-" + plugin_id, management + ["install", artifact])

    def image_turn(label, resume=None):
        prior = len(list(model_state.glob("request-*.json")))
        command = [codex, "exec"] + (["resume"] if resume else []) + ["--skip-git-repo-check", "--json"]
        command += [resume, "Continue the image acceptance conversation."] if resume else ["--image", str(image), "--", "Inspect this acceptance image."]
        if label == "native-inline":
            require(args.strace.is_file() and fingerprint(args.strace)["sha256"] == pins["strace_sha256"], "trace_unavailable_or_changed")
            require(not list(work.glob("native-trace.*")), "trace_prefix_not_fresh")
            command = [args.strace, "-ff", "-qq", "-s", "4096", "-e",
                       "trace=execve,execveat,clone,clone3,fork,vfork,openat,exit_group", "-o", work / "native-trace", "--"] + command
        output, entry = run(label, command)
        events = [json.loads(line) for line in output.splitlines() if line.strip()]
        require(any(event.get("type") == "turn.completed" for event in events), "turn_not_completed")
        require(b"attachment-proof complete" in output, "model_completion_missing")
        require(len(list(model_state.glob("request-*.json"))) == prior + 1, "unexpected_model_request_count")
        request = read_json(model_state / f"request-{prior}.json")
        require(request["schema"] == "attachment-model-probe-v1" and len(request["images"]) == 1, "model_image_input_missing")
        thread_id = next((event.get("thread_id") for event in events if event.get("type") == "thread.started"), None)
        require(type(thread_id) is str and bool(thread_id), "thread_identity_missing")
        require(not list((home / "components/state").glob("*/.attachment-transfer-*")), "attachment_staging_survived")
        return request["images"][0], thread_id, entry

    try:
        build_fixture(MODEL, "model_transport", "model_probe.py")
        build_fixture(CUSTOM, "attachment_store", "counted_store.py")
        run("select-model", management + ["select", "model_transport", "default", MODEL])
        (home / "config.toml").write_text('model = "gpt-5.1-codex"\nmodel_provider = "attachment_fixture"\n'
            '[model_providers.attachment_fixture]\nname = "Deterministic attachment fixture"\n'
            'base_url = "http://127.0.0.1:9/v1"\nwire_api = "responses"\nrequires_openai_auth = false\n')
        image = project / "fixture.png"
        image.write_bytes(image_bytes())
        installed = home / "components/objects"
        initial_objects = inventory(installed)
        baseline, _, base_cmd = image_turn("builtin-baseline")
        require(baseline["kind"] == "inline" and base_cmd["fallback_warning_count"] == 0, "builtin_baseline")
        report["cases"]["builtin_baseline"] = {"passed": True, "model_image": baseline}
        require(inventory(installed) == initial_objects, "initial_objects_changed_during_baseline")
        run("select-counted-before-native-install", management + ["select", "attachment_store", "default", CUSTOM])
        run("install-native-inline", management + ["install", package])
        settings = read_json(home / "components/config.json")
        require(settings["selections"].get("attachment_store:default") == CUSTOM, "native_install_changed_selection")
        packages_before = inventory(installed)
        require(all(packages_before.get(name) == value for name, value in initial_objects.items()), "initial_objects_changed_during_install")
        report["initial_objects_preserved_through_native_install"] = True
        run("select-native-inline", management + ["select", "attachment_store", "default", NATIVE])
        settings = read_json(home / "components/config.json")
        native_path = installed / settings["installed"][NATIVE] / native_manifest["entrypoint"]
        require(fingerprint(native_path) == expected[native_manifest["entrypoint"]], "installed_native_binary_changed")
        try:
            native_image, _, native_cmd = image_turn("native-inline")
            native_trace = inspect_trace(work / "native-trace", native_path, home / "components/state" / NATIVE)
            require(native_image == baseline and native_cmd["fallback_warning_count"] == 0, "native_inline_payload_or_fallback")
            report["cases"]["native_inline"] = {"passed": True, "proof": "exact_native_upload_path_and_no_observed_error_fallback",
                "model_image": native_image, "trace": native_trace, "fallback_warning_count": 0,
                "native_result_envelope_inspected": False}
        except Exception as error:
            report["cases"]["native_inline"] = {"passed": False, "status": "trace_or_native_gate_unavailable_or_failed",
                "failure_code": str(error) if isinstance(error, (GateError, TraceError)) else type(error).__name__}
            require(all(entry["cleanup_confirmed"] and not entry["forced_fixture_cleanup"]
                        for entry in report["commands"]), "native_cleanup_unconfirmed")
        run("select-counted", management + ["select", "attachment_store", "default", CUSTOM])
        first, thread, first_cmd = image_turn("counted-upload")
        require(first == {"kind": "file", "fixture_file_id": True} and first_cmd["fallback_warning_count"] == 0, "counted_file_reference")
        require(len(list(custom_state.glob("upload-*.json"))) == 1, "counted_first_upload")
        uploaded = read_json(custom_state / "upload-0.json")
        require(uploaded["typed_backend_failure"] is False, "counted_fixture_failed")
        resumed, resumed_thread, resumed_cmd = image_turn("counted-cold-resume", resume=thread)
        require(resumed == first and resumed_thread == thread and resumed_cmd["fallback_warning_count"] == 0, "cold_resume_reference")
        require(len(list(custom_state.glob("upload-*.json"))) == 1, "cold_resume_reuploaded")
        report["cases"]["counted_upload_and_cold_resume"] = {"passed": True, "upload_count_before_resume": 1,
            "upload_count_after_resume": 1, "same_thread_internal_check": True, "fixture_file_reference_retained": True}
        settings_path = home / "components/config.json"
        settings = read_json(settings_path)
        settings.setdefault("config", {})[CUSTOM] = {"fail": True}
        temporary = home / "components/fixture-config.new"
        write_json(temporary, settings); temporary.replace(settings_path)
        failed, _, failed_cmd = image_turn("counted-error-fallback")
        require(len(list(custom_state.glob("upload-*.json"))) == 2, "failed_upload_not_observed")
        failure = read_json(custom_state / "upload-1.json")
        require(failure["typed_backend_failure"] is True and failed_cmd["fallback_warning_count"] >= 1, "fallback_marker_positive_control")
        require(failed["kind"] == "inline" and failed["sha256"] == failure["prepared_sha256"] and failed["bytes"] == failure["bytes"], "fallback_prepared_bytes_changed")
        report["cases"]["typed_error_inline_fallback"] = {"passed": True, "upload_count": 2,
            "prepared_bytes_preserved": True, "fallback_warning_positive_control": True}
        native_state = home / "components/state" / NATIVE
        require(native_state.is_dir(), "native_state_directory_missing")
        native_state_before = inventory(native_state)
        before_removal = read_json(home / "components/config.json")
        require(before_removal["selections"].get("attachment_store:default") == CUSTOM, "remove_target_still_selected")
        expected_settings = json.loads(json.dumps(before_removal))
        expected_settings["enabled"] = [item for item in expected_settings["enabled"] if item != NATIVE]
        expected_settings["installed"].pop(NATIVE)
        expected_settings["selections"] = {key: value for key, value in expected_settings["selections"].items() if value != NATIVE}
        run("remove-unselected-native", management + ["remove", NATIVE])
        require(read_json(home / "components/config.json") == expected_settings, "native_removal_registry_mismatch")
        require(inventory(installed) == packages_before and native_path.is_file(), "native_removal_changed_immutable_objects")
        require(native_state.is_dir() and inventory(native_state) == native_state_before, "native_removal_changed_state")
        report["native_removal"] = {"passed": True, "activation_and_installed_mapping_removed": True,
            "counted_selection_preserved": True, "immutable_objects_retained": True,
            "native_state_directory_retained": True, "native_state_files_unchanged": True,
            "native_state_file_count": len(native_state_before)}
        run("reset-attachment", management + ["reset", "attachment_store", "default"])
        restored, _, reset_cmd = image_turn("builtin-restored")
        require(restored == baseline and reset_cmd["fallback_warning_count"] == 0, "builtin_restore")
        require(len(list(custom_state.glob("upload-*.json"))) == 2, "reset_still_called_custom")
        report["cases"]["builtin_restored"] = {"passed": True, "custom_upload_count_unchanged": True}
        require(inventory(installed) == packages_before, "installed_packages_changed")
        report["installed_packages_unchanged"] = True
        report["passed"] = all(case["passed"] for case in report["cases"].values())
    except Exception as error:
        report["failure_code"] = str(error) if isinstance(error, GateError) else type(error).__name__
    finally:
        report["host_after"] = {"CLI": fingerprint(codex), "manager": fingerprint(host)}
        report["host_unchanged"] = report["host_after"] == before
        report["native_package_unchanged"] = inventory(package) == expected
        report["external_report_and_proposal_inputs_unchanged"] = all(still_matches(Path(path), value) for path, value in external_before.items())
        report["helper_sources_unchanged"] = all(same_sha(repo / name, digest) for name, digest in pins["repo_source_sha256"].items())
        report["passed"] = (report["passed"] and report["host_unchanged"] and report["native_package_unchanged"]
                            and report["external_report_and_proposal_inputs_unchanged"] and report["helper_sources_unchanged"])
        write_json(work / "acceptance.json", report)
    print(work / "acceptance.json")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
