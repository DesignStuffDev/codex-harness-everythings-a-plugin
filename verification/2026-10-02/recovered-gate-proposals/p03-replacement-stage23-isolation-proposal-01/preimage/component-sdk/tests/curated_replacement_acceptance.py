#!/usr/bin/env python3
"""Own three isolated A→B test-ELF children and real smart-HTTP Git fixtures.

Run beneath the unchanged SDK strict subreaper. This fixture never certifies
whole-host/MCP closure, and any forced fixture cleanup makes acceptance fail.
"""

import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import threading
import time
from urllib.parse import urlsplit

CASE_NAME = "in_process::curated_replacement_tests::production_replacement_child"
GIT_URL = "https://github.com/openai/plugins.git"
CASES = ("pending", "replay", "race")
LIMIT = 8 * 1024 * 1024


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def trusted_git():
    # Linux mirror of pinned codex-utils-path/system_commands.rs; never PATH.
    roots = tuple(Path(p) for p in ("/usr/bin", "/bin", "/usr/sbin", "/sbin",
                                   "/usr/local", "/opt/homebrew", "/opt/local",
                                   "/Library/Developer/CommandLineTools",
                                   "/Applications/Xcode.app/Contents/Developer", "/nix/store"))
    directories = ("/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin",
                   "/run/current-system/sw/bin", "/nix/var/nix/profiles/default/bin",
                   "/Library/Developer/CommandLineTools/usr/bin",
                   "/Applications/Xcode.app/Contents/Developer/usr/bin",
                   "/usr/bin", "/bin", "/usr/sbin", "/sbin")
    for directory in directories:
        parent = Path(directory).resolve()
        candidate = (parent / "git").resolve()
        if (any(parent.is_relative_to(root) for root in roots)
                and any(candidate.is_relative_to(root) for root in roots)
                and candidate.is_file() and os.access(candidate, os.X_OK)):
            return candidate
    raise RuntimeError("production trusted Git lookup has no supported Linux executable")


def fixture_git_env():
    # Only fixture repository creation/backend service use this isolated config.
    # The production child keeps system Git policy and receives the agreed
    # ordinary user-global URL rewrite. No proxy or sandbox variables change.
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update({"GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_SYSTEM": os.devnull,
                "GIT_TERMINAL_PROMPT": "0"})
    return env


def observe_held_git(child_pid, git):
    processes = {}
    for path in Path("/proc").iterdir():
        if not path.name.isdecimal():
            continue
        try:
            fields = (path / "stat").read_text().rsplit(")", 1)[1].split()
            processes[int(path.name)] = (int(fields[1]), fields[19])
        except (OSError, ValueError, IndexError):
            continue
    descendants = {child_pid}
    while True:
        expanded = descendants | {pid for pid, (parent, _) in processes.items()
                                  if parent in descendants}
        if expanded == descendants:
            break
        descendants = expanded
    found = []
    for pid in sorted(descendants - {child_pid}):
        try:
            executable = Path("/proc") / str(pid) / "exe"
            if executable.resolve(strict=True) == git:
                found.append({"pid": pid, "start_ticks": processes[pid][1],
                              "executable_sha256": digest(executable)})
        except OSError:
            continue
    require(found, "held production Git executable was not observed below exact child")
    return found


