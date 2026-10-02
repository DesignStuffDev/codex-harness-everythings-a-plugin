"""Staged pure fixture-contract tests; no sockets, listeners or host execution.

These do not establish real native lifecycle behavior. Not executed by author.
"""
from pathlib import Path
import importlib.util
import threading
import types
import unittest

SOURCE = Path(__file__).with_name('held_production_host.py')
spec = importlib.util.spec_from_file_location('held_fixture_candidate', SOURCE)
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


def proxy():
    p = object.__new__(fixture.HeldProxy)
    p.mode, p.lock = 'git', threading.Lock()
    p.ready, p.git_seen = threading.Event(), threading.Event()
    p.stop_ns, p.held, p.events, p.git_failures = None, 0, [], 0
    p.auxiliary_started = p.auxiliary_held = p.auxiliary_peer_eof = 0
    p.server = types.SimpleNamespace(stop=threading.Event(), errors=[])
    return p


class Connection:
    def __init__(self, received=b'', during_recv=lambda: None, allow_502=False):
        self.received, self.during_recv = received, during_recv
        self.allow_502, self.sent = allow_502, []

    def recv(self, size):
        self.during_recv()
        return self.received

    def sendall(self, data):
        if not self.allow_502:
            raise AssertionError('auxiliary/held request must send no response')
        self.sent.append(data)


class AuxiliaryContractTests(unittest.TestCase):
    def test_auxiliary_never_satisfies_target_readiness_or_git_failure(self):
        p = proxy()

        def check_while_held():
            self.assertEqual(p.auxiliary_held, 1)
            self.assertEqual(p.held, 0)
            self.assertFalse(p.ready.is_set())
            self.assertFalse(p.git_seen.is_set())
            self.assertEqual(p.git_failures, 0)
            with self.assertRaisesRegex(fixture.AcceptanceError, 'held_transport_gone'):
                p.trigger()

        p.hold_auxiliary_connect(p.server, Connection(during_recv=check_while_held))
        self.assertEqual((p.auxiliary_started, p.auxiliary_held, p.auxiliary_peer_eof), (1, 0, 1))
        self.assertFalse(p.ready.is_set())
        self.assertEqual([e['event'] for e in p.events],
                         ['auxiliary_chatgpt_connect_held', 'auxiliary_chatgpt_peer_eof'])

    def test_second_auxiliary_request_is_rejected_even_after_first_eof(self):
        p = proxy()
        p.hold_auxiliary_connect(p.server, Connection())
        with self.assertRaisesRegex(fixture.AcceptanceError, 'connection_count_exceeded'):
            p.hold_auxiliary_connect(p.server, Connection())
        self.assertEqual((p.auxiliary_started, p.auxiliary_peer_eof), (1, 1))

    def test_new_auxiliary_request_after_stop_is_rejected(self):
        p = proxy()
        p.stop_ns = 1
        with self.assertRaisesRegex(fixture.AcceptanceError, 'after_stop_trigger'):
            p.hold_auxiliary_connect(p.server, Connection())
        self.assertEqual(p.auxiliary_started, 0)

    def test_fixture_release_cannot_count_as_peer_eof(self):
        p = proxy()
        p.server.stop.set()
        with self.assertRaisesRegex(fixture.AcceptanceError, 'released_without_peer_eof'):
            p.hold_auxiliary_connect(p.server, Connection())
        self.assertEqual((p.auxiliary_started, p.auxiliary_held, p.auxiliary_peer_eof), (1, 0, 0))

    def test_unexpected_bytes_are_not_eof_or_success(self):
        p = proxy()
        with self.assertRaisesRegex(fixture.AcceptanceError, 'unexpected_bytes'):
            p.hold_auxiliary_connect(p.server, Connection(received=b'x'))
        self.assertEqual(p.auxiliary_peer_eof, 0)

    def test_other_authority_still_fails_exact_guard(self):
        p = proxy()
        p.server.request = lambda conn: (b'CONNECT', b'unexpected.example:443', {}, b'')
        with self.assertRaisesRegex(fixture.AcceptanceError, 'unexpected_proxy_authority'):
            p.handle(p.server, Connection())
        self.assertEqual(p.auxiliary_started, 0)

    def test_git_failure_still_sends_only_502_without_target_readiness(self):
        p = proxy()
        p.mode = 'fallback'
        p.git_seen.set()
        p.server.request = lambda conn: (b'CONNECT', b'github.com:443', {}, b'')
        c = Connection(allow_502=True)
        p.handle(p.server, c)
        self.assertEqual(p.git_failures, 1)
        self.assertFalse(p.ready.is_set())
        self.assertEqual(p.auxiliary_started, 0)
        self.assertEqual(len(c.sent), 1)
        self.assertTrue(c.sent[0].startswith(b'HTTP/1.1 502'))


if __name__ == '__main__':
    unittest.main()
