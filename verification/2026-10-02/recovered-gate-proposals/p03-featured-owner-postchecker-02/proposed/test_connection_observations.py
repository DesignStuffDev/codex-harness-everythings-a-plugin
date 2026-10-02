"""Pure diagnostic-contract checks. No listener, subprocess or actual host.

These have not been run by the proposal author.
"""
import ast
import importlib.util
import json
from pathlib import Path
import threading
import unittest
from unittest import mock

SOURCE = Path(__file__).with_name("held_production_host.py")
spec = importlib.util.spec_from_file_location("held_observation_candidate", SOURCE)
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


class Connection:
    def __init__(self, parts):
        self.parts, self.closed = list(parts), False

    def __enter__(self):
        return self

    def __exit__(self, *unused):
        self.closed = True

    def recv(self, size):
        return self.parts.pop(0) if self.parts else b""

    def sendall(self, data):
        raise AssertionError("diagnostic must not answer or forward the held connection")


def server_and_proxy():
    server = object.__new__(fixture.LoopbackServer)
    server.stop, server.errors = threading.Event(), []
    server.observations = fixture.ConnectionObservations()
    proxy = object.__new__(fixture.HeldProxy)
    proxy.mode, proxy.lock = "git", threading.Lock()
    proxy.ready, proxy.git_seen = threading.Event(), threading.Event()
    proxy.stop_ns, proxy.held, proxy.events, proxy.git_failures = None, 0, [], 0
    proxy.auxiliary_started = proxy.auxiliary_held = proxy.auxiliary_peer_eof = 0
    proxy.server, proxy.observations = server, server.observations
    server.handler = proxy.handle
    return server, proxy


