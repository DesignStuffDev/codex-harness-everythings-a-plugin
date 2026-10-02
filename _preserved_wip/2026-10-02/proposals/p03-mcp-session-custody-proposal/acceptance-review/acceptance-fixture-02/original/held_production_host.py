#!/usr/bin/env python3
"""Unexecuted Linux acceptance fixture for rebuilt full-CLI process-final owners.

Runs eight isolated cases under the unchanged strict subreaper and frozen
executable observer. Never forwards proxy traffic or substitutes a fake Git.
Only allowlisted receipt fields, counters, hashes and identities are persisted.
See README.md for the required external session/MCP receipt bindings and limits.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import signal
import socket
import subprocess
import sys
import threading
import time


PATHS = ("exec-success", "exec-error", "app-server-eof", "app-server-sigterm")
MODES = ("git", "fallback")
GRACEFUL, HARD, OUTER, READY = 200, 205, 210, 60
OBSERVER_SHA = "334d2f2cac92c124b77c2865f7fedd96198eb0d9d1ea00d42be2ba19a000892e"
TARGET = "codex_core_plugins::lifecycle"
EVENT = "curated_process_shutdown_observed"
CURATED = {
    "receipt_version": int, "worker_generation": int, "worker_disposition": str,
    "worker_quarantined": bool, "worker_unexpected_handles": int,
    "callback_pending": int, "callback_completed": int,
    "callback_suppressed": int, "callback_failed": int,
    "curated_ownership_clean": bool, "optional_sync_outcome": str,
}
LIMITS = [
    "CONNECT establishment only; no held TLS response-body claim",
    "session/MCP evidence is independently bound; callback counts cannot replace it",
    "exact internal first-stop/deadline origin is not observed by the harness",
    "post-stop publication and callback-admission counters are unverified",
    "sampled Git identities do not prove all descendants or an exact Git operation",
    "no same-process same-home App Server A-to-B replacement or refresh-delivery proof",
    "no GUI automation, slow-storage/backend hold, or whole-host-clean claim",
    "MCP exact-handle custody remains unverified unless an independent future receipt proves it",
]


class AcceptanceError(Exception):
    pass


def require(condition, code):
    if not condition:
        raise AcceptanceError(code)


def digest(path):
    with open(path, "rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, data):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump(data, stream, indent=2, sort_keys=True)
        stream.write("\n")


def identity(pid):
    try:
        raw = Path(f"/proc/{pid}/stat").read_text()
        fields = raw[raw.rindex(")") + 2:].split()
        return {"pid": pid, "start_ticks": int(fields[19]),
                "ppid": int(fields[1]), "pgid": int(fields[2]), "state": fields[0]}
    except (OSError, ValueError, IndexError):
        return None


def load_schema(path):
    schema = json.loads(Path(path).read_text())
    require(schema.get("schema") == "held-host-log-bindings-v1", "receipt_schema_version")
    require(schema.get("curated") == {"target": TARGET, "event": EVENT, "receipt_version": 1},
            "curated_schema_binding_mismatch")
    for final_path in PATHS:
        bindings = schema.get("paths", {}).get(final_path, {})
        require(set(bindings) == {"session_loop", "mcp"}, "component_binding_declarations_required")
        for component, binding in bindings.items():
            if binding is None:
                continue  # Explicitly unverified, never converted into a pass.
            for key in ("target", "event"):
                require(isinstance(binding.get(key), str) and
                        re.fullmatch(r"[A-Za-z_][A-Za-z0-9_:.-]*", binding[key]),
                        "invalid_session_mcp_event_binding")
            require(binding["target"] != TARGET or binding["event"] != EVENT,
                    "curated_receipt_cannot_prove_session_mcp_drain")
            require(component != "mcp" or binding["event"] != "session_shutdown_report_observed",
                    "session_loop_report_cannot_prove_exact_mcp_custody")
            fields = binding.get("fields", {})
            wanted = ({"receipt_version", "completed", "submit_failed", "timed_out", "report_clean"}
                      if component == "session_loop" else {"pending", "failed", "drained", "exact_handle_custody"})
            require(set(fields) == wanted, "session_mcp_required_fields_missing")
            require(all(isinstance(value, str) and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", value)
                        for value in fields.values()), "invalid_session_mcp_field_binding")
    return schema


def log_fields(line, target, event, wanted):
    """Read ordinary tracing JSON or fmt output, retaining no raw line."""
    text = re.sub(r"\x1b\[[0-9;]*m", "", line.decode("utf-8", "replace"))
    try:
        record = json.loads(text)
    except ValueError:
        record = None
    if isinstance(record, dict):
        fields = record.get("fields", {})
        if record.get("target") != target or fields.get("event") != event:
            return None
    else:
        if not re.search(r"(?<![A-Za-z0-9_:])" + re.escape(target) + r"(?=:|\s)", text):
            return None
        fields = dict((match.group(1), match.group(2) if match.group(2) is not None else match.group(3))
                      for match in re.finditer(r'(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=(?:"([^"\n]*)"|([^\s]+))', text))
        if fields.get("event") != event:
            return None
    result = {}
    for key, kind in wanted.items():
        require(key in fields, "matching_receipt_missing_required_field")
        value = fields[key]
        if kind is bool and value in ("true", "false"):
            value = value == "true"
        elif kind is int and isinstance(value, str) and re.fullmatch(r"[0-9]+", value):
            value = int(value)
        require(type(value) is kind and (kind is not int or value >= 0), "receipt_field_type")
        result[key] = value
    return result


class Receipts:
    def __init__(self, bindings):
        self.bindings = bindings
        self.curated, self.sessions, self.errors = [], {"session_loop": [], "mcp": []}, []

    def consume(self, line):
        try:
            found = log_fields(line, TARGET, EVENT, CURATED)
            if found is not None:
                require(len(self.curated) < 64, "curated_receipt_limit")
                self.curated.append(dict(found, observed_monotonic_ns=time.monotonic_ns()))
            for component, binding in self.bindings.items():
                if binding is None:
                    continue
                kinds = ({"receipt_version": int, "completed": int, "submit_failed": int, "timed_out": int,
                          "report_clean": bool} if component == "session_loop" else
                         {"pending": int, "failed": int, "drained": bool, "exact_handle_custody": bool})
                types = {binding["fields"][key]: kind for key, kind in kinds.items()}
                found = log_fields(line, binding["target"], binding["event"], types)
                if found is not None:
                    require(len(self.sessions[component]) < 64, "session_receipt_limit")
                    self.sessions[component].append({key: found[field]
                                                    for key, field in binding["fields"].items()})
        except AcceptanceError as error:
            if len(self.errors) < 64:
                self.errors.append(str(error))

    def validate(self):
        require(not self.errors, "invalid_matching_receipt")
        require(bool(self.curated), "missing_curated_worker_callback_receipt")
        final = self.curated[-1]
        require(final["receipt_version"] == 1 and final["worker_generation"] > 0,
                "held_worker_generation_not_observed")
        require(final["worker_disposition"] == "joined" and
                final["optional_sync_outcome"] in ("succeeded", "failed"), "worker_exact_join_missing")
        require(not final["worker_quarantined"] and final["worker_unexpected_handles"] == 0 and
                final["callback_pending"] == 0 and final["callback_failed"] == 0 and
                final["curated_ownership_clean"], "curated_ownership_unclean")
        for component, status in self.component_status().items():
            require(status != "reported_failed", component + "_reported_failed")

    def component_status(self):
        result = {}
        for component, rows in self.sessions.items():
            if not rows:
                result[component] = "unverified_no_receipt"
            elif component == "session_loop":
                row = rows[-1]
                clean = (row["receipt_version"] == 1 and row["submit_failed"] == 0 and row["timed_out"] == 0 and
                         row["report_clean"])
                result[component] = "session_loop_report_clean_not_mcp_proof" if clean else "reported_failed"
            else:
                row = rows[-1]
                clean = row == {"pending": 0, "failed": 0, "drained": True, "exact_handle_custody": True}
                result[component] = "exact_custody_receipt_clean" if clean else "reported_failed"
        return result

    def report(self):
        return {"schema": "held-host-normalized-receipts-v1", "curated": self.curated,
                "independent_session_mcp": self.sessions, "component_status": self.component_status(),
                "parse_failures": self.errors, "whole_host_clean": False}



class PipeDrain:
    def __init__(self, pipe, consumer=None):
        self.pipe, self.consumer = pipe, consumer
        self.stop, self.eof = threading.Event(), threading.Event()
        self.hasher, self.bytes, self.errors = hashlib.sha256(), 0, []
        self.thread = threading.Thread(target=self.run, name="held-host-pipe")
        self.thread.start()

    def run(self):
        pending = b""
        try:
            os.set_blocking(self.pipe.fileno(), False)
            with selectors.DefaultSelector() as selected:
                selected.register(self.pipe.fileno(), selectors.EVENT_READ)
                while not self.stop.is_set():
                    if not selected.select(0.1):
                        continue
                    data = os.read(self.pipe.fileno(), 65536)
                    if not data:
                        if pending and self.consumer:
                            self.consumer(pending)
                        self.eof.set()
                        return
                    self.hasher.update(data)
                    self.bytes += len(data)
                    if self.consumer:
                        pending += data
                        require(len(pending) <= 2 * 1024 * 1024, "pipe_line_limit")
                        while b"\n" in pending:
                            line, pending = pending.split(b"\n", 1)
                            self.consumer(line)
        except Exception as error:
            self.errors.append(type(error).__name__)

    def close(self):
        self.thread.join(2)
        if self.thread.is_alive():
            self.errors.append("pipe_eof_not_observed")
            self.stop.set()
            self.thread.join(2)
        require(not self.thread.is_alive(), "fixture_pipe_thread_not_joined")
        self.pipe.close()

    def report(self):
        return {"bytes": self.bytes, "sha256": self.hasher.hexdigest(),
                "eof": self.eof.is_set(), "errors": self.errors}


class GitSampler:
    """Read identities and pinned executable bytes only; never signal or reap."""
    def __init__(self, root, expected_sha, trusted_git, seen):
        self.root, self.expected_sha = root, expected_sha
        self.expected_stat = Path(trusted_git).stat()
        self.stop, self.seen = threading.Event(), seen
        self.rows, self.errors = [], []
        self.thread = threading.Thread(target=self.run, name="held-real-git-observer")
        self.thread.start()

    def run(self):
        known = {(self.root["pid"], self.root["start_ticks"])}
        try:
            while not self.stop.is_set():
                snapshot = {int(path.name): identity(int(path.name))
                            for path in Path("/proc").iterdir() if path.name.isdecimal()}
                snapshot = {pid: row for pid, row in snapshot.items() if row is not None}
                changed = True
                while changed:
                    changed = False
                    live = {pid for pid, ticks in known if pid in snapshot and snapshot[pid]["start_ticks"] == ticks}
                    for row in snapshot.values():
                        key = (row["pid"], row["start_ticks"])
                        if row["ppid"] in live and key not in known:
                            require(len(known) < 4096, "identity_sample_limit")
                            known.add(key)
                            changed = True
                if not self.seen.is_set():
                    for pid, ticks in known:
                        row = snapshot.get(pid)
                        if row is None or row["start_ticks"] != ticks or pid == self.root["pid"]:
                            continue
                        try:
                            with open(f"/proc/{pid}/exe", "rb") as stream:
                                before = os.fstat(stream.fileno())
                                if (before.st_dev, before.st_ino) != (self.expected_stat.st_dev, self.expected_stat.st_ino):
                                    continue
                                sha = hashlib.file_digest(stream, "sha256").hexdigest()
                                after = os.fstat(stream.fileno())
                            refreshed = identity(pid)
                            if sha == self.expected_sha and refreshed and refreshed["start_ticks"] == ticks and (
                                    before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) == (
                                    after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns):
                                self.rows.append(dict(row, sha256=sha, observed_monotonic_ns=time.monotonic_ns()))
                                self.seen.set()
                                break
                        except OSError:
                            continue
                self.stop.wait(0.02)
        except Exception as error:
            self.errors.append(type(error).__name__)

    def close(self):
        self.stop.set()
        self.thread.join(5)
        require(not self.thread.is_alive(), "fixture_sampler_thread_not_joined")


class LoopbackServer:
    def __init__(self, handler):
        self.handler, self.stop, self.errors = handler, threading.Event(), []
        self.lock, self.connections, self.threads = threading.Lock(), [], []
        self.listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen(16)
        self.listener.settimeout(0.2)
        self.port = self.listener.getsockname()[1]
        self.thread = threading.Thread(target=self.accept, name="held-loopback-listener")
        self.thread.start()

    def accept(self):
        try:
            while not self.stop.is_set():
                try:
                    conn, _ = self.listener.accept()
                except socket.timeout:
                    continue
                conn.settimeout(0.2)
                with self.lock:
                    require(len(self.threads) < 128, "fixture_connection_limit")
                    self.connections.append(conn)
                    thread = threading.Thread(target=self.connection, args=(conn,), name="held-loopback-connection")
                    self.threads.append(thread)
                    thread.start()
        except OSError:
            if not self.stop.is_set():
                self.errors.append("listener_os_error")
        except Exception as error:
            self.errors.append(type(error).__name__)

    def connection(self, conn):
        try:
            with conn:
                self.handler(self, conn)
        except Exception as error:
            if not self.stop.is_set():
                self.errors.append(str(error) if isinstance(error, AcceptanceError) else type(error).__name__)

    def request(self, conn):
        data, deadline = b"", time.monotonic() + READY
        while b"\r\n\r\n" not in data:
            require(not self.stop.is_set() and time.monotonic() < deadline, "fixture_header_timeout")
            try:
                part = conn.recv(2048)
            except socket.timeout:
                continue
            require(bool(part), "fixture_incomplete_header")
            data += part
            require(len(data) <= 16384, "fixture_header_limit")
        header, body = data.split(b"\r\n\r\n", 1)
        lines = header.split(b"\r\n")
        words = lines[0].split(b" ")
        require(len(words) == 3, "fixture_request_line")
        headers = {}
        for line in lines[1:]:
            key, separator, value = line.partition(b":")
            require(bool(separator), "fixture_invalid_header")
            headers[key.lower()] = value.strip()
        return words[0], words[1], headers, body

    def close(self):
        self.stop.set()
        self.listener.close()
        with self.lock:
            for conn in self.connections:
                try:
                    conn.shutdown(socket.SHUT_RDWR)
                except OSError:
                    pass
        self.thread.join(3)
        require(not self.thread.is_alive(), "fixture_listener_not_joined")
        for thread in self.threads:
            thread.join(3)
            require(not thread.is_alive(), "fixture_connection_not_joined")
        for conn in self.connections:
            conn.close()


class HeldProxy:
    def __init__(self, mode):
        self.mode, self.lock = mode, threading.Lock()
        self.ready, self.git_seen = threading.Event(), threading.Event()
        self.stop_ns, self.held, self.events = None, 0, []
        self.git_failures = 0
        self.server = LoopbackServer(self.handle)

    def record(self, name):
        self.events.append({"event": name, "monotonic_ns": time.monotonic_ns()})

    def handle(self, server, conn):
        method, authority, _, extra = server.request(conn)
        require(method == b"CONNECT" and not extra, "unexpected_proxy_request")
        require(authority in (b"github.com:443", b"api.github.com:443"), "unexpected_proxy_authority")
        with self.lock:
            require(self.stop_ns is None, "new_proxy_request_after_stop_trigger")
            self.record("git_connect" if authority == b"github.com:443" else "fallback_connect")
        if authority == b"github.com:443":
            deadline = time.monotonic() + READY
            while not self.git_seen.wait(0.1):
                require(not server.stop.is_set() and time.monotonic() < deadline,
                        "real_git_identity_not_observed")
            if self.mode == "fallback":
                with self.lock:
                    self.git_failures += 1
                    self.record("git_connect_failed_502")
                conn.sendall(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                return
        else:
            require(self.mode == "fallback" and self.git_failures > 0, "fallback_without_real_git_failure")
        with self.lock:
            self.held += 1
            self.record("held_connect_ready")
            self.ready.set()
        try:
            # Intentionally send no CONNECT response and never forward bytes.
            while not server.stop.is_set():
                try:
                    data = conn.recv(1)
                except socket.timeout:
                    continue
                require(not data, "unexpected_bytes_while_connect_held")
                with self.lock:
                    self.record("held_peer_eof")
                return
        finally:
            with self.lock:
                self.held -= 1

    def trigger(self):
        with self.lock:
            require(self.held > 0, "held_transport_gone_before_trigger")
            self.stop_ns = time.monotonic_ns()
            self.record("host_stop_trigger")
            return self.stop_ns

    def report(self):
        with self.lock:
            return {"mode": self.mode, "held_connections": self.held,
                    "events": list(self.events), "git_502_count": self.git_failures,
                    "errors": list(self.server.errors)}


class ModelFixture:
    def __init__(self, fail):
        self.fail, self.ready, self.release = fail, threading.Event(), threading.Event()
        self.requests, self.terminal_sent = 0, False
        self.server = LoopbackServer(self.handle)

    def handle(self, server, conn):
        method, path, headers, body = server.request(conn)
        require(method == b"POST" and path == b"/v1/responses", "unexpected_model_route")
        require(b"transfer-encoding" not in headers, "unsupported_model_request_framing")
        length = int(headers.get(b"content-length", b"0"))
        require(0 <= length <= 8 * 1024 * 1024, "model_request_body_limit")
        deadline = time.monotonic() + READY
        while len(body) < length:
            require(not server.stop.is_set() and time.monotonic() < deadline, "model_body_timeout")
            try:
                part = conn.recv(min(65536, length - len(body)))
            except socket.timeout:
                continue
            require(bool(part), "incomplete_model_body")
            body += part
        del headers, body  # Neither credentials nor request content enters evidence.
        self.requests += 1
        require(self.requests == 1, "unexpected_model_retry")
        conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n")
        self.ready.set()
        while not self.release.wait(0.1):
            if server.stop.is_set():
                return
        events = [{"type": "response.created", "response": {"id": "held-fixture-response"}}]
        if self.fail:
            events.append({"type": "response.failed", "response": {"id": "held-fixture-response",
                           "error": {"code": "rate_limit_exceeded", "message": "synthetic server error"}}})
        else:
            events.extend([
                {"type": "response.output_item.done", "item": {"type": "message", "role": "assistant",
                 "id": "held-fixture-message", "content": [{"type": "output_text", "text": "ready"}]}},
                {"type": "response.completed", "response": {"id": "held-fixture-response", "usage": {
                 "input_tokens": 0, "input_tokens_details": None, "output_tokens": 0,
                 "output_tokens_details": None, "total_tokens": 0}}},
            ])
        conn.sendall(b"".join(b"data: " + json.dumps(event).encode() + b"\n\n" for event in events))
        self.terminal_sent = True


def child_environment(home, proxy_port, bindings):
    env = os.environ.copy()
    for key in list(env):
        if key.startswith("GIT_CONFIG_") or key in ("GIT_CONFIG", "GIT_ASKPASS", "SSH_ASKPASS",
                "GIT_SSH", "GIT_SSH_COMMAND", "GIT_SSL_NO_VERIFY", "OPENAI_API_KEY", "CODEX_API_KEY"):
            env.pop(key, None)
    empty = home / "empty-git-config"
    empty.write_text("")
    proxy = f"http://127.0.0.1:{proxy_port}"
    for key in ("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"):
        env[key] = proxy
    env.update({"NO_PROXY": "127.0.0.1,localhost,::1", "no_proxy": "127.0.0.1,localhost,::1",
                "CODEX_HOME": str(home), "CODEX_SQLITE_HOME": str(home), "CODEX_API_KEY": "dummy",
                "GIT_CONFIG_GLOBAL": str(empty), "GIT_CONFIG_SYSTEM": str(empty), "GIT_CONFIG_NOSYSTEM": "1",
                "GIT_TERMINAL_PROMPT": "0", "NO_COLOR": "1", "LOG_FORMAT": "json"})
    targets = sorted({TARGET} | {binding["target"] for binding in bindings.values() if binding is not None})
    env["RUST_LOG"] = "error," + ",".join(target + "=info" for target in targets)
    return env


def wait_ready(event, host, deadline, servers):
    while not event.wait(0.02):
        require(host.poll() is None, "host_exited_before_fixture_ready")
        require(time.monotonic() < deadline, "fixture_unreachable_with_current_policy")
        require(not any(server.errors for server in servers), "fixture_failed_before_readiness")


def run_case(config):
    case_dir = Path(config["case_dir"])
    home, cwd = case_dir / "home", case_dir / "cwd"
    home.mkdir(mode=0o700)
    cwd.mkdir(mode=0o700)
    schema = load_schema(config["receipt_schema"])
    receipts = Receipts(schema["paths"][config["final_path"]])
    proxy, model, host, sampler, pidfd = None, None, None, None, None
    drains, cleanup_errors, status = [], [], None
    report = {"schema": "held-production-host-case-v1", "case_id": config["case_id"],
              "mode": config["mode"], "final_path": config["final_path"],
              "policy_seconds": {"graceful": GRACEFUL, "native_hard": HARD, "launcher_outer": OUTER},
              "source_commit": config["source_commit"], "source_tree": config["source_tree"],
              "binary_sha256": config["binary_sha256"], "unverified": LIMITS,
              "curated_runtime_gates_passed": False, "whole_host_clean": False, "forced_fixture_cleanup": False}
    failure = None
    try:
        require(digest(config["binary"]) == config["binary_sha256"], "binary_changed_before_case")
        report["binary_bytes_before"] = Path(config["binary"]).stat().st_size
        proxy = HeldProxy(config["mode"])
        is_exec = config["final_path"].startswith("exec-")
        if is_exec:
            model = ModelFixture(config["final_path"] == "exec-error")
            (home / "config.toml").write_text(
                'model = "gpt-5.1-codex"\nmodel_provider = "held_fixture"\n'
                '[model_providers.held_fixture]\nname = "held fixture"\n'
                f'base_url = "http://127.0.0.1:{model.server.port}/v1"\n'
                'wire_api = "responses"\nrequires_openai_auth = false\nsupports_websockets = false\n')
            argv = [config["binary"], "exec", "--skip-git-repo-check", "--color", "never",
                    "--json", "Return the word ready."]
        else:
            argv = [config["binary"], "app-server", "--stdio"]
        report["host_argv"] = argv
        env = child_environment(home, proxy.server.port, receipts.bindings)
        host = subprocess.Popen(argv, cwd=cwd, env=env,
                                stdin=subprocess.DEVNULL if is_exec else subprocess.PIPE,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        pidfd = os.pidfd_open(host.pid)
        report["host_identity"] = identity(host.pid)
        require(report["host_identity"] is not None, "host_identity_unavailable")
        sampler = GitSampler(report["host_identity"], config["trusted_git_sha256"],
                             config["trusted_git"], proxy.git_seen)
        initialized = threading.Event()

        def stdout_line(line):
            if not is_exec:
                try:
                    message = json.loads(line)
                except ValueError:
                    return
                if isinstance(message, dict) and message.get("id") == 1 and "result" in message:
                    initialized.set()

        drains.append(PipeDrain(host.stdout, stdout_line))
        drains.append(PipeDrain(host.stderr, receipts.consume))
        if not is_exec:
            request = {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "clientInfo": {"name": "held-production-host", "version": "1"}}}
            host.stdin.write(json.dumps(request).encode() + b"\n")
            host.stdin.flush()
        deadline = time.monotonic() + READY
        servers = [proxy.server] + ([model.server] if model else [])
        wait_ready(proxy.ready, host, deadline, servers)
        wait_ready(sampler.seen, host, deadline, servers)
        wait_ready(model.ready if model else initialized, host, deadline, servers)
        require(host.poll() is None, "host_exited_before_stop_trigger")
        if not is_exec:
            host.stdin.write(b'{"jsonrpc":"2.0","method":"initialized"}\n')
            host.stdin.flush()
        report["stop_trigger_monotonic_ns"] = proxy.trigger()
        if model:
            model.release.set()
        elif config["final_path"] == "app-server-eof":
            host.stdin.close()
        else:
            signal.pidfd_send_signal(pidfd, signal.SIGTERM)
        remaining = OUTER - (time.monotonic_ns() - report["stop_trigger_monotonic_ns"]) / 1e9
        status = host.wait(timeout=max(0, remaining))
        report["host_exit_monotonic_ns"] = time.monotonic_ns()
        elapsed = (report["host_exit_monotonic_ns"] - report["stop_trigger_monotonic_ns"]) / 1e9
        report["seconds_from_external_trigger_to_exit"] = elapsed
        require(status != 124, "hard_watchdog_cleanup_durability_unconfirmed")
        require(status != 125, "watchdog_unavailable_cleanup_durability_unconfirmed")
        require(status != 126, "forced_shutdown_cleanup_durability_unconfirmed")
        require(status == (1 if config["final_path"] == "exec-error" else 0), "original_exit_status_changed")
        require(elapsed <= GRACEFUL, "graceful_trigger_budget_exceeded")
        if model:
            require(model.terminal_sent, "model_terminal_not_released")
        for drain in drains:
            drain.close()
        require(not any(drain.errors for drain in drains), "host_log_pipe_incomplete")
        receipts.validate()
        # This tail only observes peer EOF; it never releases a held connection.
        eof_deadline = time.monotonic() + 1
        while proxy.report()["held_connections"] and time.monotonic() < eof_deadline:
            time.sleep(0.01)
        require(proxy.report()["held_connections"] == 0, "held_connection_live_after_host_exit")
        require(not proxy.server.errors and (not model or not model.server.errors), "fixture_protocol_failed")
        require(not sampler.errors and bool(sampler.rows), "trusted_git_observation_missing")
        report["curated_runtime_gates_passed"] = True
    except Exception as error:
        failure = str(error) if isinstance(error, AcceptanceError) else type(error).__name__
        # Preserve the pre-cleanup failure before any kill or proxy release.
        write_json(case_dir / "failure-before-cleanup.json", {
            "reason": failure, "host_status_before_cleanup": host.poll() if host else None,
            "proxy_before_cleanup": proxy.report() if proxy else None,
            "observed_at_monotonic_ns": time.monotonic_ns(),
        })
    finally:
        if host and host.poll() is None:
            report["forced_fixture_cleanup"] = True
            try:
                # A pidfd targets the exact owned process, never a recycled PID/group.
                if pidfd is not None:
                    signal.pidfd_send_signal(pidfd, signal.SIGKILL)
                else:
                    host.kill()
                host.wait(timeout=5)
            except Exception as error:
                cleanup_errors.append(type(error).__name__)
        if host and host.stdin and not host.stdin.closed:
            host.stdin.close()
        for item in (sampler, model.server if model else None, proxy.server if proxy else None):
            if item:
                try:
                    item.close()
                except Exception as error:
                    cleanup_errors.append(type(error).__name__)
        for drain in drains:
            if not drain.pipe.closed:
                try:
                    drain.close()
                except Exception as error:
                    cleanup_errors.append(type(error).__name__)
        if pidfd is not None:
            os.close(pidfd)
    report.update(host_original_returncode=status, failure=failure, cleanup_errors=cleanup_errors,
                  proxy=proxy.report() if proxy else None,
                  trusted_git_observations=sampler.rows if sampler else [],
                  stdout=drains[0].report() if drains else None,
                  stderr=drains[1].report() if len(drains) > 1 else None)
    report["independent_component_status"] = receipts.component_status()
    report["binary_bytes_after"] = Path(config["binary"]).stat().st_size
    report["binary_sha256_after"] = digest(config["binary"])
    if (cleanup_errors or (proxy and proxy.server.errors) or (model and model.server.errors)
            or report["binary_sha256_after"] != config["binary_sha256"]):
        report["curated_runtime_gates_passed"] = False
    write_json(case_dir / "receipts.json", receipts.report())
    report["normalized_receipt_sha256"] = digest(case_dir / "receipts.json")
    write_json(case_dir / "case-result.json", report)
    return 0 if report["curated_runtime_gates_passed"] else 1


def main():
    os.umask(0o077)
    if len(sys.argv) == 3 and sys.argv[1] == "--case-config":
        return run_case(json.loads(Path(sys.argv[2]).read_text()))
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("binary", "trusted-git", "observer", "strict-runner", "receipt-schema", "source-binding", "output-dir"):
        parser.add_argument("--" + name, required=True, type=Path)
    for name in ("binary-sha256", "trusted-git-sha256", "strict-runner-sha256", "source-commit", "source-tree"):
        parser.add_argument("--" + name, required=True)
    args = parser.parse_args()
    require(sys.platform.startswith("linux") and hasattr(os, "pidfd_open") and
            hasattr(signal, "pidfd_send_signal"), "linux_pidfd_support_required")
    for name in ("binary", "trusted_git", "observer", "strict_runner", "receipt_schema", "source_binding", "output_dir"):
        require(getattr(args, name).is_absolute(), "absolute_paths_required")
    require(not args.output_dir.exists(), "output_directory_must_be_new")
    require(bool(re.fullmatch(r"[0-9a-f]{40}", args.source_commit)) and
            bool(re.fullmatch(r"[0-9a-f]{40}", args.source_tree)), "source_binding_format")
    for path, sha in ((args.binary, args.binary_sha256), (args.trusted_git, args.trusted_git_sha256),
                      (args.observer, OBSERVER_SHA), (args.strict_runner, args.strict_runner_sha256)):
        require(bool(re.fullmatch(r"[0-9a-f]{64}", sha)) and digest(path) == sha, "input_hash_binding_mismatch")
    load_schema(args.receipt_schema)
    args.output_dir.mkdir(mode=0o700)
    binding = {key: str(value) if isinstance(value, Path) else value for key, value in vars(args).items()}
    binding.update(schema="held-production-host-invocation-v1", script_sha256=digest(__file__),
                   receipt_schema_sha256=digest(args.receipt_schema), source_binding_sha256=digest(args.source_binding),
                   observer_sha256=OBSERVER_SHA, source_labels_independently_verified_by_this_script=False)
    write_json(args.output_dir / "invocation.json", binding)
    results = []
    for mode in MODES:
        for final_path in PATHS:
            case_id = mode + "-" + final_path
            case_dir = args.output_dir / case_id
            case_dir.mkdir(mode=0o700)
            config = dict(binding, case_id=case_id, case_dir=str(case_dir), mode=mode, final_path=final_path)
            config_path = case_dir / "case-config.json"
            write_json(config_path, config)
            observer_report, strict_report = case_dir / "observer.json", case_dir / "strict.json"
            argv = [sys.executable, str(args.observer), "--report", str(observer_report), "--",
                    sys.executable, str(args.strict_runner), "--report", str(strict_report), "--",
                    sys.executable, str(Path(__file__).resolve()), "--case-config", str(config_path)]
            # No drain override: retain the strict runner's original five seconds.
            original_status = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            result = {"case_id": case_id, "wrapper_original_returncode": original_status,
                      "curated_runtime_gates_passed": False, "strict_argv": argv}
            try:
                case = json.loads((case_dir / "case-result.json").read_text())
                strict = json.loads(strict_report.read_text())
                observer = json.loads(observer_report.read_text())
                result.update(host_original_returncode=case["host_original_returncode"],
                              strict_original_returncode=strict["command_returncode"],
                              strict_exit_status=strict["exit_status"], strict_runner_error=strict["runner_error"],
                              strict_report_sha256=digest(strict_report), observer_report_sha256=digest(observer_report),
                              case_report_sha256=digest(case_dir / "case-result.json"),
                              receipt_path=str(case_dir / "receipts.json"), receipt_sha256=digest(case_dir / "receipts.json"))
                require(original_status == 0 and strict["command_returncode"] == 0 and
                        strict["runner_error"] is None and strict["exit_status"] == 0, "strict_descendant_gate_failed")
                require(observer["diagnostic_complete"] and observer["command_returncode"] == original_status,
                        "frozen_observer_incomplete")
                require(any(item.get("sha256") == args.trusted_git_sha256 and item.get("metadata_stable_while_hashed")
                            for item in observer["executables"].values()), "frozen_observer_real_git_missing")
                require(case["curated_runtime_gates_passed"] and not case["forced_fixture_cleanup"], "case_owned_gate_failed")
                result["curated_runtime_gates_passed"] = True
            except Exception as error:
                result["failure"] = str(error) if isinstance(error, AcceptanceError) else type(error).__name__
            results.append(result)
            # A failed case may leave descendants; never run later cases over that uncertainty.
            if not result["curated_runtime_gates_passed"]:
                break
        if results and not results[-1]["curated_runtime_gates_passed"]:
            break
    passed = len(results) == 8 and all(result["curated_runtime_gates_passed"] for result in results)
    write_json(args.output_dir / "acceptance.json", {
        "schema": "held-production-host-matrix-v1", "all_eight_curated_runtime_gates_passed": passed,
        "whole_host_clean": False,
        "status": "curated_runtime_gates_passed_with_unverified_scope" if passed else "failed_or_incomplete",
        "cases": results, "unverified": LIMITS, "invocation_sha256": digest(args.output_dir / "invocation.json"),
    })
    return 0 if passed else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AcceptanceError as error:
        print("Acceptance preflight failed: " + str(error), file=sys.stderr)
        raise SystemExit(2)
