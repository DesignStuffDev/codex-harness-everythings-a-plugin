"""Installed capsule shutdown/abrupt interruption after real admission; no test hooks."""

import argparse
import ctypes
import errno
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import queue
import signal
import stat
import sys
import threading
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("repo", "host", "package", "request", "real-git", "work-dir", "prior-helper"):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    assert hashlib.sha256(args.prior_helper.read_bytes()).hexdigest() == (
        "8257c0f1d7d1bfb22ccaa03a64878598c07f891e28f712ef27c543da1f52181e")
    spec = importlib.util.spec_from_file_location("prior_acceptance", args.prior_helper)
    prior = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(prior)
    fingerprint, package_files, save = prior.fingerprint, prior.package_files, prior.save
    repo, host, package = (p.resolve(strict=True) for p in (args.repo, args.host, args.package))
    work = args.work_dir.resolve(strict=False)
    assert not work.exists() and repo != work and repo not in work.parents
    os.umask(0o077)
    work.mkdir(mode=0o700, parents=True)
    sys.path.insert(0, str(repo / "component-sdk/tests"))
    from p01_gui_acceptance import OwnedProcess, identity, send_signal, drain_subreaper
    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
    libc.prctl.restype = ctypes.c_int
    assert libc.prctl(36, 1, 0, 0, 0) == 0, "child subreaper unavailable"
    manifest = json.loads((package / "codex-component.json").read_bytes())
    plugin_id, name = manifest["id"], "upstream_source_capsule"
    assert plugin_id == "codex.maintenance.upstream-review" and manifest["version"] == "0.2.0"
    raw = args.request.read_bytes()
    assert len(raw) <= 32768
    request = json.loads(raw)
    assert request["candidate_revision"] == "2e5fea64eefcaa19f48458b2386011b619f69c70"
    assert request["composition_revision"] == "914cc59374c1149463e78bc33851d83e3f14d0a4"
    real_git = args.real_git.resolve(strict=True)
    with real_git.open("rb") as stream:
        assert stream.read(4) == b"\x7fELF", "real Git must be native"
    inputs = [args.request, Path(request["lineage_index"])]
    if "publication_receipt" in request:
        inputs.append(Path(request["publication_receipt"]))
    before = {"host": fingerprint(host), "package": package_files(package),
              "inputs": {str(p): fingerprint(p) for p in inputs}}
    environment = dict(os.environ, PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1",
                       GIT_OPTIONAL_LOCKS="0", GIT_NO_LAZY_FETCH="1", GIT_NO_REPLACE_OBJECTS="1")
    home = work / "component-home"
    manager = [str(host), "--codex-home", str(home)]
    report = dict(passed=False, package_version="0.2.0", before=before, cases=[], commands=[],
                  scope="direct installed SDK protocol; recovery through real component manager",
                  manager_graceful_forwarding_tested=False, update_or_rollback_tested=False,
                  barrier_race="Admission-to-FIFO replacement may lose scheduling race; any blob or missing native FD fails establishment.",
                  rescue_used=False)
    owners, readers, writer, fifo, fifo_identity, alternates = [], [], None, None, None, None

    def start(argv, label, directory=work, interactive=False, env=environment):
        owner = OwnedProcess(argv, directory, work / label, env, interactive)
        owners.append(owner)
        report["commands"].append(dict(label=label, pid=owner.root["pid"]))
        return owner

    def completed(argv, label, expected=0, env=environment):
        owner = start(argv, label, env=env)
        status = owner.process.wait(timeout=130)
        owner.reap_and_check()
        report["commands"][-1]["returncode"] = status
        assert status == expected, label + " failed"
        return owner.stdout_path

    def manager_call(tool_name, value, label):
        envelope = dict(call_id=label, name=tool_name, arguments=value)
        tool = json.loads(completed(manager + ["call", "tool", tool_name, "invoke", json.dumps(envelope)], label).read_bytes())
        assert tool["success"] is True and len(tool["text"].encode()) <= 8192
        value = json.loads(tool["text"])
        assert value["update_allowed"] is False
        return value

    def send(owner, frame):
        owner.process.stdin.write(json.dumps(frame, separators=(",", ":")) + "\n")
        owner.process.stdin.flush()

    def reader_for(owner):
        messages = queue.Queue()
        def collect():
            try:
                for line in owner.process.stdout:
                    owner.stdout.write(line)
                    owner.stdout.flush()
                    messages.put(line)
            finally:
                messages.put(None)
        reader = threading.Thread(target=collect, daemon=True)
        readers.append((owner, reader))
        reader.start()
        return messages

    def receive(messages, seconds):
        line = messages.get(timeout=max(0, seconds))
        assert line is not None and len(line.encode()) <= 65536, "missing or oversized frame"
        return json.loads(line)

    def fifo_reader(pid):
        try:
            for fd in Path(f"/proc/{pid}/fd").iterdir():
                try:
                    item = fd.stat()
                    if (item.st_dev, item.st_ino) != fifo_identity:
                        continue
                    lines = Path(f"/proc/{pid}/fdinfo/{fd.name}").read_text().splitlines()
                    flags = next(line.split()[1] for line in lines if line.startswith("flags:"))
                    if int(flags, 8) & os.O_ACCMODE == os.O_RDONLY:
                        return int(fd.name)
                except (FileNotFoundError, PermissionError):
                    continue
        except (FileNotFoundError, PermissionError):
            pass
        return None

    def release_barrier():
        nonlocal writer, fifo
        if fifo is None:
            return
        current = fifo.stat()
        assert stat.S_ISFIFO(current.st_mode) and (current.st_dev, current.st_ino) == fifo_identity
        replacement = fifo.with_name("alternates.release")
        replacement.write_bytes(alternates)
        os.replace(replacement, fifo)
        fifo = None
        if writer is not None:
            try:
                os.write(writer, alternates)
            except BrokenPipeError:
                pass
            finally:
                os.close(writer)
                writer = None

    def inspect(job, label, expected):
        value = json.loads(completed([sys.executable, "-I", "-B", installed / "capsule_bootstrap.py", job],
                                    label, expected=expected, env=dict(environment, PATH=str(empty))).read_bytes())
        assert value["candidate_assembled"] is False and value["update_allowed"] is False
        assert value["activation_allowed"] is False
        return value

    try:
        completed(manager + ["install", str(package)], "install")
        catalog = json.loads(completed(manager + ["list"], "list").read_bytes())
        installed = home / "components/objects" / catalog["installed"][plugin_id]
        installed_before = package_files(installed)
        assert installed_before == before["package"]
        entrypoint = (installed / manifest["entrypoint"]).resolve(strict=True)
        assert installed.resolve() in entrypoint.parents
        empty = work / "empty-path"
        empty.mkdir()
        fixture = work / "object-fixture.git"
        completed([str(real_git), "init", "--bare", str(fixture)], "git-init")
        common = completed([str(real_git), "-C", request["repository"], "rev-parse", "--git-common-dir"], "git-common")
        common_dir = Path(common.read_text().strip())
        if not common_dir.is_absolute():
            common_dir = Path(request["repository"]) / common_dir
        shallow = common_dir / "shallow"
        report["fixture_shallow"] = dict(source=str(shallow), present=shallow.exists(), copied=False)
        if shallow.exists():
            assert shallow.is_file() and not shallow.is_symlink(), "regular shallow input required"
            with shallow.open("rb") as stream:
                shallow_bytes = stream.read(16385)
            rows = shallow_bytes.splitlines()
            assert 0 < len(shallow_bytes) <= 16384 and 0 < len(rows) <= 256
            assert len(set(rows)) == len(rows) and b"\n".join(rows) + b"\n" == shallow_bytes
            assert all(len(row) == 40 and all(c in b"0123456789abcdef" for c in row) for row in rows)
            digest = hashlib.sha256(shallow_bytes).hexdigest()
            inputs.append(shallow)
            before["inputs"][str(shallow)] = dict(sha256=digest, bytes=len(shallow_bytes))
            with (fixture / "shallow").open("xb") as stream:
                stream.write(shallow_bytes)
                stream.flush()
                os.fsync(stream.fileno())
            assert fingerprint(fixture / "shallow") == before["inputs"][str(shallow)]
            report["fixture_shallow"].update(copied=True, sha256=digest, bytes=len(shallow_bytes), boundaries=len(rows))
        alternates = (str((common_dir / "objects").resolve(strict=True)) + "\n").encode()
        assert len(alternates) <= 4096 and b"\x00" not in alternates
        alternate_path = fixture / "objects/info/alternates"
        alternate_path.write_bytes(alternates)
        bound = dict(request, repository=str(fixture))
        review = manager_call("upstream_impact_review", bound, "review")
        assert review["status"] == "review_required"
        value = dict(contract_version=1, purpose="prepare_source_inputs", review_request=bound,
                     expected_plan_id=review["plan_id"], limits=dict(max_changed_paths=13,
                     max_unique_blob_bytes=64 * 1024 * 1024, max_manifest_bytes=8 * 1024 * 1024))
        for mode in ("shutdown", "abrupt"):
            primary = dict(mode=mode, passed=False, admission_verified=False, barrier_verified=False,
                           rescue_used=False, abrupt_durability="not attested" if mode == "abrupt" else None)
            primary["control"] = {"type": "shutdown"} if mode == "shutdown" else {"signal": "SIGKILL", "target": "installed plugin only"}
            report["cases"].append(primary)
            state = work / (mode + "-state")
            state.mkdir()
            pending_fifo = work / (mode + "-barrier")
            os.mkfifo(pending_fifo, 0o600)
            info = pending_fifo.stat()
            fifo_identity = (info.st_dev, info.st_ino)
            held = start([str(entrypoint), *manifest["args"]], mode, installed, True)
            messages = reader_for(held)
            send(held, dict(type="initialize", api_version=1, plugin_id=plugin_id, config={}, state_dir=str(state)))
            assert receive(messages, 10) == dict(type="ready", api_version=1)
            send(held, dict(type="request", id=1, component=dict(kind="tool", name=name), method="invoke",
                            params=dict(call_id=mode, name=name, arguments=value)))
            try:
                event = receive(messages, 95)
                assert event["type"] == "event" and event["id"] == 1
                assert event["event"]["stage"] == "source_inputs_admitted"
                job_id = event["event"]["job_id"]
                assert len(job_id) == 32 and all(c in "0123456789abcdef" for c in job_id)
                os.replace(pending_fifo, alternate_path)
                fifo = alternate_path
                job = state / "upstream-capsules" / job_id
                assert (job / "INTENT.json").is_file() and (job / "blobs").is_dir()
                primary.update(admission_verified=True, admission_event=event, job_id=job_id,
                               admission_observed_monotonic_ns=time.monotonic_ns())
                ready, deadline = None, time.monotonic() + 10
                while time.monotonic() < deadline:
                    assert held.process.poll() is None, "component exited before barrier"
                    assert not list((job / "blobs").iterdir()) and not (job / "SEALED.json").exists(), "first blob already materialized"
                    for saved in held.snapshot():
                        current = identity(saved["pid"])
                        if (saved.get("executable") != str(real_git) or not current
                                or current["start_ticks"] != saved["start_ticks"] or current["state"] in ("Z", "X")):
                            continue
                        if writer is None:
                            try:
                                writer = os.open(fifo, os.O_WRONLY | os.O_NONBLOCK)
                            except OSError as error:
                                if error.errno != errno.ENXIO:
                                    raise
                                continue
                        fd = fifo_reader(saved["pid"])
                        if fd is not None:
                            ready = dict(git=saved, fifo_dev_inode=fifo_identity, git_fifo_fd=fd,
                                         ack_monotonic_ns=time.monotonic_ns())
                            break
                    if ready:
                        break
                    time.sleep(0.01)
                assert ready is not None, "actual Git FIFO barrier not established"
                primary.update(barrier_verified=True, readiness=ready, writer_held=True)
                started = time.monotonic()
                if mode == "shutdown":
                    send(held, dict(type="shutdown"))
                    frame = receive(messages, 5)
                    assert frame["type"] == "result" and frame["id"] == 1
                    assert frame["result"]["success"] is False
                    result = json.loads(frame["result"]["text"])
                    assert result["status"] == "incomplete_or_unavailable" and not result["source_materials_ready"]
                    assert result["diagnostic_code"] == "cancelled_or_deadline_reached" and result["job_id"] == job_id
                    primary["typed_result"] = result
                else:
                    send_signal(held.root, signal.SIGKILL)
                primary["plugin_status"] = held.process.wait(timeout=max(0, 5 - (time.monotonic() - started)))
                assert primary["plugin_status"] == (0 if mode == "shutdown" else -9)
                held.reap_and_check(seconds=max(0, 5 - (time.monotonic() - started)))
                primary["cleanup_elapsed_seconds"] = time.monotonic() - started
                assert primary["cleanup_elapsed_seconds"] <= 5
                assert (job / "INTENT.json").is_file() and not (job / "SEALED.json").exists()
                assert not list((job / "blobs").iterdir()), "held first blob must remain unmaterialized"
                assert (job / "TERMINAL.json").exists() is (mode == "shutdown")
                primary["offline_inspection"] = inspect(job, mode + "-inspect", 2)
                assert primary["offline_inspection"]["status"] == "incomplete_or_uncertain"
                primary["passed"] = True
            except BaseException as error:
                primary["failure"] = dict(type=type(error).__name__, message=str(error))
            primary["tracked"] = held.snapshot()
            primary["reaped_orphans"] = list(held.reaped_orphans)
            primary["remaining"] = [p for p in held.snapshot() if Path(f"/proc/{p['pid']}").exists()]
            primary["passed"] = primary["passed"] and not primary["remaining"]
            save(work / (mode + "-primary-before-rescue.json"), primary)
            primary["rescue_used"] = not primary["passed"]
            report["rescue_used"] |= primary["rescue_used"]
            release_barrier()
            held.process.wait(timeout=10)
            held.reap_and_check()
            assert primary["passed"], "primary gate failed; preserved before rescue"
            retained = package_files(job)
            recovery = manager_call(name, value, mode + "-recovery")
            assert recovery["status"] == "sealed" and recovery["durability"] == "directory_fsync_completed"
            recovered_job = home / "components/state" / plugin_id / "upstream-capsules" / recovery["job_id"]
            assert inspect(recovered_job, mode + "-recovery-inspect", 0)["status"] == "sealed"
            assert package_files(job) == retained, "recovery altered interrupted job"
            primary["recovery"] = recovery
            primary["interrupted_job_files"] = retained
            primary["drain"] = drain_subreaper()
            assert primary["drain"]["passed"]
        report["installed_package_unchanged"] = package_files(installed) == installed_before
    except BaseException as error:
        report["error"] = dict(type=type(error).__name__, message=str(error))
    finally:
        try:
            release_barrier()
        except Exception as error:
            report["barrier_cleanup_error"] = type(error).__name__
        for owner in reversed(owners):
            try:
                if owner.process.poll() is None:
                    report["rescue_used"] = True
                    owner.emergency_cleanup()
                owner.reap_and_check()
            except Exception as error:
                report.setdefault("cleanup_errors", []).append(type(error).__name__)
                report["rescue_used"] = True
                try:
                    owner.emergency_cleanup()
                except Exception as cleanup_error:
                    report["cleanup_errors"].append(type(cleanup_error).__name__)
            finally:
                try:
                    for reader_owner, reader in readers:
                        if reader_owner is owner:
                            reader.join(timeout=5)
                            if reader.is_alive():
                                report.setdefault("cleanup_errors", []).append("reader_still_alive")
                    owner.finish()
                except Exception as finish_error:
                    report.setdefault("cleanup_errors", []).append(type(finish_error).__name__)
        try:
            report["final_drain"] = drain_subreaper()
        except Exception as drain_error:
            report["final_drain"] = dict(passed=False, error=type(drain_error).__name__)
        report["inputs_unchanged"] = False
        try:
            report["after"] = dict(host=fingerprint(host), package=package_files(package),
                                   inputs={str(p): fingerprint(p) for p in inputs})
            report["inputs_unchanged"] = before == report["after"]
        except Exception as input_error:
            report["input_check_error"] = type(input_error).__name__
        report["passed"] = bool(len(report["cases"]) == 2 and all(c["passed"] and c.get("recovery") for c in report["cases"])
                                and report.get("installed_package_unchanged") and report["inputs_unchanged"]
                                and report["final_drain"]["passed"] and not report["rescue_used"]
                                and not report.get("error") and not report.get("cleanup_errors")
                                and not report.get("barrier_cleanup_error"))
        save(work / "acceptance-private.json", report)
    print(work / "acceptance-private.json")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
