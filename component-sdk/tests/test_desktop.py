"""Presentation transport acceptance tests with an explicit app-server fixture.

This verifies client behavior, not the real engine. Run the installed component
against a compiled Codex binary separately for real model/execution coverage.
"""

import http.client
import json
import os
from pathlib import Path
import select
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import Mock, patch
from urllib.parse import urlsplit

SDK = Path(__file__).resolve().parents[1]
DESKTOP = SDK / "examples" / "desktop"
sys.path.insert(0, str(DESKTOP))
from desktop_ui.gateway import Bridge, Gateway, Handler

FIXTURE = """#!/usr/bin/env python3
import json, os, pathlib, sys
state = pathlib.Path(__file__).with_suffix('.json')
threads = json.loads(state.read_text()) if state.exists() else {}
thread = next(reversed(threads.values()), None) if threads else None
pending = {}
def send(value):
    print(json.dumps(value), flush=True)
def notify(method, params):
    send({'method': method, 'params': {'threadId':thread['id'], **params}})
def save():
    state.write_text(json.dumps(threads))
for line in sys.stdin:
    message = json.loads(line)
    method, params = message.get('method'), message.get('params', {})
    if params.get('threadId') in threads: thread = threads[params['threadId']]
    if method == 'initialized': continue
    if method is None:
        thread_id, turn_id = pending.pop(message['id'])
        thread = threads[thread_id]
        turn = next(turn for turn in thread['turns'] if turn['id'] == turn_id)
        number = turn['id'].removeprefix('turn-')
        decision = message.get('result', {}).get('decision', 'decline')
        notify('item/agentMessage/delta', {'turnId':turn_id,'itemId':'answer-' + number,'delta': 'Approved.' if decision == 'accept' else 'Declined.'})
        turn['status'] = 'completed'
        tool = next(item for item in turn['items'] if item['type'] == 'commandExecution')
        tool['status'] = 'completed' if decision == 'accept' else 'declined'
        answer = turn['items'][-1]
        answer['text'] += 'Approved.' if decision == 'accept' else 'Declined.'
        save()
        notify('item/completed', {'turnId':turn_id,'item':tool})
        notify('item/completed', {'turnId':turn_id,'item':answer})
        notify('turn/completed', {'turn':turn})
        continue
    if method == 'initialize': result = {'userAgent':'desktop-test-fixture','cwd':os.getcwd(),'codexHome':os.environ.get('CODEX_HOME')}
    elif method == 'thread/list': result = {'data':list(reversed(threads.values())),'nextCursor':None}
    elif method == 'thread/start':
        thread = {'id':'thread-' + str(len(threads) + 1),'name':None,'preview':'A saved test task','cwd':'/workspace','updatedAt':1790760000,'turns':[],'historyMode':'paginated'}
        threads[thread['id']] = thread
        save(); result = {'thread':thread}
    elif method in ('thread/resume', 'thread/read'): result = {'thread':thread}
    elif method == 'thread/turns/list': result = {'data':list(reversed(thread['turns'])),'nextCursor':None}
    elif method == 'turn/start':
        number = str(sum(len(value['turns']) for value in threads.values()) + 1)
        item = {'id':'user-' + number,'type':'userMessage','content':params['input']}
        tool = {'id':'tool-' + number,'type':'commandExecution','command':'printf desktop','status':'inProgress','aggregatedOutput':'desktop output'}
        answer = {'id':'answer-' + number,'type':'agentMessage','text':'Hello from Codex. '}
        turn = {'id':'turn-' + number,'status':'inProgress','items':[item,tool,answer],'itemsView':'full','error':None}
        thread['turns'].append(turn); save()
        send({'id':message['id'],'result':{'turn':turn}})
        notify('turn/started', {'turn':turn})
        notify('item/started', {'turnId':turn['id'],'item':item})
        notify('item/agentMessage/delta', {'turnId':turn['id'],'itemId':answer['id'],'delta':answer['text']})
        notify('item/started', {'turnId':turn['id'],'item':{**tool,'aggregatedOutput':''}})
        notify('item/commandExecution/outputDelta', {'turnId':turn['id'],'itemId':tool['id'],'delta':tool['aggregatedOutput']})
        pending['approval-' + number] = (thread['id'], turn['id'])
        send({'id':'approval-' + number,'method':'item/commandExecution/requestApproval','params':{'threadId':thread['id'],'turnId':turn['id'],'itemId':tool['id'],'command':'printf desktop','reason':'Allow this test command?'}})
        continue
    elif method == 'turn/interrupt':
        thread['turns'][-1]['status'] = 'interrupted'; save()
        for request_id, value in list(pending.items()):
            if value == (thread['id'], thread['turns'][-1]['id']):
                pending.pop(request_id)
                notify('serverRequest/resolved', {'requestId':request_id})
        notify('turn/completed', {'turn':thread['turns'][-1]}); result = {}
    else:
        send({'id':message['id'],'error':{'code':-32601,'message':'Unsupported fixture method'}}); continue
    send({'id':message['id'],'result':result})
"""


