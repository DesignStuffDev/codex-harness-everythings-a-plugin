"""Pure all-mode acknowledgment checks; no host, subprocess or listener.

Staged only. The author has not executed these tests.
"""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import threading
import types
import unittest
from unittest import mock


HERE = Path(__file__).parent


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


fixture = load("held_all_mode_candidate", HERE / "held_production_host.py")
observer = load("observer_all_mode_candidate", HERE / "observe_descendants.py")
SHA = "a" * 64


def proxy(mode, directory):
    p = object.__new__(fixture.HeldProxy)
    p.mode, p.lock = mode, threading.Lock()
    p.ready, p.git_seen = threading.Event(), threading.Event()
    p.git_seen.set()
    p.stop_ns, p.held, p.events, p.git_failures = None, 0, [], 0
    p.auxiliary_started = p.auxiliary_held = p.auxiliary_peer_eof = 0
    p.git_ack_path = directory / "observer-git-ready.json"
    p.expected_git_sha, p.case_id = SHA, mode + "-exec-success"
    p.outer_git_acknowledgment = None
    p.observations = fixture.ConnectionObservations()
    p.server = types.SimpleNamespace(stop=threading.Event(), errors=[], observations=p.observations)
    p.server.request = lambda conn: (b"CONNECT", b"github.com:443", {}, b"")
    return p


def receipt(p):
    return {
        "schema": "observed-live-git-readiness-v1", "case_id": p.case_id,
        "wrapped_root_identity": {"pid": 11, "start_ticks": 21},
        "identity": {"pid": 12, "start_ticks": 22},
        "executable_identity": "1:2:3:4", "sha256": SHA,
        "metadata_stable_while_hashed": True, "live_generation_rechecked": True,
        "observed_monotonic_ns": 123,
    }


class Connection:
    def __init__(self, during_recv=lambda: None, during_send=lambda: None):
        self.during_recv, self.during_send = during_recv, during_send
        self.sent, self.receives = [], 0

    def recv(self, size):
        self.receives += 1
        self.during_recv()
        return b""

    def sendall(self, data):
        self.during_send()
        self.sent.append(data)


