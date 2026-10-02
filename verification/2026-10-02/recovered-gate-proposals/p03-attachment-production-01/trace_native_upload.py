"""Bounded, payload-free interpretation of one isolated strace attachment run."""

import ast
import hashlib
import os
from pathlib import Path
import re
import stat

MAX_FILES = 128
MAX_BYTES = 16 * 1024 * 1024
MAX_LINES = 100000
SYSCALLS = {"execve", "execveat", "clone", "clone3", "fork", "vfork", "openat", "exit_group"}


class TraceError(Exception):
    """Contains a fixed diagnostic code only; never includes trace contents."""


def need(ok, code):
    if not ok:
        raise TraceError(code)


def arguments(text):
    parts, start, depth, quoted, escaped = [], 0, 0, False, False
    for index, char in enumerate(text):
        if quoted:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
        elif char == '"':
            quoted = True
        elif char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
            need(depth >= 0, "trace_arguments")
        elif char == "," and depth == 0:
            parts.append(text[start:index].strip())
            start = index + 1
    need(not quoted and depth == 0, "trace_arguments")
    return parts + [text[start:].strip()]


def pathname(token):
    need(re.fullmatch(r'"(?:[^"\\]|\\.)*"', token) is not None, "trace_path_truncated")
    try:
        value = ast.literal_eval(token)
    except (SyntaxError, ValueError, RecursionError):
        raise TraceError("trace_path_encoding") from None
    need(type(value) is str and "\x00" not in value, "trace_path_encoding")
    return value


def records(data):
    try:
        lines = data.decode("utf-8").splitlines()
    except UnicodeDecodeError:
        raise TraceError("trace_encoding") from None
    need(not data or data.endswith(b"\n"), "trace_partial_final_line")
    rows, pending = [], None
    for line in lines:
        if not line or re.fullmatch(r"--- SIG[A-Z0-9]+ \{.*\} ---", line):
            continue
        terminal = re.fullmatch(r"\+\+\+ (exited with \d+|killed by SIG[A-Z0-9]+(?: \(core dumped\))?) \+\+\+", line)
        if terminal:
            if terminal[1] != "exited with 0":
                rows.append(("trace_failed_termination", [], ""))
            continue
        if line.endswith(" <unfinished ...>"):
            need(pending is None, "trace_nested_unfinished")
            pending = line[:-len(" <unfinished ...>")]
            continue
        resumed = re.fullmatch(r"<\.\.\. (\w+) resumed>(.*)", line)
        if resumed:
            need(pending is not None and pending.startswith(resumed[1] + "("), "trace_unmatched_resume")
            line, pending = pending + resumed[2], None
        else:
            need(pending is None, "trace_unmatched_unfinished")
        match = re.fullmatch(r"(\w+)\((.*)\)\s+= (.+)", line)
        need(match is not None and match[1] in SYSCALLS, "trace_unrecognized_record")
        rows.append((match[1], arguments(match[2]), match[3].strip()))
    need(pending is None, "trace_unmatched_unfinished")
    return rows


