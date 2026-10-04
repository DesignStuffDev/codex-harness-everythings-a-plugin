"""Real installed-tool/manager SIGINT gate; all evidence stays in a new private home."""

import argparse
import ctypes
import errno
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import time


def fingerprint(path):
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"sha256": digest, "bytes": path.stat().st_size}


def package_files(package):
    return {str(p.relative_to(package)): fingerprint(p)
            for p in sorted(package.rglob("*")) if p.is_file()}


def save(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--host", required=True, type=Path)
    parser.add_argument("--package", required=True, type=Path)
    parser.add_argument("--request", required=True, type=Path)
    parser.add_argument("--real-git", required=True, type=Path)
    parser.add_argument("--work-dir", required=True, type=Path)
    args = parser.parse_args()
    repo, host, package = (p.resolve(strict=True) for p in
                           (args.repo, args.host, args.package))
    work = args.work_dir.absolute()
    assert not work.exists() and repo != work and repo not in work.parents
    os.umask(0o077)
    work.mkdir(parents=True, mode=0o700)
    sys.path.insert(0, str(repo / "component-sdk/tests"))
    from p01_gui_acceptance import OwnedProcess, identity, send_signal, drain_subreaper

    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
    libc.prctl.restype = ctypes.c_int
    assert libc.prctl(36, 1, 0, 0, 0) == 0, "child subreaper unavailable"
    request_raw = args.request.read_bytes()
    assert len(request_raw) <= 32768
    request = json.loads(request_raw)
    real_git = args.real_git.resolve(strict=True)
    with real_git.open("rb") as stream:
        assert stream.read(4) == b"\x7fELF", "real Git must be a native ELF"
    before = {"host": fingerprint(host), "package": package_files(package)}
    manifest = json.loads((package / "codex-component.json").read_text())
    name = "upstream_impact_review"
    assert manifest["id"] == "codex.maintenance.upstream-review"
    environment = dict(os.environ, PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1")
    base = [str(host), "--codex-home", str(work / "component-home")]
    report = {"passed": False, "package_id": manifest["id"],
              "package_version": manifest["version"], "before": before,
              "request": fingerprint(args.request), "real_git": str(real_git),
              "rescue_used": False, "graceful_protocol_proven": False,
              "tracking_limit": "sampled identities plus unchanged owner/subreaper checks",
              "commands": []}
    owners, held, writer, fifo = [], None, None, None

    def start(argv, label):
        owner = OwnedProcess(argv, work, work / label, environment)
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

    def call(value):
        params = {"call_id": "active-git-cancellation", "name": name,
                  "arguments": value}
        return base + ["call", "tool", name, "invoke", json.dumps(params)]

    def fifo_reader(pid, expected):
        try:
            for fd in Path(f"/proc/{pid}/fd").iterdir():
                try:
                    item = fd.stat()
                    if (item.st_dev, item.st_ino) != expected:
                        continue
                    info = Path(f"/proc/{pid}/fdinfo/{fd.name}").read_text()
                    flags = next(line.split()[1] for line in info.splitlines()
                                 if line.startswith("flags:"))
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
        if fifo.exists():
            current = fifo.stat()
            assert stat.S_ISFIFO(current.st_mode)
            assert (current.st_dev, current.st_ino) == fifo_identity
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
        installed_before = package_files(work / "component-home/components/objects")
        fixture = work / "object-fixture.git"
        completed(["git", "init", "--bare", str(fixture)], "git-init", 15)
        common = completed(["git", "-C", request["repository"],
                            "rev-parse", "--git-common-dir"], "git-common", 15)
        common_dir = Path(common.read_text().strip())
        if not common_dir.is_absolute():
            common_dir = Path(request["repository"]) / common_dir
        object_dir = (common_dir / "objects").resolve(strict=True)
        alternates = (str(object_dir) + "\n").encode()
        assert len(alternates) <= 4096 and b"\x00" not in alternates
        fifo = fixture / "objects/info/alternates"
        os.mkfifo(fifo, 0o600)
        info = fifo.stat()
        fifo_identity = (info.st_dev, info.st_ino)
        blocked_request = dict(request, repository=str(fixture))
        held = start(call(blocked_request), "held-call")
        deadline = time.monotonic() + 10
        ready = None
        while time.monotonic() < deadline:
            assert held.process.poll() is None, "manager exited before real Git readiness"
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
                fd = fifo_reader(saved["pid"], fifo_identity)
                if fd is not None:
                    ready = {"git": saved, "fifo_dev_inode": fifo_identity,
                             "git_fifo_fd": fd, "ack_monotonic_ns": time.monotonic_ns(),
                             "owned_snapshot": held.snapshot()}
                    break
            if ready:
                break
            time.sleep(0.01)
        assert ready is not None, "actual Git FIFO barrier not established"
        assert ready["git"]["group"] != held.root["group"], "Git is in manager group"
        report["readiness"] = ready
        primary = {"passed": False, "writer_held": True, "manager_status": None,
                   "signal": "SIGINT to manager PID only", "rescue_used": False}
        started = time.monotonic()
        primary["signal_monotonic_ns"] = time.monotonic_ns()
        send_signal(held.root, signal.SIGINT)
        try:
            primary["manager_status"] = held.process.wait(timeout=5)
            held.reap_and_check(seconds=max(0, 5 - (time.monotonic() - started)))
            primary["passed"] = True
        except (AssertionError, subprocess.TimeoutExpired) as error:
            primary["failure"] = type(error).__name__
        primary["elapsed_seconds"] = time.monotonic() - started
        primary["remaining"] = [saved for saved in held.snapshot()
                                if Path(f"/proc/{saved['pid']}").exists()]
        primary["passed"] = (primary["passed"] and not primary["remaining"]
                             and primary["elapsed_seconds"] <= 5)
        report["primary"] = primary
        save(work / "primary-before-rescue.json", primary)
        if not primary["passed"]:
            report["rescue_used"] = True
        release_barrier()
        try:
            held.process.wait(timeout=10)
            held.reap_and_check()
        except (AssertionError, subprocess.TimeoutExpired):
            report["rescue_used"] = True
            report["forced_owned_cleanup"] = True
            held.emergency_cleanup()
        report["after_held_drain"] = drain_subreaper()
        assert report["after_held_drain"]["passed"], "owned children remain after rescue"
        normal = completed(call(request), "recovery-call")
        tool = json.loads(normal.read_text())
        value = json.loads(tool["text"])
        assert len(tool["text"].encode()) <= 8192 and value["update_allowed"] is False
        assert tool["success"] == (value["status"] == "review_required")
        assert value["status"] == "review_required", "known-good invocation did not recover"
        report["recovery"] = {"passed": True, "status": value["status"],
                              "update_allowed": False}
        report["installed_packages_unchanged"] = (
            installed_before == package_files(work / "component-home/components/objects"))
        assert report["installed_packages_unchanged"]
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
                except Exception as cleanup_error:
                    report.setdefault("cleanup_errors", []).append(
                        type(cleanup_error).__name__)
            finally:
                owner.finish()
        report["final_drain"] = drain_subreaper()
        report["after"] = {"host": fingerprint(host), "package": package_files(package)}
        report["inputs_unchanged"] = report["after"] == before
        report["passed"] = bool(report.get("primary", {}).get("passed")
                                and report.get("recovery", {}).get("passed")
                                and report["final_drain"]["passed"]
                                and report["inputs_unchanged"]
                                and report.get("installed_packages_unchanged")
                                and not report["rescue_used"]
                                and not report.get("error") and not report.get("cleanup_errors"))
        save(work / "acceptance-private.json", report)
    print(work / "acceptance-private.json")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
