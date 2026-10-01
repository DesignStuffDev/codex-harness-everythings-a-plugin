"""Bounded stdio JSON-RPC observer for a real App Server, with strict cleanup.

The unchanged repository subreaper must wrap the top-level acceptance command.
Neither that runner nor this observer treats zombies as absent. No retries.
"""

from collections import deque
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import time

from acceptance_support import (
    Runner,
    fingerprint,
    is_pid_present,
    process_snapshot,
    require,
    write_report,
)

SEARCH_METHODS = {
    "fuzzyFileSearch/sessionUpdated",
    "fuzzyFileSearch/sessionCompleted",
    "fuzzyFileSearch/sessionFailed",
}
MAX_FRAME_BYTES = 16 * 1024 * 1024
MAX_BUFFERED_MESSAGES = 2048
MAX_TRANSCRIPT_BYTES = 32 * 1024 * 1024


class RpcServer:
    def __init__(self, label, argv, cwd, environment, work, report, report_path):
        self.report, self.report_path = report, report_path
        self.entry = {
            "label": label,
            "argv": list(map(str, argv)),
            "cwd": str(cwd),
            "passed": False,
            "normal_shutdown": "stdin EOF",
            "rpc_checks": [],
        }
        report["rpc_processes"].append(self.entry)
        write_report(report_path, report)
        self.started = time.monotonic()
        self.closed = False
        self.next_id = 0
        self.sequence = 0
        self.buffer = bytearray()
        self.pending = deque()
        self.tracked = {}
        self.eof = False
        self.transcript_bytes = 0
        self.output_path = work / (label + ".stdout.ndjson")
        self.input_path = work / (label + ".stdin.ndjson")
        self.error_path = work / (label + ".stderr")
        self.output = self.output_path.open("wb")
        self.inputs = self.input_path.open("wb")
        self.errors = self.error_path.open("wb")
        self.selector = selectors.DefaultSelector()
        self.child = None
        try:
            self.child = subprocess.Popen(
                self.entry["argv"],
                cwd=cwd,
                env=environment,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=self.errors,
                start_new_session=True,
                bufsize=0,
            )
            self.entry["pid"] = self.child.pid
            os.set_blocking(self.child.stdout.fileno(), False)
            os.set_blocking(self.child.stdin.fileno(), False)
            self.selector.register(self.child.stdout, selectors.EVENT_READ)
            self._track()
        except BaseException:
            self.entry["observer_construction_failed"] = True
            if self.child is not None:
                self.abort()
            else:
                self.selector.close()
                for stream in (self.output, self.inputs, self.errors):
                    stream.close()
                write_report(self.report_path, self.report)
            raise

    def _track(self):
        snapshot = process_snapshot()
        parents = {self.child.pid, *self.tracked}
        while True:
            expanded = parents | {
                pid for pid, item in snapshot.items() if item["parent"] in parents
            }
            if expanded == parents:
                break
            parents = expanded
        for pid in parents:
            if pid in snapshot:
                previous = self.tracked.get(pid, {})
                require(
                    not previous
                    or previous["start_ticks"] == snapshot[pid]["start_ticks"],
                    "PID reuse during acceptance; lifecycle evidence is ambiguous",
                )
                self.tracked[pid] = {**previous, **snapshot[pid]}

    def pump(self, seconds=0.02):
        self._track()
        for _key, _mask in self.selector.select(seconds):
            chunk = os.read(self.child.stdout.fileno(), 65536)
            if not chunk:
                self.eof = True
                self.selector.unregister(self.child.stdout)
                require(not self.buffer, "App Server ended with a partial JSON frame")
                continue
            self.transcript_bytes += len(chunk)
            require(
                self.transcript_bytes <= MAX_TRANSCRIPT_BYTES,
                "RPC transcript budget exceeded",
            )
            self.output.write(chunk)
            self.output.flush()
            self.buffer.extend(chunk)
            while b"\n" in self.buffer:
                line, _, remaining = self.buffer.partition(b"\n")
                self.buffer = bytearray(remaining)
                require(
                    len(line) <= MAX_FRAME_BYTES,
                    "App Server frame exceeded runner ceiling",
                )
                message = json.loads(line)
                require(isinstance(message, dict), "App Server frame is not an object")
                require(
                    not ("method" in message and "id" in message),
                    "unexpected server request",
                )
                self.sequence += 1
                self.pending.append((self.sequence, message))
                require(
                    len(self.pending) <= MAX_BUFFERED_MESSAGES,
                    "RPC observer backlog exceeded",
                )
            require(
                len(self.buffer) <= MAX_FRAME_BYTES,
                "unterminated App Server frame exceeded ceiling",
            )
        self._track()

    def send(self, method, params=None, *, notification=False):
        message = {"method": method}
        if params is not None:
            message["params"] = params
        request_id = None
        if not notification:
            self.next_id += 1
            request_id = self.next_id
            message["id"] = request_id
        payload = (
            json.dumps(message, ensure_ascii=False, separators=(",", ":")).encode()
            + b"\n"
        )
        require(len(payload) <= MAX_FRAME_BYTES, "test input frame exceeds ceiling")
        self.inputs.write(payload)
        self.inputs.flush()
        view = memoryview(payload)
        deadline = time.monotonic() + 30
        while view:
            require(time.monotonic() < deadline, "App Server input write timed out")
            require(
                self.child.poll() is None, "App Server exited before accepting input"
            )
            try:
                written = os.write(self.child.stdin.fileno(), view)
                view = view[written:]
            except BlockingIOError:
                self.pump(0.01)
        return request_id

    def receive(self, predicate, *, timeout=60):
        deadline = time.monotonic() + timeout
        while True:
            for entry in self.pending:
                if predicate(entry[1]):
                    self.pending.remove(entry)
                    return entry
            require(
                time.monotonic() < deadline,
                "RPC observation timed out; see saved transcript",
            )
            require(
                not self.eof, "App Server stdout closed before expected RPC observation"
            )
            self.pump(min(0.02, max(0, deadline - time.monotonic())))

    def response(self, request_id, *, error_code=None):
        sequence, response = self.receive(lambda msg: msg.get("id") == request_id)
        if error_code is None:
            require(
                "result" in response and "error" not in response,
                "unexpected RPC error; see transcript",
            )
        else:
            require(
                response.get("error", {}).get("code") == error_code,
                "wrong expected RPC error",
            )
        return sequence, response

    def request(self, method, params=None, *, error_code=None):
        return self.response(self.send(method, params), error_code=error_code)

    def initialize(self):
        _, response = self.request(
            "initialize",
            {
                "clientInfo": {"name": "p02b-real-native-search", "version": "0.1.0"},
                "capabilities": {"experimentalApi": True},
            },
        )
        require(
            isinstance(response["result"], dict), "initialize result is not an object"
        )
        self.send("initialized", notification=True)
        self.entry["rpc_checks"].append("initialize experimentalApi + initialized")

    def cycle(self, request_id, session_id, query):
        """Completion has no query field; serialize explicitly admitted cycles.

        Observe response + current-query snapshot + completion in either response
        order. The snapshot must precede completion in the actual server stream.
        """
        response_seen = complete = updated = False
        files = None
        deadline = time.monotonic() + 90
        while not response_seen or not complete:
            _, message = self.receive(
                lambda msg: (
                    msg.get("id") == request_id
                    or (
                        msg.get("method") in SEARCH_METHODS
                        and msg.get("params", {}).get("sessionId") == session_id
                    )
                ),
                timeout=max(0, deadline - time.monotonic()),
            )
            if message.get("id") == request_id:
                require(
                    "result" in message and "error" not in message,
                    "search request failed",
                )
                response_seen = True
            elif message["method"] == "fuzzyFileSearch/sessionUpdated":
                require(not complete, "snapshot followed completion for the same query")
                require(
                    message["params"]["query"] == query,
                    "stale or unexpected query snapshot",
                )
                files = message["params"]["files"]
                updated = True
            elif message["method"] == "fuzzyFileSearch/sessionCompleted":
                require(
                    updated and not complete,
                    "completion lacks preceding current snapshot or repeats",
                )
                complete = True
            else:
                raise AssertionError(
                    "unexpected search session failure; see transcript"
                )
        require(updated, "completed query did not publish a snapshot")
        return files

    def assert_quiet(self, session_id, after_sequence, seconds=0.25):
        deadline = time.monotonic() + seconds
        while True:
            require(
                not any(
                    seq > after_sequence
                    and msg.get("method") in SEARCH_METHODS
                    and msg.get("params", {}).get("sessionId") == session_id
                    for seq, msg in self.pending
                ),
                "search notification arrived after joined stop acknowledgement",
            )
            if time.monotonic() >= deadline:
                return
            self.pump(0.01)

    def require_worker(self, worker):
        self._track()
        matches = [
            entry
            for entry in self.tracked.values()
            if entry.get("executable") == str(worker)
        ]
        require(matches, "actual installed worker executable was not observed")
        self.entry["observed_installed_worker_pids"] = [
            entry["pid"] for entry in matches
        ]

    def finish(self, *, expected_status=0):
        require(not self.closed, "App Server finish called twice")
        self.child.stdin.close()
        self.entry["stdin_closed_for_normal_shutdown"] = True
        deadline = time.monotonic() + 150  # Source cleanup budget is 120 seconds.
        while self.child.poll() is None or not self.eof:
            require(time.monotonic() < deadline, "normal App Server shutdown timed out")
            self.pump(0.02)
        self.child.wait()
        self._record(False)
        if expected_status == "nonzero":
            require(
                self.child.returncode > 0,
                "expected explicit operation-failure exit, not success/signal",
            )
        else:
            require(
                self.child.returncode == expected_status,
                "unexpected App Server shutdown status",
            )
        require(
            self.entry["all_observed_pids_absent"],
            "App Server or tracked descendants remain",
        )
        self.entry["passed"] = True
        write_report(self.report_path, self.report)

    def abort(self):
        if self.closed:
            if self.entry.get("remaining_pids"):
                # Absence failed before logs closed. Cleanup may still be useful,
                # but cannot replace the failed lifecycle evidence above.
                self.entry["emergency_forced_cleanup"] = True
                Runner._signal(self.child, self.tracked, signal.SIGKILL)
                write_report(self.report_path, self.report)
            return
        self._track()
        self.entry["emergency_forced_cleanup"] = True
        Runner._signal(self.child, self.tracked, signal.SIGKILL)
        try:
            self.child.wait(timeout=10)
        finally:
            self._record(True)

    def _record(self, forced):
        self._track()
        deadline = time.monotonic() + 10
        while (
            any(is_pid_present(pid) for pid in self.tracked)
            and time.monotonic() < deadline
        ):
            time.sleep(0.01)
        remaining = [pid for pid in self.tracked if is_pid_present(pid)]
        self.entry.update(
            status=self.child.returncode,
            emergency_forced_cleanup=forced,
            elapsed_seconds=time.monotonic() - self.started,
            observed_processes=list(self.tracked.values()),
            remaining_pids=remaining,
            all_observed_pids_absent=not remaining,
        )
        self.closed = True
        self.selector.close()
        for stream in (
            self.output,
            self.inputs,
            self.errors,
            self.child.stdin,
            self.child.stdout,
        ):
            stream.close()
        self.entry["logs"] = {
            name: {"path": str(path), **fingerprint(path)}
            for name, path in (
                ("stdin", self.input_path),
                ("stdout", self.output_path),
                ("stderr", self.error_path),
            )
        }
        write_report(self.report_path, self.report)