def inspect_trace(prefix: Path, native_executable: Path, staging_root: Path) -> dict:
    """Require exact native exec, owned staged-file read, and leader exit_group(0).

    Caller owns fresh-prefix enforcement, successful tracer/strict-runner status,
    and package/ELF/source before-after hashes. This is not a result-envelope or
    whole-host shutdown proof. No trace lines, paths, arguments or PIDs escape.
    """
    need(native_executable.is_absolute() and staging_root.is_absolute(), "trace_expected_path")
    need(".." not in native_executable.parts and ".." not in staging_root.parts, "trace_expected_path")
    paths = []
    try:
        for path in prefix.parent.iterdir():
            if path.name.startswith(prefix.name + "."):
                need(path.name[len(prefix.name) + 1:].isdecimal(), "trace_filename")
                paths.append(path)
                need(len(paths) <= MAX_FILES, "trace_file_limit")
        need(bool(paths), "trace_missing")
        table, fingerprints, total_bytes, total_lines = {}, [], 0, 0
        for path in paths:
            identity = path.lstat()
            need(stat.S_ISREG(identity.st_mode), "trace_not_regular")
            fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
            with os.fdopen(fd, "rb") as stream:
                before = os.fstat(stream.fileno())
                need((before.st_dev, before.st_ino) == (identity.st_dev, identity.st_ino), "trace_changed")
                data = stream.read(MAX_BYTES - total_bytes + 1)
                after = os.fstat(stream.fileno())
            need((before.st_size, before.st_mtime_ns, before.st_ctime_ns) ==
                 (after.st_size, after.st_mtime_ns, after.st_ctime_ns), "trace_changed")
            total_bytes += len(data)
            total_lines += data.count(b"\n")
            need(total_bytes <= MAX_BYTES, "trace_byte_limit")
            need(total_lines <= MAX_LINES, "trace_line_limit")
            pid = int(path.name[len(prefix.name) + 1:])
            need(pid > 0 and pid not in table, "trace_duplicate_identity")
            table[pid] = records(data)
            fingerprints.append({"sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data),
                                 "lines": data.count(b"\n")})
    except OSError:
        raise TraceError("trace_io_error") from None
    executions, births = [], set()
    for pid, rows in table.items():
        for index, (name, args, result) in enumerate(rows):
            if name in ("clone", "clone3", "fork", "vfork") and re.fullmatch(r"[1-9][0-9]*", result):
                child = int(result)
                need(child not in births, "trace_reused_identity")
                need(child in table, "trace_child_file_missing")
                births.add(child)
            if name in ("execve", "execveat"):
                need(len(args) == (3 if name == "execve" else 5), "trace_exec_arguments")
                path = pathname(args[0 if name == "execve" else 1])
                if result == "0" and path == str(native_executable):
                    executions.append((pid, index))
    need(len(executions) == 1, "trace_native_exec_count")
    leader, exec_index = executions[0]
    owned, queue, staged_opens, leader_exits = {leader}, [(leader, exec_index + 1)], 0, 0
    while queue:
        pid, start = queue.pop()
        rows = table[pid]
        for index in range(start, len(rows)):
            name, args, result = rows[index]
            need(name != "trace_failed_termination", "trace_native_termination")
            need(not (name in ("execve", "execveat") and result == "0"), "trace_owned_reexec")
            if name in ("clone", "clone3") and re.fullmatch(r"[1-9][0-9]*", result):
                flags = re.search(r"\bflags=([^,}]+)", ",".join(args))
                need(flags is not None, "trace_clone_flags")
                if "CLONE_THREAD" in flags[1].split("|"):
                    child = int(result)
                    need(child in table and child not in owned, "trace_thread_identity")
                    owned.add(child)
                    queue.append((child, 0))
            if name == "openat":
                need(len(args) in (3, 4), "trace_open_arguments")
                path = Path(pathname(args[1]))
                is_staged = (path.is_absolute() and ".." not in path.parts and path.name == "input.bin"
                             and path.parent.parent == staging_root
                             and path.parent.name.startswith(".attachment-transfer-")
                             and len(path.parent.name) > len(".attachment-transfer-"))
                if is_staged and re.fullmatch(r"[0-9]+", result):
                    flags = set(args[2].split("|"))
                    need("O_RDONLY" in flags and flags <= {"O_RDONLY", "O_CLOEXEC", "O_LARGEFILE",
                         "O_NOFOLLOW", "O_NONBLOCK", "O_NOCTTY", "O_NOATIME"}, "trace_staging_not_readonly")
                    staged_opens += 1
            if name == "exit_group":
                need(args == ["0"] and result == "?" and index == len(rows) - 1, "trace_native_exit")
                if pid == leader:
                    leader_exits += 1
    need(staged_opens >= 1, "trace_native_staged_read_missing")
    need(leader_exits == 1, "trace_native_exit_missing")
    return {"schema": "native-attachment-trace-v1", "native_exec_count": 1,
            "owned_thread_count": len(owned) - 1, "staged_read_open_count": staged_opens,
            "native_leader_exit_zero": True, "native_upload_path_invoked": True,
            "native_result_envelope_proven": False, "whole_host_clean": False,
            "trace_file_count": len(table), "trace_bytes": total_bytes, "trace_lines": total_lines,
            "trace_fingerprints": sorted(fingerprints, key=lambda item: item["sha256"])}
