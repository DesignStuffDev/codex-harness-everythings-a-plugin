#!/usr/bin/env python3
"""Observe a wrapped command without changing its child-reaping or drain policy.

Only /proc stat identity and executable information are read. No cmdline,
environment, cwd, descriptor targets or socket contents are read. Process-name
bytes occur in /proc/stat but are discarded and never stored.
The observer never sends a signal or becomes a subreaper. The wrapped command's
return status is returned independently of diagnostic completeness.
Optional exact-Git readiness acknowledgment hashes only its observed live
descriptor during sampling and publishes a bounded allowlisted sidecar.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def identity(pid):
    try:
        raw = Path(f"/proc/{pid}/stat").read_text()
        fields = raw[raw.rindex(")") + 2:].split()
        return {
            "pid": pid,
            "start_ticks": int(fields[19]),
            "ppid": int(fields[1]),
            "pgid": int(fields[2]),
            "state": fields[0],
        }
    except (OSError, ValueError, IndexError):
        return None


def key(row):
    return (row["pid"], row["start_ticks"])


def executable(row, held, failures):
    """Pin observed executable bytes; hash only after the sampling window."""
    pid = row["pid"]
    try:
        link = os.readlink(f"/proc/{pid}/exe")
        fd = os.open(f"/proc/{pid}/exe", os.O_RDONLY | os.O_CLOEXEC)
    except OSError:
        return {"path": None, "identity": None}
    try:
        before = os.fstat(fd)
        refreshed = identity(pid)
        link_after = os.readlink(f"/proc/{pid}/exe")
        current = os.stat(f"/proc/{pid}/exe")
        if refreshed is None or key(refreshed) != key(row) or link != link_after:
            return {"path": None, "identity": None}
        if (current.st_dev, current.st_ino) != (before.st_dev, before.st_ino):
            return {"path": None, "identity": None}
        token = ":".join(map(str, (
            before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns,
        )))
        if token not in held:
            if len(held) >= 128:
                failures.add("executable_descriptor_cap_reached")
                return {"path": link, "identity": None}
            held[token] = {"fd": fd, "path": link, "stat": before}
            fd = None
        return {"path": link, "identity": token}
    except OSError:
        return {"path": None, "identity": None}
    finally:
        if fd is not None:
            os.close(fd)


class GitAcknowledgment:
    """Optional readiness proof from this observer's own live ancestry sample.

    Only the selected Git descriptor is hashed during sampling. All existing
    deferred executable hashes, descendant observations and exit gates remain.
    """
    def __init__(self, path, executable_path, expected_sha, case_id):
        self.path, self.expected_sha, self.case_id = path, expected_sha, case_id
        self.expected = executable_path.stat()
        self.receipt = None

    def observe(self, row, executable_info, held, root_identity):
        if self.receipt is not None or row['pid'] == root_identity['pid']:
            return
        token = executable_info.get('identity')
        item = held.get(token)
        if item is None:
            return
        before = item['stat']
        expected = self.expected
        if (before.st_dev, before.st_ino) != (expected.st_dev, expected.st_ino):
            return
        # pread leaves the held descriptor offset at zero for the unchanged
        # post-window hash. Never hash the large host during the sample loop.
        hasher, offset = hashlib.sha256(), 0
        while offset < before.st_size:
            data = os.pread(item['fd'], min(1024 * 1024, before.st_size - offset), offset)
            if not data:
                raise ValueError('ack_git_executable_truncated')
            hasher.update(data)
            offset += len(data)
        after = os.fstat(item['fd'])
        refreshed = identity(row['pid'])
        try:
            current = os.stat(f"/proc/{row['pid']}/exe")
        except OSError:
            return  # No live positive proof; proxy remains held and will fail closed.
        signature = lambda st: (st.st_dev, st.st_ino, st.st_size, st.st_mtime_ns)
        if (refreshed is None or key(refreshed) != key(row) or
                refreshed['state'] == 'Z' or signature(before) != signature(after) or
                signature(before) != signature(current) or
                hasher.hexdigest() != self.expected_sha):
            return
        receipt = {
            'schema': 'observed-live-git-readiness-v1', 'case_id': self.case_id,
            'wrapped_root_identity': root_identity, 'identity': row,
            'executable_identity': token, 'sha256': hasher.hexdigest(),
            'metadata_stable_while_hashed': True, 'live_generation_rechecked': True,
            'observed_monotonic_ns': time.monotonic_ns(),
        }
        pending = self.path.with_name(self.path.name + '.pending')
        fd = os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, 'w') as stream:
            json.dump(receipt, stream, sort_keys=True)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        # Publish a complete receipt without overwriting any existing path.
        os.link(pending, self.path)
        pending.unlink()
        self.receipt = receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--interval-seconds", type=float, default=0.05)
    parser.add_argument("--post-exit-seconds", type=float, default=0.25)
    parser.add_argument("--git-ready-ack", type=Path)
    parser.add_argument("--git-ready-executable", type=Path)
    parser.add_argument("--git-ready-sha256")
    parser.add_argument("--git-ready-case-id")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or not sys.platform.startswith("linux"):
        parser.error("Linux and a command after -- are required")
    if not 0.01 <= args.interval_seconds <= 1 or not 0 <= args.post_exit_seconds <= 5:
        parser.error("invalid diagnostic sample interval or post-exit tail")
    if not args.report.is_absolute() or os.path.lexists(args.report):
        parser.error("report must be a new absolute path")
    ack_args = (args.git_ready_ack, args.git_ready_executable,
                args.git_ready_sha256, args.git_ready_case_id)
    git_ack = None
    if any(value is not None for value in ack_args):
        if not all(value is not None for value in ack_args):
            parser.error("all four Git readiness acknowledgment options are required")
        if (not args.git_ready_ack.is_absolute() or not args.git_ready_executable.is_absolute() or
                args.git_ready_ack == args.report or os.path.lexists(args.git_ready_ack) or
                os.path.lexists(args.git_ready_ack.with_name(args.git_ready_ack.name + '.pending')) or
                not args.git_ready_executable.is_file() or args.git_ready_executable.is_symlink() or
                len(args.git_ready_sha256) != 64 or
                any(c not in '0123456789abcdef' for c in args.git_ready_sha256) or
                args.git_ready_case_id not in {'git-exec-success', 'git-exec-error',
                                              'git-app-server-eof', 'git-app-server-sigterm',
                                              'fallback-exec-success', 'fallback-exec-error',
                                              'fallback-app-server-eof', 'fallback-app-server-sigterm'}):
            parser.error("invalid or existing exact Git acknowledgment binding")
        git_ack = GitAcknowledgment(*ack_args)
    # Refuse before starting a workload if the output cannot be reserved.
    fd = os.open(args.report, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    started = time.monotonic()
    child = None
    held, known, previous = {}, {}, {}
    events, failures = [], set()
    root_identity, exited_at = None, None
    sample_count = 0
    try:
        # Inherit environment/session. Existing wrapper and strict subreaper
        # remain the only owners/waiters for their respective direct children.
        child = subprocess.Popen(command)
        root_identity = identity(child.pid)
        if root_identity is not None:
            known[key(root_identity)] = None
        else:
            failures.add("root_identity_unavailable")
        while True:
            snapshot = {}
            for proc in Path("/proc").iterdir():
                if proc.name.isdecimal():
                    row = identity(int(proc.name))
                    if row is not None:
                        snapshot[row["pid"]] = row
            changed = True
            while changed:
                changed = False
                live = {pid: tick for pid, tick in known
                        if pid in snapshot and snapshot[pid]["start_ticks"] == tick}
                for row in snapshot.values():
                    parent_tick = live.get(row["ppid"])
                    if parent_tick is not None and key(row) not in known:
                        if len(known) >= 4096:
                            failures.add("tracked_identity_cap_reached")
                            continue
                        known[key(row)] = (row["ppid"], parent_tick)
                        changed = True
            for ident, parent in list(known.items()):
                row = snapshot.get(ident[0])
                if row is None or key(row) != ident:
                    observed = None
                else:
                    observed = dict(row, executable=executable(row, held, failures))
                if ident not in previous or previous[ident] != observed:
                    if len(events) < 20000:
                        events.append({
                            "seconds": time.monotonic() - started,
                            "identity": list(ident),
                            "discovered_via_parent_identity": list(parent) if parent else None,
                            "observation": observed,
                        })
                    else:
                        failures.add("event_cap_reached")
                    previous[ident] = observed
                if observed is not None and git_ack is not None:
                    git_ack.observe(row, observed['executable'], held, root_identity)
            sample_count += 1
            returncode = child.poll()  # Only waits our owned wrapper child.
            if returncode is not None:
                if exited_at is None:
                    exited_at = time.monotonic()
                if time.monotonic() - exited_at >= args.post_exit_seconds:
                    break
            time.sleep(args.interval_seconds)
    except Exception as error:
        failures.add("observer_exception_" + type(error).__name__)
    finally:
        # An observer error does not kill/force the wrapped command or replace
        # its status. Root remains responsible for normal workload supervision.
        returncode = child.wait() if child is not None else None
        executables = {}
        for token, item in held.items():
            try:
                before = item["stat"]
                with os.fdopen(item["fd"], "rb", closefd=False) as stream:
                    digest = hashlib.file_digest(stream, "sha256").hexdigest()
                after = os.fstat(item["fd"])
                stable = (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) == (
                    after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
                executables[token] = {
                    "path": item["path"], "sha256": digest if stable else None,
                    "bytes": before.st_size, "metadata_stable_while_hashed": stable,
                }
                if not stable:
                    failures.add("executable_changed_while_hashed")
            except Exception as error:
                failures.add("executable_hash_exception_" + type(error).__name__)
            finally:
                os.close(item["fd"])
        report = {
            "schema": "sampled-descendant-executable-identities-v1",
            "wrapped_root_identity": root_identity,
            "command_returncode": returncode,
            "sample_interval_seconds": args.interval_seconds,
            "post_exit_observation_seconds": args.post_exit_seconds,
            "post_exit_tail_changes_acceptance_drain": False,
            "sample_count": sample_count,
            "diagnostic_complete": not failures,
            "diagnostic_failures": sorted(failures),
            "events": events, "executables": executables,
            "git_readiness_acknowledgment": git_ack.receipt if git_ack is not None else None,
            "limits": [
                "No argv, environment, cwd, process name or socket content captured.",
                "Sampled ancestry may miss short-lived or already-reparented descendants.",
                "PID and start ticks identify generations; exec within one generation can still race a sample.",
                "A missing /proc/PID/exe is not proof of PID absence; state and generation are retained.",
                "Executable digests describe pinned observed files, not argument-level operation attribution.",
                "No wait statuses for descendant processes are invented; retain the unchanged strict report separately.",
                "Diagnostic tail and later disappearance cannot repair a strict drain failure.",
            ],
        }
        with os.fdopen(fd, "w") as output:
            json.dump(report, output, indent=2)
            output.write("\n")
        print(json.dumps({"report": str(args.report), "command_returncode": returncode,
                          "diagnostic_complete": not failures}))
    return 125 if returncode is None else (128 - returncode if returncode < 0 else returncode)


if __name__ == "__main__":
    raise SystemExit(main())