class Fixture:
    def __init__(self, root, git, case, deadline):
        self.root, self.git, self.case, self.deadline = root, git, case, deadline
        self.home = root / "home"
        self.home.mkdir(parents=True)
        self.gate_seen, self.git_release = threading.Event(), threading.Event()
        self.model_seen, self.model_release = threading.Event(), threading.Event()
        self.lock = threading.Lock()
        self.backends, self.errors, self.forced, self.events = [], [], [], []
        self.git_requests, self.model_requests, self.mcp_calls = [], 0, []
        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), self.handler_type())
        self.server.daemon_threads = False
        self.server.block_on_close = True
        self.url = "http://127.0.0.1:" + str(self.server.server_port)
        self.thread = threading.Thread(target=self.server.serve_forever,
                                       kwargs={"poll_interval": 0.05}, name="curated-fixture")
        self.thread.start()

    def remaining(self):
        value = self.deadline - time.monotonic()
        require(value > 0, "fixture absolute deadline expired")
        return value

    def await_event(self, event, description):
        require(event.wait(self.remaining()), "missing observed " + description)

    def run_git(self, cwd, *args):
        # Repository setup only. The host child receives its own ordinary user
        # global config; neither production Git nor its policy is replaced.
        timeout = min(20, self.remaining())
        process = subprocess.Popen([str(self.git), *args], cwd=cwd, env=fixture_git_env(),
                                   stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        with self.lock:
            self.backends.append(process)
        output, error = self.collect_git(process, b"", timeout)
        require(process.returncode == 0, "fixture Git setup failed")
        require(len(output) + len(error) < LIMIT, "fixture Git output limit")
        return output.decode().strip()

    def collect_git(self, process, body, timeout):
        try:
            return process.communicate(body, timeout=timeout)
        except subprocess.TimeoutExpired as original:
            self.forced.append("fixture Git timeout")
            (self.root / ("git-timeout-" + str(process.pid) + ".stdout")).write_bytes(original.output or b"")
            (self.root / ("git-timeout-" + str(process.pid) + ".stderr")).write_bytes(original.stderr or b"")
            if process.poll() is None:
                process.kill()
            try:
                process.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                # An inherited writer is uncertainty, never an unbounded wait
                # or a successful join claim. Strict subreaper still owns its
                # independent descendant check after this fixture fails.
                self.forced.append("fixture Git pipe drain unconfirmed")
                for stream in (process.stdin, process.stdout, process.stderr):
                    if stream is not None:
                        stream.close()
                process.wait(timeout=5)
            raise RuntimeError("fixture Git required forced cleanup") from original

    def write_plugin(self, repo, version):
        plugin = repo / "plugins/replacement-probe"
        for directory in (".codex-plugin", "skills/replacement-skill", "hooks"):
            (plugin / directory).mkdir(parents=True, exist_ok=True)
        write_json(plugin / ".codex-plugin/plugin.json", {
            "name": "replacement-probe", "version": version, "description": version})
        (plugin / "skills/replacement-skill/SKILL.md").write_text(
            "---\nname: replacement-skill\ndescription: " + version
            + "\n---\n\nObserved curated skill " + version + "\n")
        endpoint = "old" if version == "version1" else "new"
        write_json(plugin / ".mcp.json", {"mcpServers": {
            "probe-" + endpoint: {"type": "http", "url": self.url + "/mcp/" + endpoint}}})
        write_json(plugin / "hooks/hooks.json", {"hooks": {"Interrupt": [{"hooks": [{
            "type": "command", "command": "python3 ${PLUGIN_ROOT}/hooks/probe.py",
            "timeout": 5}]}]}})
        (plugin / "hooks/probe.py").write_text(
            "from pathlib import Path\nimport sys\nsys.stdin.read()\n"
            + "with Path(" + repr(str(self.home / "hook.log")) + ").open('a') as out:\n"
            + "    out.write(" + repr(version + "\n") + ")\nprint('{}')\n")
        manifests = repo / ".agents/plugins"
        manifests.mkdir(parents=True, exist_ok=True)
        write_json(manifests / "marketplace.json", {"name": "openai-curated", "plugins": []})
        write_json(manifests / "api_marketplace.json", {
            "name": "openai-api-curated", "plugins": [{"name": "replacement-probe",
                "source": {"source": "local", "path": "./plugins/replacement-probe"}}]})

    def inventory(self, directory):
        return {str(p.relative_to(directory)): digest(p) for p in sorted(directory.rglob("*"))
                if p.is_file() and ".git" not in p.relative_to(directory).parts}

    def prepare(self):
        repo = self.root / "source"
        repo.mkdir()
        self.run_git(repo, "init", "-b", "main")
        self.write_plugin(repo, "version1")
        self.run_git(repo, "add", ".")
        commit_args = ("-c", "user.name=Acceptance", "-c", "user.email=acceptance@example.invalid",
                       "-c", "commit.gpgSign=false", "commit", "-qm")
        self.run_git(repo, *commit_args, "old curated fixture")
        old_sha = self.run_git(repo, "rev-parse", "HEAD")
        write_json(self.root / "old-inventory.json", self.inventory(repo))
        shutil.copytree(repo, self.home / ".tmp/plugins", ignore=shutil.ignore_patterns(".git"))
        (self.home / ".tmp/plugins.sha").write_text(old_sha + "\n")
        shutil.copytree(repo / "plugins/replacement-probe",
                        self.home / "plugins/cache/openai-api-curated/replacement-probe/local")
        self.write_plugin(repo, "version2")
        self.run_git(repo, "add", ".")
        self.run_git(repo, *commit_args, "new curated fixture")
        new_sha = self.run_git(repo, "rev-parse", "HEAD")
        write_json(self.root / "new-inventory.json", self.inventory(repo))
        (self.root / "export").mkdir()
        self.run_git(repo, "clone", "--bare", str(repo), str(self.root / "export/plugins.git"))
        self.git_config = self.root / "global.gitconfig"
        self.git_config.write_text('[url "' + self.url + '/plugins.git"]\n'
                                   '    insteadOf = ' + GIT_URL + '\n')
        # Hook trust is a supported child ConfigOverrides harness override,
        # applied by the Rust fixture, not an invented TOML field.
        (self.home / "config.toml").write_text('''model = "mock-model"
model_provider = "replacement_fixture"
approval_policy = "never"
sandbox_mode = "danger-full-access"
[features]
plugins = true
hooks = true
[plugins."replacement-probe@openai-api-curated"]
enabled = true
[model_providers.replacement_fixture]
name = "Replacement acceptance fixture"
base_url = "''' + self.url + '''/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
supports_websockets = false
requires_openai_auth = false
''')
        self.identities = {"old_commit": old_sha, "new_commit": new_sha,
                           "git_sha256": digest(self.git),
                           "global_config_sha256": digest(self.git_config)}

    def handler_type(self):
        fixture = self

        class Handler(http.server.BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def setup(self):
                super().setup()
                self.connection.settimeout(10)

            def log_message(self, *_args):
                pass

            def reply(self, status, body=b"", content_type="application/json"):
                self.send_response(status)
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def handle_request(self):
                try:
                    target = urlsplit(self.path)
                    length = int(self.headers.get("Content-Length", "0"))
                    require(0 <= length <= LIMIT, "fixture request body limit")
                    body = self.rfile.read(length)
                    if target.path.startswith("/plugins.git/"):
                        with fixture.lock:
                            fixture.git_requests.append({"method": self.command,
                                "path": target.path, "query": target.query})
                            first = len(fixture.git_requests) == 1
                        if first:
                            require(self.command == "GET" and target.path == "/plugins.git/info/refs"
                                    and target.query == "service=git-upload-pack",
                                    "first request was not actual Git advertisement")
                            fixture.gate_seen.set()
                            fixture.await_event(fixture.git_release, "Git release")
                        env = fixture_git_env()
                        env.update({"GIT_PROJECT_ROOT": str(fixture.root / "export"),
                                    "GIT_HTTP_EXPORT_ALL": "1", "PATH_INFO": target.path,
                                    "REQUEST_METHOD": self.command, "QUERY_STRING": target.query,
                                    "CONTENT_TYPE": self.headers.get("Content-Type", ""),
                                    "CONTENT_LENGTH": str(length),
                                    "HTTP_GIT_PROTOCOL": self.headers.get("Git-Protocol", "")})
                        backend_timeout = min(20, fixture.remaining())
                        backend = subprocess.Popen([str(fixture.git), "http-backend"], env=env,
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                        with fixture.lock:
                            fixture.backends.append(backend)
                        output, error = fixture.collect_git(backend, body, backend_timeout)
                        require(backend.returncode == 0, "Git backend returned failure")
                        require(len(output) + len(error) <= LIMIT, "Git backend output limit")
                        header, payload = output.split(b"\r\n\r\n", 1)
                        headers = [line.decode().split(":", 1) for line in header.split(b"\r\n")]
                        status = next((int(v.strip().split()[0]) for k, v in headers
                                       if k.lower() == "status"), 200)
                        self.send_response(status)
                        for key, value in headers:
                            if key.lower() not in ("status", "content-length"):
                                self.send_header(key, value.strip())
                        self.send_header("Content-Length", str(len(payload)))
                        self.end_headers()
                        self.wfile.write(payload)
                    elif target.path in ("/mcp/old", "/mcp/new"):
                        if self.command == "DELETE":
                            self.reply(204)
                            return
                        if self.command != "POST":
                            self.reply(405)
                            return
                        request = json.loads(body)
                        if "id" not in request:
                            self.reply(202)
                            return
                        version = "version1" if target.path.endswith("/old") else "version2"
                        method = request["method"]
                        if method == "initialize":
                            result = {"protocolVersion": request["params"]["protocolVersion"],
                                      "capabilities": {"tools": {}},
                                      "serverInfo": {"name": "replacement_probe", "version": version}}
                        elif method == "tools/list":
                            result = {"tools": [{"name": "replacement_probe", "description": version,
                                       "inputSchema": {"type": "object", "properties": {}},
                                       "annotations": {"readOnlyHint": True}}]}
                        elif method == "tools/call":
                            require(request["params"]["name"] == "replacement_probe", "wrong MCP tool")
                            with fixture.lock:
                                fixture.mcp_calls.append(version)
                            result = {"content": [{"type": "text", "text": version}], "isError": False}
                        elif method == "ping":
                            result = {}
                        else:
                            self.reply(200, json.dumps({"jsonrpc": "2.0", "id": request["id"],
                                "error": {"code": -32601, "message": "fixture method absent"}}).encode())
                            return
                        self.reply(200, json.dumps({"jsonrpc": "2.0", "id": request["id"],
                                                    "result": result}).encode())
                    elif target.path == "/v1/responses" and self.command == "POST":
                        with fixture.lock:
                            fixture.model_requests += 1
                        self.send_response(200)
                        self.send_header("Content-Type", "text/event-stream")
                        self.send_header("Connection", "close")
                        self.end_headers()
                        self.wfile.write(b'event: response.created\ndata: {"type":"response.created",'
                                         b'"response":{"id":"replacement-held"}}\n\n')
                        self.wfile.flush()
                        fixture.model_seen.set()
                        fixture.await_event(fixture.model_release, "held model termination")
                        self.close_connection = True
                    elif target.path == "/v1/models":
                        self.reply(200, b'{"data":[]}')
                    else:
                        self.reply(404)
                except (BrokenPipeError, ConnectionResetError):
                    # Interrupted model/MCP clients can close normally. Exact
                    # child protocol outcomes remain mandatory for acceptance.
                    pass
                except Exception as error:
                    with fixture.lock:
                        fixture.errors.append(type(error).__name__ + ": " + str(error)[:300])
                    self.close_connection = True

            do_GET = handle_request
            do_POST = handle_request
            do_DELETE = handle_request

        return Handler

    def close(self):
        self.git_release.set()
        self.model_release.set()
        self.server.shutdown()
        # Wake the owners before joining request handlers. Each handler uses a
        # socket timeout and finite collect_git/post-kill pipe-drain budgets.
        for backend in self.backends:
            if backend.poll() is None:
                self.forced.append("Git backend required final failure cleanup")
                backend.kill()
        self.server.server_close()
        self.thread.join(timeout=5)
        require(not self.thread.is_alive(), "fixture server did not join")
        for backend in self.backends:
            if backend.poll() is None:
                backend.wait(timeout=5)
        require(all(p.poll() is not None for p in self.backends), "fixture Git backend still owned")


def run_case(args, case, git):
    root = args.work_dir / case
    root.mkdir()
    fixture = Fixture(root, git, case, time.monotonic() + args.timeout)
    child = None
    report = {"case": case, "passed": False, "whole_host_clean": False,
              "mcp_shutdown_custody": "not established by this acceptance"}
    try:
        fixture.prepare()
        with socket.socket() as listener, (root / "child.stdout").open("wb") as stdout, \
                (root / "child.stderr").open("wb") as stderr:
            listener.bind(("127.0.0.1", 0))
            listener.listen(1)
            listener.settimeout(fixture.remaining())
            env = os.environ.copy()
            for name in ("CODEX_API_KEY", "OPENAI_API_KEY", "CHATGPT_API_KEY"):
                env.pop(name, None)
            env.update({"CODEX_HOME": str(fixture.home), "HOME": str(fixture.home),
                        "GIT_CONFIG_GLOBAL": str(fixture.git_config),
                        "CODEX_TEST_CURATED_REPLACEMENT_HOME": str(fixture.home),
                        "CODEX_TEST_CURATED_REPLACEMENT_CASE": case,
                        "CODEX_TEST_CURATED_REPLACEMENT_CONTROL":
                            "127.0.0.1:" + str(listener.getsockname()[1])})
            child = subprocess.Popen([str(args.test_elf), "--ignored", "--exact", CASE_NAME,
                                      "--nocapture", "--test-threads=1"], cwd=fixture.home,
                                     env=env, stdout=stdout, stderr=stderr)
            connection, _ = listener.accept()
            with connection, connection.makefile("rwb", buffering=0) as control:
                connection.settimeout(fixture.remaining())
                expected = ["a_started", "a_closed"]
                expected += ["race_start"] if case == "race" else []
                expected += ["b_ready", "behavior_verified", "complete"]
                for name in expected:
                    connection.settimeout(fixture.remaining())
                    raw = control.readline(65537)
                    require(raw.endswith(b"\n") and len(raw) <= 65536, "invalid child phase frame")
                    event = json.loads(raw)
                    require(event.get("event") == name, "unexpected child phase order")
                    fixture.events.append(event)
                    if name == "a_started":
                        fixture.await_event(fixture.gate_seen, "trusted Git advertisement")
                        report["held_git_observations"] = observe_held_git(child.pid, git)
                    if name == "b_ready":
                        fixture.await_event(fixture.model_seen, "actual B model request")
                    if name == "a_closed" and case == "replay":
                        fixture.git_release.set()
                    if name == "b_ready" and case == "pending":
                        fixture.git_release.set()
                    connection.sendall(json.dumps({"ack": name}).encode() + b"\n")
                    if name == "race_start":
                        # B starts as this barrier releases Git, with no delay
                        # chosen to force either legal registration ordering.
                        fixture.git_release.set()
                    if name == "behavior_verified":
                        fixture.model_release.set()
                require(child.wait(timeout=fixture.remaining()) == 0, "child acceptance failed")
        receipt = json.loads((fixture.home / "curated-replacement-child.json").read_text())
        require(receipt.get("passed") is True, "missing successful typed child receipt")
        require(fixture.model_requests == 1, "unexpected model retries/turn count")
        require("version1" in fixture.mcp_calls and "version2" in fixture.mcp_calls,
                "old and new MCP behavior were not exercised")
        require(len(fixture.git_requests) >= 2, "real Git materialization was not observed")
        require(not fixture.errors and not fixture.forced, "fixture failure or forced cleanup")
        require(digest(args.test_elf) == args.test_elf_sha256, "test ELF changed during run")
        require(digest(git) == args.git_sha256, "trusted Git changed during run")
        report.update({"passed": True, "child": receipt, "identities": fixture.identities})
    except Exception as error:
        report["failure"] = type(error).__name__ + ": " + str(error)[:500]
    finally:
        try:
            if child is not None and child.poll() is None:
                fixture.forced.append("child required failure cleanup")
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=5)
        except Exception as error:
            fixture.errors.append("child cleanup: " + type(error).__name__)
        try:
            fixture.close()
        except Exception as error:
            fixture.errors.append("fixture cleanup: " + type(error).__name__ + ": " + str(error)[:200])
        report.update({"child_returncode": child.returncode if child else None,
                       "events": fixture.events, "git_requests": fixture.git_requests,
                       "mcp_calls": fixture.mcp_calls, "model_requests": fixture.model_requests,
                       "fixture_errors": fixture.errors, "forced_cleanup": fixture.forced,
                       "backend_returncodes": [p.returncode for p in fixture.backends]})
        if fixture.forced or fixture.errors:
            report["passed"] = False
        write_json(root / "result.json", report)
    require(report["passed"], "case failed; original operation/cleanup evidence retained in result.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--test-elf", type=Path, required=True)
    parser.add_argument("--test-elf-sha256", required=True)
    parser.add_argument("--git-sha256", required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=120)
    args = parser.parse_args()
    require(sys.platform == "linux", "this acceptance currently supports Linux only")
    args.test_elf = args.test_elf.resolve(strict=True)
    args.work_dir = args.work_dir.absolute()
    require(not args.work_dir.exists(), "work directory must be fresh")
    require(30 <= args.timeout <= 240, "fixture deadline outside supported bound")
    require(digest(args.test_elf) == args.test_elf_sha256, "test ELF identity mismatch")
    git = trusted_git()
    require(digest(git) == args.git_sha256, "trusted Git identity mismatch")
    args.work_dir.mkdir(parents=True)
    for case in CASES:
        run_case(args, case, git)
    write_json(args.work_dir / "summary.json", {"passed": True, "cases": list(CASES),
               "test_elf_sha256": args.test_elf_sha256, "git_sha256": args.git_sha256,
               "whole_host_clean": False, "strict_runner_required": True})


if __name__ == "__main__":
    main()
