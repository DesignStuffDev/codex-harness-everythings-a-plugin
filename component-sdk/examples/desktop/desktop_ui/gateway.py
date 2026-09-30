"""Loopback-only HTTP/SSE adapter for the pinned Codex app-server protocol.

The gateway owns transient RPC correlation and bounded event replay. App-server
owns approvals, running turns, sandboxing, authentication and durable history.
"""

from collections import deque
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib.resources import files
import json
import os
from pathlib import Path
import secrets
import subprocess
import threading
from urllib.parse import parse_qs, urlsplit

MAX_FRAME = 4 * 1024 * 1024
# App-server may drain background work and its selected storage component.
GRACEFUL_CLOSE_SECONDS = 200
SIGNAL_CLOSE_SECONDS = 2
METHODS = frozenset(
    {
        "thread/list",
        "thread/start",
        "thread/resume",
        "thread/read",
        "thread/turns/list",
        "thread/items/list",
        "turn/start",
        "turn/interrupt",
    }
)


class Bridge:
    def __init__(self, executable, codex_home=None, cwd=None):
        self.condition = threading.Condition()
        self.write_lock = threading.Lock()
        self.requests = {}
        self.approvals = {}
        self.events = deque()
        self.event_bytes = 0
        self.sequence = 0
        self.next_id = 0
        self.closed = False
        self.close_lock = threading.Lock()
        self.close_finished = False
        self.close_error = None
        environment = dict(os.environ)
        if codex_home is not None:
            environment["CODEX_HOME"] = codex_home
        # Inherit the host's process group so host cancellation also reaches Codex.
        self.child = subprocess.Popen(
            [executable, "app-server"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            cwd=cwd,
            env=environment,
        )
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()
        try:
            response = self.rpc(
                "initialize",
                {
                    "clientInfo": {
                        "name": "codex_component_desktop",
                        "title": "Codex Components",
                        "version": "0.1.0",
                    }
                },
            )
            if "error" in response:
                raise RuntimeError("app-server initialization rejected")
            self.send({"method": "initialized"})
        except BaseException:
            self.close()
            raise

    def send(self, frame):
        data = json.dumps(frame, allow_nan=False).encode() + b"\n"
        if len(data) > MAX_FRAME:
            raise ValueError("request too large")
        with self.write_lock:
            if self.closed:
                raise RuntimeError("Codex disconnected")
            self.child.stdin.write(data)
            self.child.stdin.flush()

    def rpc(self, method, params, timeout=90):
        with self.condition:
            self.next_id += 1
            request_id = self.next_id
            self.requests[request_id] = None
        try:
            self.send({"id": request_id, "method": method, "params": params})
            with self.condition:
                if not self.condition.wait_for(
                    lambda: self.requests[request_id] is not None or self.closed,
                    timeout,
                ):
                    raise RuntimeError(
                        "Codex did not respond; check the session before retrying"
                    )
                response = self.requests[request_id]
                if response is None:
                    raise RuntimeError("Codex disconnected")
                return {
                    key: value
                    for key, value in response.items()
                    if key in ("result", "error")
                }
        finally:
            with self.condition:
                self.requests.pop(request_id, None)

    def reply(self, frame):
        request_id = frame.get("id")
        if not isinstance(request_id, (str, int)) or isinstance(request_id, bool):
            raise ValueError("invalid request id")
        with self.condition:
            if request_id not in self.approvals:
                raise ValueError("request already resolved")
            if ("result" in frame) == ("error" in frame):
                raise ValueError("expected result or error")
            self.send(
                {
                    key: value
                    for key, value in frame.items()
                    if key in ("id", "result", "error")
                }
            )
            del self.approvals[request_id]
            self._publish(
                {
                    "method": "serverRequest/resolved",
                    "params": {"requestId": request_id},
                }
            )

    def _publish(self, message):
        data = json.dumps(message, separators=(",", ":"))
        self.sequence += 1
        self.events.append((self.sequence, data))
        self.event_bytes += len(data)
        while len(self.events) > 2048 or self.event_bytes > 8 * 1024 * 1024:
            self.event_bytes -= len(self.events.popleft()[1])
        self.condition.notify_all()

    def _read(self):
        try:
            while line := self.child.stdout.readline(MAX_FRAME + 1):
                if len(line) > MAX_FRAME or not line.endswith(b"\n"):
                    break
                message = json.loads(line)
                if not isinstance(message, dict):
                    break
                with self.condition:
                    if "method" in message:
                        if "id" in message:
                            if len(self.approvals) >= 64:
                                self.send(
                                    {
                                        "id": message["id"],
                                        "error": {
                                            "code": -32000,
                                            "message": "Too many pending interactions",
                                        },
                                    }
                                )
                                continue
                            self.approvals[message["id"]] = message
                        if message["method"] == "serverRequest/resolved":
                            self.approvals.pop(
                                message.get("params", {}).get("requestId"), None
                            )
                        self._publish(message)
                    elif message.get("id") in self.requests:
                        self.requests[message["id"]] = message
                        self.condition.notify_all()
        except (OSError, ValueError, TypeError):
            pass
        finally:
            with self.condition:
                self.closed = True
                self.condition.notify_all()

    def close(self):
        # A shutdown hook can run after launch has already observed an exit.
        # Preserve a failed close rather than reporting success on the second call.
        with self.close_lock:
            if not self.close_finished:
                try:
                    self._close()
                except (OSError, RuntimeError) as error:
                    self.close_error = error
                except BaseException:
                    self.close_error = RuntimeError(
                        "Codex cleanup was interrupted; write outcome unknown"
                    )
                    raise
                finally:
                    self.close_finished = True
            if self.close_error:
                raise self.close_error

    def _close(self):
        with self.condition:
            self.closed = True
            self.condition.notify_all()
        pipe_failed = False
        forced = False
        try:
            if self.child.stdin:
                try:
                    self.child.stdin.close()
                except OSError:
                    pipe_failed = True
            try:
                status = self.child.wait(timeout=GRACEFUL_CLOSE_SECONDS)
            except subprocess.TimeoutExpired:
                forced = True
                try:
                    self.child.terminate()
                except ProcessLookupError:
                    pass
                try:
                    status = self.child.wait(timeout=SIGNAL_CLOSE_SECONDS)
                except subprocess.TimeoutExpired:
                    try:
                        self.child.kill()
                    except ProcessLookupError:
                        pass
                    try:
                        status = self.child.wait(timeout=SIGNAL_CLOSE_SECONDS)
                    except subprocess.TimeoutExpired:
                        raise RuntimeError(
                            "Codex did not exit after forced shutdown; write outcome unknown"
                        ) from None
            if forced:
                raise RuntimeError(
                    "Codex required forced shutdown; write outcome unknown"
                )
            if status != 0 or pipe_failed:
                raise RuntimeError(
                    "Codex exited without confirmed cleanup; write outcome unknown"
                )
        finally:
            self.reader.join(timeout=2)
            # An escaped descendant may still own the pipe. Never block here
            # waiting on the reader's buffered-file lock after the bounded wait.
            if not self.reader.is_alive():
                self.child.stdout.close()
            else:
                raise RuntimeError("Codex output did not close; write outcome unknown")


class Gateway:
    def __init__(self, params):
        if not isinstance(params, dict):
            raise ValueError("launch parameters required")
        executable = params.get("codex_bin", "")
        port = params.get("port", 0)
        for directory in (params.get("codex_home"), params.get("cwd")):
            if directory is not None and (
                not isinstance(directory, str)
                or not Path(directory).is_absolute()
                or not Path(directory).is_dir()
            ):
                raise ValueError(
                    "home and working directory must be existing absolute directories"
                )
        if (
            params.get("host", "127.0.0.1") != "127.0.0.1"
            or type(port) is not int
            or not 0 <= port <= 65535
            or not isinstance(executable, str)
            or not Path(executable).is_absolute()
            or not Path(executable).is_file()
            or not os.access(executable, os.X_OK)
        ):
            raise ValueError(
                "requires loopback binding and an absolute executable path"
            )
        self.token = secrets.token_urlsafe(32)
        self.bridge = Bridge(executable, params.get("codex_home"), params.get("cwd"))
        try:
            self.server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
            self.server.daemon_threads = True
            self.server.gateway = self
            self.origin = f"http://127.0.0.1:{self.server.server_port}"
            self.url = f"{self.origin}/#token={self.token}"
            self.thread = threading.Thread(
                target=self.server.serve_forever, daemon=True
            )
            self.thread.start()
        except BaseException:
            self.bridge.close()
            raise

    def close(self):
        try:
            self.bridge.close()
        finally:
            self.server.shutdown()
            self.server.server_close()
            self.thread.join(timeout=2)


class Handler(BaseHTTPRequestHandler):
    def handle(self):
        try:
            super().handle()
        except (BrokenPipeError, ConnectionResetError):
            # A reload can discard an accepted HTTP request before its reply.
            # Do not retry the action or write another response to that socket.
            self.close_connection = True

    def log_message(self, format, *args):
        pass  # Access logs must not expose the SSE bearer token.

    def _headers(self, status, content_type):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Referrer-Policy", "no-referrer")
        self.send_header(
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        )
        self.end_headers()

    def _json(self, status, data):
        self._headers(status, "application/json")
        self.wfile.write(json.dumps(data).encode())

    def _allowed(self, authenticated=True):
        gateway = self.server.gateway
        origin = self.headers.get("Origin")
        if self.headers.get("Host") != gateway.origin.removeprefix("http://"):
            return False
        if origin is not None and origin != gateway.origin:
            return False
        if self.command == "POST" and origin != gateway.origin:
            return False
        if not authenticated:
            return True
        token = self.headers.get("Authorization", "").removeprefix("Bearer ")
        if urlsplit(self.path).path == "/events":
            token = parse_qs(urlsplit(self.path).query).get("token", [""])[0]
        return secrets.compare_digest(token, gateway.token)

    def do_GET(self):
        path = urlsplit(self.path).path
        static = {
            "/": ("index.html", "text/html; charset=utf-8"),
            "/app.js": ("app.js", "text/javascript; charset=utf-8"),
            "/style.css": ("style.css", "text/css; charset=utf-8"),
        }
        if not self._allowed(authenticated=path not in static):
            self._json(403, {"error": "Access denied"})
            return
        if path in static:
            name, content_type = static[path]
            self._headers(200, content_type)
            self.wfile.write(files("desktop_ui").joinpath("static", name).read_bytes())
        elif path == "/status":
            bridge = self.server.gateway.bridge
            with bridge.condition:
                self._json(
                    200,
                    {
                        "connected": not bridge.closed,
                        "cursor": bridge.sequence,
                        "requests": list(bridge.approvals.values()),
                    },
                )
        elif path == "/events":
            self._events()
        else:
            self._json(404, {"error": "Not found"})

    def _events(self):
        bridge = self.server.gateway.bridge
        try:
            query = parse_qs(urlsplit(self.path).query)
            cursor = int(
                self.headers.get("Last-Event-ID", query.get("after", ["0"])[0])
            )
        except ValueError:
            self._json(400, {"error": "Invalid event cursor"})
            return
        self._headers(200, "text/event-stream")
        try:
            while True:
                with bridge.condition:
                    bridge.condition.wait_for(
                        lambda: bridge.sequence > cursor or bridge.closed, 15
                    )
                    if bridge.closed:
                        self.wfile.write(b"event: disconnected\ndata: {}\n\n")
                        self.wfile.flush()
                        return
                    if bridge.events and cursor < bridge.events[0][0] - 1:
                        self.wfile.write(b"event: reset\ndata: {}\n\n")
                        self.wfile.flush()
                        return
                    events = [
                        (sequence, data)
                        for sequence, data in bridge.events
                        if sequence > cursor
                    ]
                for sequence, data in events:
                    self.wfile.write(f"id: {sequence}\ndata: {data}\n\n".encode())
                    cursor = sequence
                self.wfile.write(b": heartbeat\n\n")
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass

    def do_POST(self):
        if not self._allowed():
            self._json(403, {"error": {"message": "Access denied"}})
            return
        status = 200
        try:
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length <= 1024 * 1024:
                raise ValueError("invalid body size")
            frame = json.loads(self.rfile.read(length))
            if not isinstance(frame, dict):
                raise ValueError("invalid request")
            if self.path == "/rpc":
                if frame.get("method") not in METHODS or not isinstance(
                    frame.get("params", {}), dict
                ):
                    raise ValueError("unsupported method")
                result = self.server.gateway.bridge.rpc(
                    frame["method"], frame.get("params", {})
                )
            elif self.path == "/reply":
                self.server.gateway.bridge.reply(frame)
                result = {"result": {}}
            else:
                status, result = 404, {"error": {"message": "Not found"}}
        except (ValueError, TypeError):
            status, result = 400, {"error": {"message": "Invalid request"}}
        except (RuntimeError, OSError):
            status, result = (
                502,
                {
                    "error": {
                        "message": "Codex disconnected or timed out; check the session before retrying"
                    }
                },
            )
        # Keep response I/O outside the engine-error translation above: a
        # disconnected browser is not an app-server failure requiring HTTP 502.
        self._json(status, result)
