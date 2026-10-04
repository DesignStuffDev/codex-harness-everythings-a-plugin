"""Installed maintenance SDK shutdown with active real Git; not manager SIGINT."""

import argparse
import ctypes
import errno
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import queue
import stat
import sys
import threading
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("repo", "host", "package", "request", "real-git", "work-dir", "prior-helper"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--expected-version", default="0.1.1")
    args = parser.parse_args()
    assert hashlib.sha256(args.prior_helper.read_bytes()).hexdigest() == (
        "8257c0f1d7d1bfb22ccaa03a64878598c07f891e28f712ef27c543da1f52181e")
    spec = importlib.util.spec_from_file_location("prior_acceptance", args.prior_helper)
    prior = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(prior)
    fingerprint, package_files, save = prior.fingerprint, prior.package_files, prior.save
    repo, host, package = (p.resolve(strict=True) for p in
                           (args.repo, args.host, args.package))
    work = args.work_dir.absolute()
    assert not work.exists() and repo != work and repo not in work.parents
    os.umask(0o077)
    work.mkdir(mode=0o700, parents=True)
    sys.path.insert(0, str(repo / "component-sdk/tests"))
    from p01_gui_acceptance import OwnedProcess, identity, drain_subreaper

    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
    libc.prctl.restype = ctypes.c_int
    assert libc.prctl(36, 1, 0, 0, 0) == 0, "child subreaper unavailable"
    manifest = json.loads((package / "codex-component.json").read_text())
    plugin_id, name = manifest["id"], "upstream_impact_review"
    assert plugin_id == "codex.maintenance.upstream-review"
    assert manifest["version"] == args.expected_version
    raw = args.request.read_bytes()
    assert len(raw) <= 32768
    request = json.loads(raw)
    real_git = args.real_git.resolve(strict=True)
    with real_git.open("rb") as stream:
        assert stream.read(4) == b"\x7fELF", "real Git must be native"
    environment = dict(os.environ, PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1")
    before = {"host": fingerprint(host), "package": package_files(package)}
    home = work / "component-home"
    base = [str(host), "--codex-home", str(home)]
    report = {"passed": False, "scope": "direct installed-component SDK protocol",
              "manager_graceful_forwarding_tested": False,
              "package_version": manifest["version"], "before": before,
              "request": fingerprint(args.request), "real_git": str(real_git),
              "tracking_limit": "sampled identities plus unchanged owner/subreaper checks",
              "rescue_used": False, "commands": []}
    owners, readers, held, writer, fifo = [], [], None, None, None

    def start(argv, label, directory=work, interactive=False):
        owner = OwnedProcess(argv, directory, work / label, environment, interactive)
        owners.append(owner)
        report["commands"].append({"label": label, "pid": owner.root["pid"]})
        return owner

    def completed(argv, label, seconds=130):
        owner = start(argv, label)
        status = owner.process.wait(timeout=seconds)
        owner.reap_and_check()
        report["commands"][-1]["returncode"] = status
        assert status == 0, label + " failed"
        return owner.stdout_path

    def send(owner, frame):
        owner.process.stdin.write(json.dumps(frame, separators=(",", ":")) + "\n")
        owner.process.stdin.flush()

    def receive(messages, seconds):
        line = messages.get(timeout=max(0, seconds))
        assert line is not None, "component EOF before expected frame"
        assert len(line.encode()) <= 65536, "unexpected oversized fixture response"
        return json.loads(line)

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

    def fifo_reader(pid):
        try:
            for fd in Path(f"/proc/{pid}/fd").iterdir():
                try:
                    info = fd.stat()
                    if (info.st_dev, info.st_ino) != fifo_identity:
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
        nonlocal writer
        if writer is None:
            return
        info = fifo.stat()
        assert stat.S_ISFIFO(info.st_mode)
        assert (info.st_dev, info.st_ino) == fifo_identity
        replacement = fifo.with_name("alternates.release")
        replacement.write_bytes(alternates)
        os.replace(replacement, fifo)
        try:
            os.write(writer, alternates)
        except BrokenPipeError:
            pass
        finally:
            os.close(writer)
            writer = None

    try:
        completed(base + ["install", str(package)], "install")
        catalog = json.loads(completed(base + ["list"], "list").read_text())
        installed = home / "components/objects" / catalog["installed"][plugin_id]
        installed_before = package_files(installed)
        assert installed_before == before["package"], "installed package differs"
        installed_manifest = json.loads((installed / "codex-component.json").read_text())
        entrypoint = (installed / installed_manifest["entrypoint"]).resolve(strict=True)
        assert installed.resolve() in entrypoint.parents
        report["installed_entrypoint"] = str(entrypoint)
        fixture = work / "object-fixture.git"
        completed(["git", "init", "--bare", str(fixture)], "git-init", 15)
        common = completed(["git", "-C", request["repository"],
                            "rev-parse", "--git-common-dir"], "git-common", 15)
        common_dir = Path(common.read_text().strip())
        if not common_dir.is_absolute():
            common_dir = Path(request["repository"]) / common_dir
        alternates = (str((common_dir / "objects").resolve(strict=True)) + "\n").encode()
        assert len(alternates) <= 4096
        fifo = fixture / "objects/info/alternates"
        os.mkfifo(fifo, 0o600)
        info = fifo.stat()
        fifo_identity = (info.st_dev, info.st_ino)
        held = start([str(entrypoint), *installed_manifest["args"]],
                     "protocol", installed, True)
        messages = reader_for(held)
        state = work / "protocol-state"
        state.mkdir()
        send(held, {"type": "initialize", "api_version": 1, "plugin_id": plugin_id,
                    "config": {}, "state_dir": str(state)})
        assert receive(messages, 10) == {"type": "ready", "api_version": 1}
        send(held, {"type": "request", "id": 1,
                    "component": {"kind": "tool", "name": name}, "method": "invoke",
                    "params": {"call_id": "protocol-cancellation", "name": name,
                               "arguments": dict(request, repository=str(fixture))}})
        ready, deadline = None, time.monotonic() + 10
        while time.monotonic() < deadline:
            assert held.process.poll() is None, "component exited before real Git readiness"
            for saved in held.snapshot():
                if saved.get("executable") != str(real_git):
                    continue
                current = identity(saved["pid"])
                if not current or current["start_ticks"] != saved["start_ticks"]:
                    continue
                if current["state"] in ("Z", "X"):
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
                    ready = {"git": saved, "fifo_dev_inode": fifo_identity,
                             "git_fifo_fd": fd, "ack_monotonic_ns": time.monotonic_ns()}
                    break
            if ready:
                break
            time.sleep(0.01)
        assert ready is not None, "actual Git FIFO barrier not established"
        report["readiness"] = ready
        primary = {"passed": False, "writer_held": True, "signal_sent": False,
                   "control_frame": {"type": "shutdown"}, "rescue_used": False}
        started = time.monotonic()
        primary["shutdown_sent_monotonic_ns"] = time.monotonic_ns()
        send(held, {"type": "shutdown"})
        try:
            frame = receive(messages, 5 - (time.monotonic() - started))
            assert frame["type"] == "result" and frame["id"] == 1
            tool = frame["result"]
            assert tool["success"] is False and len(tool["text"].encode()) <= 8192
            value = json.loads(tool["text"])
            assert value["schema"] == "codex-installed-upstream-impact-review-v1"
            assert value["status"] == "cancelled" and value["update_allowed"] is False
            assert value["diagnostic_code"] == "cancelled_or_deadline_reached"
            primary["typed_result"] = value
            primary["plugin_status"] = held.process.wait(
                timeout=max(0, 5 - (time.monotonic() - started)))
            assert primary["plugin_status"] == 0
            held.reap_and_check(seconds=max(0, 5 - (time.monotonic() - started)))
            primary["passed"] = True
        except Exception as error:
            primary["failure"] = type(error).__name__
        primary["elapsed_seconds"] = time.monotonic() - started
        primary["tracked"] = held.snapshot()
        primary["reaped_orphans"] = list(held.reaped_orphans)
        primary["remaining"] = [p for p in held.snapshot()
                                if Path(f"/proc/{p['pid']}").exists()]
        primary["passed"] = (primary["passed"] and not primary["remaining"]
                             and primary["elapsed_seconds"] <= 5)
        report["primary"] = primary
        save(work / "primary-before-rescue.json", primary)
        report["rescue_used"] = not primary["passed"]
        release_barrier()
        try:
            held.process.wait(timeout=10)
            held.reap_and_check()
        except Exception:
            report["rescue_used"] = True
            held.emergency_cleanup()
        report["pre_recovery_drain"] = drain_subreaper()
        assert report["pre_recovery_drain"]["passed"], "owned children remain before recovery"
        params = {"call_id": "after-protocol-cancel", "name": name, "arguments": request}
        path = completed(base + ["call", "tool", name, "invoke", json.dumps(params)], "recovery")
        tool = json.loads(path.read_text())
        assert len(tool["text"].encode()) <= 8192
        value = json.loads(tool["text"])
        assert tool["success"] is True and value["status"] == "review_required"
        assert value["update_allowed"] is False
        report["recovery"] = {"passed": True, "status": value["status"]}
        report["installed_package_unchanged"] = installed_before == package_files(installed)
    except BaseException as error:
        report["error"] = {"type": type(error).__name__, "message": str(error)}
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
                except Exception as error:
                    report["cleanup_errors"].append(type(error).__name__)
            finally:
                for reader_owner, reader in readers:
                    if reader_owner is owner:
                        reader.join(timeout=5)
                        if reader.is_alive():
                            report.setdefault("cleanup_errors", []).append("reader_still_alive")
                owner.finish()
        report["final_drain"] = drain_subreaper()
        report["after"] = {"host": fingerprint(host), "package": package_files(package)}
        report["inputs_unchanged"] = before == report["after"]
        report["passed"] = bool(report.get("primary", {}).get("passed")
                                and report.get("recovery", {}).get("passed")
                                and report.get("installed_package_unchanged")
                                and report["inputs_unchanged"] and report["final_drain"]["passed"]
                                and not report["rescue_used"] and not report.get("error")
                                and not report.get("cleanup_errors"))
        save(work / "acceptance-private.json", report)
    print(work / "acceptance-private.json")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