def fixture(directory):
    path = Path(directory) / "codex-fixture"
    path.write_text(FIXTURE)
    path.chmod(0o755)
    return path


class DesktopTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.executable = fixture(self.temporary.name)
        self.gateway = Gateway({"codex_bin": str(self.executable)})

    def tearDown(self):
        self.gateway.close()
        self.temporary.cleanup()

    def request(self, path, body=None, token=True, origin=None, host=None):
        connection = http.client.HTTPConnection(
            "127.0.0.1", self.gateway.server.server_port, timeout=4
        )
        headers = {
            "Origin": origin or self.gateway.origin,
            "Authorization": "Bearer " + (self.gateway.token if token else "wrong"),
        }
        if host:
            headers["Host"] = host
        connection.request(
            "POST" if body is not None else "GET",
            path,
            json.dumps(body) if body is not None else None,
            headers,
        )
        response = connection.getresponse()
        data = response.read()
        status = response.status
        connection.close()
        return status, json.loads(data) if "application/json" in response.getheader(
            "Content-Type", ""
        ) else data

    def rpc(self, method, params={}):
        status, body = self.request("/rpc", {"method": method, "params": params})
        self.assertEqual(status, 200)
        self.assertNotIn("error", body)
        return body["result"]

    def test_client_disconnect_after_accepted_rpc_does_not_retry_response_or_action(
        self,
    ):
        accepted, release, finished = (threading.Event() for _ in range(3))
        bridge_rpc = self.gateway.bridge.rpc
        send_response = Handler.send_response
        finish = Handler.finish
        statuses = []

        def delayed_reply(method, params):
            result = bridge_rpc(method, params)
            accepted.set()
            if not release.wait(5):
                raise RuntimeError("test did not release accepted request")
            return result

        def record_response(handler, status, message=None):
            statuses.append(status)
            return send_response(handler, status, message)

        def record_finish(handler):
            try:
                return finish(handler)
            finally:
                finished.set()

        with (
            patch.object(self.gateway.bridge, "rpc", side_effect=delayed_reply) as rpc,
            patch.object(self.gateway.server, "handle_error") as handle_error,
            patch.object(Handler, "send_response", record_response),
            patch.object(Handler, "finish", record_finish),
        ):
            connection = http.client.HTTPConnection(
                "127.0.0.1", self.gateway.server.server_port, timeout=4
            )
            try:
                connection.request(
                    "POST",
                    "/rpc",
                    json.dumps({"method": "thread/start", "params": {}}),
                    {
                        "Origin": self.gateway.origin,
                        "Authorization": "Bearer " + self.gateway.token,
                    },
                )
                self.assertTrue(accepted.wait(4))
                # Reset the real TCP connection while the successfully completed
                # engine operation's response is held behind the test barrier.
                connection.sock.setsockopt(
                    socket.SOL_SOCKET, socket.SO_LINGER, struct.pack("ii", 1, 0)
                )
                connection.close()
            finally:
                release.set()
                connection.close()
            self.assertTrue(finished.wait(4))
            rpc.assert_called_once_with("thread/start", {})
            self.assertEqual(statuses, [200])
            handle_error.assert_not_called()
        self.assertEqual(len(self.rpc("thread/list")["data"]), 1)

    def test_engine_pipe_failure_still_returns_error_to_connected_browser(self):
        with patch.object(self.gateway.bridge, "rpc", side_effect=BrokenPipeError):
            status, body = self.request("/rpc", {"method": "thread/list", "params": {}})
        self.assertEqual(status, 502)
        self.assertIn("Codex disconnected", body["error"]["message"])

    def test_fixture_stdio_streaming_approval_and_saved_session_recovery(self):
        self.assertEqual(self.rpc("thread/list"), {"data": [], "nextCursor": None})
        self.rpc("thread/start")
        connection = http.client.HTTPConnection(
            "127.0.0.1", self.gateway.server.server_port, timeout=4
        )
        connection.request("GET", "/events?token=" + self.gateway.token)
        stream = connection.getresponse()
        self.rpc(
            "turn/start",
            {
                "threadId": "thread-1",
                "input": [{"type": "text", "text": "hello", "text_elements": []}],
            },
        )
        messages = []
        while len(messages) < 6:
            line = stream.readline().decode()
            if line.startswith("data: "):
                messages.append(json.loads(line[6:]))
        self.assertEqual(
            [message["method"] for message in messages],
            [
                "turn/started",
                "item/started",
                "item/agentMessage/delta",
                "item/started",
                "item/commandExecution/outputDelta",
                "item/commandExecution/requestApproval",
            ],
        )
        self.assertEqual(self.request("/status")[1]["requests"], [messages[-1]])
        self.assertEqual(
            self.request(
                "/reply", {"id": "approval-1", "result": {"decision": "accept"}}
            ),
            (200, {"result": {}}),
        )
        while True:
            line = stream.readline().decode()
            if (
                line.startswith("data: ")
                and json.loads(line[6:]).get("method") == "turn/completed"
            ):
                break
        connection.close()
        self.assertEqual(self.request("/status")[1]["requests"], [])
        self.gateway.close()
        self.gateway = Gateway({"codex_bin": str(self.executable)})
        resumed = self.rpc("thread/resume", {"threadId": "thread-1"})["thread"]
        self.assertEqual(
            resumed["turns"][0]["items"][-1]["text"], "Hello from Codex. Approved."
        )

    def test_interrupt_resolves_approval_and_stops_turn(self):
        self.rpc("thread/start")
        self.rpc("turn/start", {"input": []})
        self.rpc("turn/interrupt", {"threadId": "thread-1", "turnId": "turn-1"})
        self.assertEqual(
            self.rpc("thread/read", {"threadId": "thread-1"})["thread"]["turns"][0][
                "status"
            ],
            "interrupted",
        )
        self.assertEqual(self.request("/status")[1]["requests"], [])
        self.assertEqual(
            self.request(
                "/reply", {"id": "approval-1", "result": {"decision": "accept"}}
            )[0],
            400,
        )

    def test_second_turn_ids_are_distinct_and_interrupt_preserves_first(self):
        self.rpc("thread/start")
        first = self.rpc("turn/start", {"input": []})["turn"]
        self.assertEqual(
            self.request(
                "/reply", {"id": "approval-1", "result": {"decision": "accept"}}
            )[0],
            200,
        )
        first_saved = self.rpc("thread/read", {"threadId": "thread-1"})["thread"][
            "turns"
        ][0]
        self.assertEqual(first_saved["status"], "completed")
        second = self.rpc("turn/start", {"input": []})["turn"]
        self.assertNotEqual(first["id"], second["id"])
        self.assertTrue(
            {item["id"] for item in first["items"]}.isdisjoint(
                {item["id"] for item in second["items"]}
            )
        )
        self.rpc("turn/interrupt", {"threadId": "thread-1", "turnId": second["id"]})
        saved = self.rpc("thread/read", {"threadId": "thread-1"})["thread"]["turns"]
        self.assertEqual(saved[0], first_saved)
        self.assertEqual(saved[1]["status"], "interrupted")
        self.assertEqual(self.request("/status")[1]["requests"], [])

    def test_auth_origin_host_limits_and_zip_resources(self):
        self.assertEqual(self.request("/status", token=False)[0], 403)
        self.assertEqual(
            self.request(
                "/rpc", {"method": "thread/start"}, origin="https://untrusted.example"
            )[0],
            403,
        )
        self.assertEqual(self.request("/status", host="untrusted.example")[0], 403)
        self.assertEqual(
            self.request("/rpc", {"method": "command/exec", "params": {}})[0], 400
        )
        self.assertEqual(self.request("/reply", {"id": []})[0], 400)
        for path in ("/", "/app.js", "/style.css"):
            status, data = self.request(path)
            self.assertEqual(status, 200)
            self.assertGreater(len(data), 100)
        with self.assertRaises(ValueError):
            Gateway({"codex_bin": str(self.executable), "host": "0.0.0.0"})

    def test_explicit_codex_home_and_workspace_reach_child(self):
        self.gateway.close()
        self.gateway = Gateway(
            {
                "codex_bin": str(self.executable),
                "codex_home": self.temporary.name,
                "cwd": self.temporary.name,
            }
        )
        response = self.gateway.bridge.rpc("initialize", {})
        self.assertEqual(
            response,
            {
                "result": {
                    "userAgent": "desktop-test-fixture",
                    "cwd": self.temporary.name,
                    "codexHome": self.temporary.name,
                }
            },
        )
        with self.assertRaises(ValueError):
            Gateway({"codex_bin": str(self.executable), "codex_home": "relative"})

    def test_packaged_plugin_launch_and_signal_cleanup(self):
        self.packaged_cleanup("signal")

    def test_packaged_shutdown_waits_for_slow_app_server_cleanup(self):
        # Real stdio child: the former three-second budget killed this cleanup
        # before it could persist its completion marker.
        self.executable.write_text(
            FIXTURE
            + "\nimport time\ntime.sleep(3.25)\n"
            + "pathlib.Path(__file__).with_suffix('.closed').write_text('drained')\n"
        )
        self.packaged_cleanup("protocol")
        self.assertEqual(self.executable.with_suffix(".closed").read_text(), "drained")

    def test_packaged_dead_app_server_is_not_success(self):
        self.packaged_cleanup("crash")

    def packaged_cleanup(self, stop):
        package = Path(self.temporary.name) / "desktop-package"
        project = Path(self.temporary.name) / "external-desktop-project"
        shutil.copytree(DESKTOP, project, ignore=shutil.ignore_patterns("__pycache__"))
        subprocess.run(
            [
                sys.executable,
                "-m",
                "codex_component_sdk",
                "build",
                str(project),
                "--output",
                str(package),
            ],
            env={**os.environ, "PYTHONPATH": str(SDK)},
            check=True,
            capture_output=True,
        )
        process = subprocess.Popen(
            [sys.executable, str(package / "plugin.pyz")],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        try:
            process.stdin.write(
                (
                    json.dumps(
                        {
                            "type": "initialize",
                            "api_version": 1,
                            "plugin_id": "codex.desktop",
                            "config": {},
                            "state_dir": self.temporary.name,
                        }
                    )
                    + "\n"
                ).encode()
            )
            process.stdin.flush()
            self.assertEqual(
                json.loads(process.stdout.readline()),
                {"type": "ready", "api_version": 1},
            )
            process.stdin.write(
                (
                    json.dumps(
                        {
                            "type": "request",
                            "id": 1,
                            "component": {"kind": "presentation", "name": "desktop"},
                            "method": "launch",
                            "params": {"codex_bin": str(self.executable)},
                        }
                    )
                    + "\n"
                ).encode()
            )
            process.stdin.flush()
            self.assertTrue(select.select([process.stdout], [], [], 8)[0])
            event = json.loads(process.stdout.readline())["event"]
            self.assertEqual(event["method"], "presentation/ready")
            url = urlsplit(event["params"]["url"])
            connection = http.client.HTTPConnection(url.hostname, url.port, timeout=4)
            connection.request("GET", "/app.js")
            response = connection.getresponse()
            self.assertEqual(response.status, 200)
            self.assertIn(b"EventSource", response.read())
            connection.close()
            if stop == "signal":
                process.terminate()
            else:
                if stop == "crash":
                    os.kill(event["params"]["app_server_pid"], signal.SIGKILL)
                else:
                    process.stdin.write(b'{"type":"shutdown"}\n')
                    process.stdin.flush()
                self.assertTrue(select.select([process.stdout], [], [], 8)[0])
                terminal = json.loads(process.stdout.readline())
                if stop == "crash":
                    self.assertEqual(terminal["type"], "error")
                    self.assertIn("write outcome unknown", terminal["message"])
                    process.stdin.write(b'{"type":"shutdown"}\n')
                    process.stdin.flush()
                else:
                    self.assertEqual(
                        terminal,
                        {"type": "result", "id": 1, "result": event["params"]},
                    )
            self.assertEqual(process.wait(timeout=8), 1 if stop == "crash" else 0)
            with self.assertRaises(ProcessLookupError):
                os.kill(event["params"]["app_server_pid"], 0)
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            process.stdin.close()
            process.stdout.close()
            process.stderr.close()


class BridgeShutdownTests(unittest.TestCase):
    def bridge(self, waits):
        bridge = Bridge.__new__(Bridge)
        bridge.condition = threading.Condition()
        bridge.close_lock = threading.Lock()
        bridge.close_finished = False
        bridge.close_error = None
        bridge.closed = False
        bridge.child = Mock()
        bridge.child.wait.side_effect = waits
        bridge.reader = Mock()
        bridge.reader.is_alive.return_value = False
        return bridge

    def test_forced_escalation_is_bounded_and_never_reports_durable_success(self):
        for last in (-9, subprocess.TimeoutExpired("fixture", 0)):
            with self.subTest(last=type(last).__name__):
                bridge = self.bridge(
                    [
                        subprocess.TimeoutExpired("fixture", 207),
                        subprocess.TimeoutExpired("fixture", 2),
                        last,
                    ]
                )
                # Graceful wait consumes207, TERM consumes the remaining2;
                # KILL receipt and reader join cannot receive a fresh budget.
                with patch(
                    "desktop_ui.gateway.time.monotonic",
                    side_effect=[100, 100, 307, 309, 309],
                ):
                    for _ in range(2):
                        with self.assertRaisesRegex(
                            RuntimeError, "write outcome unknown"
                        ):
                            bridge.close()
                self.assertEqual(
                    [call.kwargs for call in bridge.child.wait.call_args_list],
                    [{"timeout": 207}, {"timeout": 2}, {"timeout": 0}],
                )
                bridge.reader.join.assert_called_once_with(timeout=0)
                bridge.child.terminate.assert_called_once()
                bridge.child.kill.assert_called_once()
                bridge.child.stdin.close.assert_called_once()
                bridge.child.stdout.close.assert_called_once()

    def test_reader_join_uses_only_remaining_absolute_budget(self):
        bridge = self.bridge([0])
        with patch("desktop_ui.gateway.time.monotonic", side_effect=[100, 100, 308.5]):
            bridge.close()
        bridge.child.wait.assert_called_once_with(timeout=207)
        bridge.reader.join.assert_called_once_with(timeout=0.5)
        bridge.child.terminate.assert_not_called()
        bridge.child.kill.assert_not_called()

    def test_nonzero_exit_is_not_masked_by_repeat_cleanup(self):
        for status in (7, 124, 125, 126):
            with self.subTest(status=status):
                bridge = self.bridge([status])
                reason = {
                    124: "watchdog deadline expired",
                    125: "watchdog unavailable",
                    126: "forced process shutdown requested",
                }.get(status)
                for _ in range(2):
                    with self.assertRaisesRegex(
                        RuntimeError, "write outcome unknown"
                    ) as caught:
                        bridge.close()
                    if reason:
                        self.assertIn(reason, str(caught.exception))
                bridge.child.terminate.assert_not_called()
                bridge.child.kill.assert_not_called()
                self.assertEqual(bridge.child.wait.call_count, 1)


class GatewayShutdownBudgetTests(unittest.TestCase):
    def test_http_join_shares_bridge_deadline_across_repeat_close(self):
        gateway = Gateway.__new__(Gateway)
        gateway.close_lock = threading.Lock()
        gateway.close_deadline = None
        gateway.bridge = Mock()
        gateway.server = Mock()
        gateway.thread = Mock()
        gateway.thread.is_alive.return_value = False
        with patch("desktop_ui.gateway.time.monotonic", side_effect=[100, 308.5, 309]):
            gateway.close()
            gateway.close()
        self.assertEqual(
            [call.kwargs for call in gateway.bridge.close.call_args_list],
            [{"deadline": 309}, {"deadline": 309}],
        )
        self.assertEqual(
            [call.kwargs for call in gateway.thread.join.call_args_list],
            [{"timeout": 0.5}, {"timeout": 0}],
        )


if __name__ == "__main__":
    if "--serve" in sys.argv:
        with tempfile.TemporaryDirectory() as directory:
            gateway = Gateway({"codex_bin": str(fixture(directory))})
            print(gateway.url, flush=True)
            try:
                while True:
                    time.sleep(1)
            except KeyboardInterrupt:
                pass
            finally:
                gateway.close()
    else:
        unittest.main()