class AllModeAcknowledgmentTests(unittest.TestCase):
    def test_both_modes_require_ack_before_held_readiness_and_stop(self):
        for mode in ("git", "fallback"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                p = proxy(mode, Path(directory))
                bound = receipt(p)
                p.git_ack_path.write_text(json.dumps(bound))
                original_wait = p.wait_outer_git_ack
                deadlines = []

                def wait(deadline):
                    deadlines.append(deadline)
                    self.assertFalse(p.ready.is_set())
                    self.assertEqual(p.held, 0)
                    with self.assertRaisesRegex(fixture.AcceptanceError, "held_transport_gone"):
                        p.trigger()
                    original_wait(deadline)

                p.wait_outer_git_ack = wait

                def while_held():
                    self.assertEqual(p.outer_git_acknowledgment, bound)
                    self.assertTrue(p.ready.is_set())
                    self.assertEqual(p.held, 1)
                    p.trigger()

                def before_502():
                    self.assertEqual(p.outer_git_acknowledgment, bound)
                    self.assertFalse(p.ready.is_set())
                    self.assertEqual(p.held, 0)

                connection = Connection(while_held, before_502)
                with mock.patch.object(fixture.time, "monotonic", return_value=100):
                    p.handle(p.server, connection)
                self.assertEqual(deadlines, [100 + fixture.READY])
                if mode == "fallback":
                    self.assertEqual(len(connection.sent), 1)
                    self.assertTrue(connection.sent[0].startswith(b"HTTP/1.1 502"))
                    self.assertFalse(p.ready.is_set())
                    p.server.request = lambda conn: (b"CONNECT", b"api.github.com:443", {}, b"")
                    p.handle(p.server, connection)
                else:
                    self.assertEqual(connection.sent, [])
                names = [event["event"] for event in p.events]
                self.assertLess(names.index("outer_git_hash_acknowledged"), names.index("held_connect_ready"))
                self.assertLess(names.index("held_connect_ready"), names.index("host_stop_trigger"))
                self.assertEqual(connection.receives, 1)
                self.assertEqual(p.held, 0)

    def test_missing_ack_expires_original_deadline_without_readiness_or_502(self):
        for mode in ("git", "fallback"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                p, connection = proxy(mode, Path(directory)), Connection()
                with mock.patch.object(fixture.time, "monotonic", side_effect=[100, 101, 161]), mock.patch.object(p.server.stop, "wait", return_value=False):
                    with self.assertRaisesRegex(fixture.AcceptanceError, "outer_real_git_ack_not_observed"):
                        p.handle(p.server, connection)
                self.assertFalse(p.ready.is_set())
                self.assertEqual((p.held, p.git_failures, connection.receives), (0, 0, 0))
                self.assertEqual(connection.sent, [])
                self.assertIsNone(p.outer_git_acknowledgment)

    def test_invalid_ack_never_enables_either_mode(self):
        changes = [("case_id", "other-case"), ("sha256", "b" * 64),
                   ("metadata_stable_while_hashed", False), ("live_generation_rechecked", False)]
        for mode in ("git", "fallback"):
            for key, value in changes:
                with self.subTest(mode=mode, field=key), tempfile.TemporaryDirectory() as directory:
                    p, connection = proxy(mode, Path(directory)), Connection()
                    bound = receipt(p)
                    bound[key] = value
                    p.git_ack_path.write_text(json.dumps(bound))
                    with self.assertRaises(fixture.AcceptanceError):
                        p.handle(p.server, connection)
                    self.assertFalse(p.ready.is_set())
                    self.assertEqual((p.held, p.git_failures, connection.receives), (0, 0, 0))
                    self.assertEqual(connection.sent, [])
                    self.assertIsNone(p.outer_git_acknowledgment)

    def test_deadline_expiring_after_valid_ack_read_is_still_failure(self):
        for mode in ("git", "fallback"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                p, connection = proxy(mode, Path(directory)), Connection()
                p.git_ack_path.write_text(json.dumps(receipt(p)))
                with mock.patch.object(fixture.time, "monotonic", side_effect=[100, 101, 161]):
                    with self.assertRaisesRegex(fixture.AcceptanceError, "outer_real_git_ack_not_observed"):
                        p.handle(p.server, connection)
                self.assertFalse(p.ready.is_set())
                self.assertEqual((p.held, p.git_failures, connection.receives), (0, 0, 0))
                self.assertEqual(connection.sent, [])
                self.assertIsNone(p.outer_git_acknowledgment)

    def test_ack_cannot_cross_existing_stop_fence(self):
        for mode in ("git", "fallback"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                p = proxy(mode, Path(directory))
                p.git_ack_path.write_text(json.dumps(receipt(p)))
                p.stop_ns = 1
                with mock.patch.object(fixture.time, "monotonic", return_value=100):
                    with self.assertRaisesRegex(fixture.AcceptanceError, "outer_git_ack_after_stop"):
                        p.wait_outer_git_ack(160)
                self.assertFalse(p.ready.is_set())
                self.assertIsNone(p.outer_git_acknowledgment)

    def test_fixture_pins_actual_changed_observer(self):
        self.assertEqual(fixture.OBSERVER_SHA, hashlib.sha256((HERE / "observe_descendants.py").read_bytes()).hexdigest())

    def parser_argv(self, root, case_id):
        return [str(HERE / "observe_descendants.py"), "--report", str(root / "observer.json"),
                "--git-ready-ack", str(root / "ack.json"),
                "--git-ready-executable", str((HERE / "observe_descendants.py").resolve()),
                "--git-ready-sha256", SHA, "--git-ready-case-id", case_id, "--", "not-executed"]

    def test_observer_accepts_all_eight_ack_bindings_before_any_launch(self):
        class BeforeOutputReservation(Exception):
            pass

        for mode in ("git", "fallback"):
            for path in fixture.PATHS:
                with self.subTest(mode=mode, path=path), tempfile.TemporaryDirectory() as directory:
                    argv = self.parser_argv(Path(directory), mode + "-" + path)
                    with mock.patch.object(observer.sys, "argv", argv), mock.patch.object(observer.os, "open", side_effect=BeforeOutputReservation) as opened, mock.patch.object(observer.subprocess, "Popen") as launched:
                        with self.assertRaises(BeforeOutputReservation):
                            observer.main()
                        opened.assert_called_once()
                        launched.assert_not_called()

    def test_observer_rejects_unknown_case_before_output_or_launch(self):
        for case_id in ("git-unknown", "fallback-unknown", "other-exec-success"):
            with self.subTest(case_id=case_id), tempfile.TemporaryDirectory() as directory:
                argv = self.parser_argv(Path(directory), case_id)
                with mock.patch.object(observer.sys, "argv", argv), mock.patch.object(observer.os, "open") as opened, mock.patch.object(observer.subprocess, "Popen") as launched, contextlib.redirect_stderr(io.StringIO()):
                    with self.assertRaises(SystemExit) as stopped:
                        observer.main()
                    self.assertEqual(stopped.exception.code, 2)
                    opened.assert_not_called()
                    launched.assert_not_called()


if __name__ == "__main__":
    unittest.main()