class ConnectionObservationTests(unittest.TestCase):
    def test_rejection_is_recorded_but_still_closes_without_admission_or_eof(self):
        server, proxy = server_and_proxy()
        proxy.stop_ns = 1
        conn = Connection([b"CONNECT chatgpt.com:443 HTTP/1.1\r\n\r\n"])
        fixture.observe_connection(server, "accept_return", conn, observed_ns=1)
        server.connection(conn)
        self.assertTrue(conn.closed)
        self.assertEqual(server.errors, ["auxiliary_request_after_stop_trigger"])
        self.assertEqual((proxy.auxiliary_started, proxy.auxiliary_held, proxy.auxiliary_peer_eof), (0, 0, 0))
        self.assertFalse(proxy.ready.is_set())
        rows = server.observations.snapshot()["events"]
        self.assertEqual([r["event"] for r in rows], [
            "accept_return", "handler_entry", "first_recv_return", "header_parsed",
            "route_classified", "auxiliary_guard_entry", "auxiliary_policy_rejected_after_stop"])
        self.assertEqual({r["connection_sequence"] for r in rows}, {1})

    def test_pre_stop_header_and_post_stop_guard_remains_failure(self):
        server, proxy = server_and_proxy()
        request = server.request

        def parsed_then_stopped(conn):
            parsed = request(conn)
            proxy.stop_ns = fixture.time.monotonic_ns()
            fixture.observe_connection(server, "external_stop_marker", observed_ns=proxy.stop_ns)
            return parsed

        server.request = parsed_then_stopped
        conn = Connection([b"CONNECT chatgpt.com:443 HTTP/1.1\r\n\r\n"])
        server.connection(conn)
        names = [r["event"] for r in server.observations.snapshot()["events"]]
        self.assertLess(names.index("header_parsed"), names.index("external_stop_marker"))
        self.assertLess(names.index("external_stop_marker"), names.index("auxiliary_policy_rejected_after_stop"))
        times = {r["event"]: r["monotonic_ns"] for r in server.observations.snapshot()["events"]}
        self.assertLessEqual(times["header_parsed"], times["external_stop_marker"])
        self.assertLessEqual(times["external_stop_marker"], times["auxiliary_policy_rejected_after_stop"])
        self.assertEqual(server.errors, ["auxiliary_request_after_stop_trigger"])
        self.assertTrue(conn.closed)
        self.assertEqual(proxy.auxiliary_started, 0)

    def test_only_first_receive_is_recorded_and_payload_is_not_retained(self):
        server, proxy = server_and_proxy()
        proxy.stop_ns = 1
        conn = Connection([b"CONNECT chatgpt.com:443 HTTP/1.1\r\n", b"Authorization: synthetic-secret\r\n", b"X-Private: synthetic-value\r\n\r\n"])
        server.connection(conn)
        rows = server.observations.snapshot()["events"]
        self.assertEqual(sum(r["event"] == "first_recv_return" for r in rows), 1)
        self.assertEqual([r["route_class"] for r in rows if "route_class" in r], ["auxiliary"])
        text = json.dumps(server.observations.snapshot())
        for sensitive in ("chatgpt.com", "Authorization", "synthetic-secret", "synthetic-value"):
            self.assertNotIn(sensitive, text)
        self.assertTrue(all(set(row) <= {"event", "monotonic_ns", "connection_sequence", "route_class"} for row in rows))

    def test_model_or_uninstrumented_server_remains_supported(self):
        server, unused = server_and_proxy()
        server.observations = None
        method, path, headers, body = server.request(Connection([b"POST /v1/responses HTTP/1.1\r\nContent-Length: 0\r\n\r\n"]))
        self.assertEqual((method, path, headers, body), (b"POST", b"/v1/responses", {b"content-length": b"0"}, b""))
        fixture.observe_connection(object(), "handler_entry")

    def test_capture_precedes_diagnostic_lock_acquisition(self):
        observations = fixture.ConnectionObservations()
        ticks = []

        class Lock:
            def __enter__(self):
                ticks.append(fixture.time.monotonic_ns())
            def __exit__(self, *unused):
                pass

        observations.lock = Lock()
        with mock.patch.object(fixture.time, "monotonic_ns", side_effect=[10, 20]):
            observations.record("handler_entry", object())
        observations.lock = threading.Lock()
        self.assertEqual(ticks, [20])
        self.assertEqual(observations.snapshot()["events"][0]["monotonic_ns"], 10)

    def test_per_connection_overflow_is_explicit_and_bounded(self):
        observations, conn = fixture.ConnectionObservations(), object()
        for tick in range(17):
            observations.record("handler_entry", conn, observed_ns=tick)
        snap = observations.snapshot()
        self.assertEqual(len(snap["events"]), 16)
        self.assertTrue(snap["overflow"])
        self.assertFalse(snap["records_within_bounds"])
        self.assertFalse(snap["native_admission_observed"])

    def test_global_connection_cap_is_explicit_and_bounded(self):
        observations = fixture.ConnectionObservations()
        connections = [object() for unused in range(129)]
        for conn in connections:
            observations.record("accept_return", conn, observed_ns=1)
        snap = observations.snapshot()
        self.assertEqual(len(snap["events"]), 128)
        self.assertTrue(snap["overflow"])
        self.assertFalse(snap["records_within_bounds"])

    def test_every_original_requirement_and_policy_constant_is_preserved(self):
        original = ast.parse((SOURCE.parent.parent / "preimage/held_production_host.py").read_text())
        proposed = ast.parse(SOURCE.read_text())

        def requirements(tree):
            return [ast.dump(node, include_attributes=False) for node in ast.walk(tree)
                    if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "require"]

        # Sort because added classes change breadth-first AST traversal depth.
        self.assertEqual(sorted(requirements(original)), sorted(requirements(proposed)))
        def constants(tree):
            return [ast.dump(node, include_attributes=False) for node in tree.body
                    if isinstance(node, ast.Assign) and any(isinstance(t, ast.Tuple) and
                        [e.id for e in t.elts if isinstance(e, ast.Name)] == ["GRACEFUL", "HARD", "OUTER", "READY"] for t in node.targets)]
        self.assertEqual(constants(original), constants(proposed))


if __name__ == "__main__":
    unittest.main()
