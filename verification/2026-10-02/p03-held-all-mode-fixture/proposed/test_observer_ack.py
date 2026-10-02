"""Pure staged proof-gate tests: temp files and mocked /proc; no host/listener."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import threading
import types
import unittest
from unittest import mock


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HERE = Path(__file__).parent
observer = load('observer_ack_candidate', HERE / 'observe_descendants.py')
fixture = load('held_ack_candidate', HERE / 'held_production_host.py')
SHA = 'a' * 64


def receipt():
    return {'schema': 'observed-live-git-readiness-v1', 'case_id': 'fallback-exec-success',
            'wrapped_root_identity': {'pid': 11, 'start_ticks': 21},
            'identity': {'pid': 12, 'start_ticks': 22}, 'executable_identity': '1:2:3:4',
            'sha256': SHA, 'metadata_stable_while_hashed': True,
            'live_generation_rechecked': True, 'observed_monotonic_ns': 123}


class ReceiptPolicy(unittest.TestCase):
    def test_valid_bound_receipt(self):
        fixture.validate_git_ack(receipt(), 'fallback-exec-success', SHA)

    def test_wrong_case_hash_unstable_or_unrefreshed_never_passes(self):
        for key, value in [('case_id', 'fallback-exec-error'), ('sha256', 'b' * 64),
                           ('metadata_stable_while_hashed', False), ('live_generation_rechecked', False)]:
            with self.subTest(key=key):
                value_receipt = receipt()
                value_receipt[key] = value
                with self.assertRaises(fixture.AcceptanceError):
                    fixture.validate_git_ack(value_receipt, 'fallback-exec-success', SHA)

    def test_wrapper_pid_invalid_generation_and_token_fail(self):
        for change in ('wrapper', 'generation', 'token'):
            with self.subTest(change=change):
                value_receipt = receipt()
                if change == 'wrapper': value_receipt['identity']['pid'] = 11
                elif change == 'generation': value_receipt['identity']['start_ticks'] = 0
                else: value_receipt['executable_identity'] = 'unknown'
                with self.assertRaises(fixture.AcceptanceError):
                    fixture.validate_git_ack(value_receipt, 'fallback-exec-success', SHA)

    def test_expired_or_missing_binding_never_releases_502(self):
        proxy = object.__new__(fixture.HeldProxy)
        proxy.git_ack_path, proxy.expected_git_sha, proxy.case_id = None, None, None
        with self.assertRaisesRegex(fixture.AcceptanceError, 'binding_required'):
            proxy.wait_outer_git_ack(0)
        proxy.git_ack_path, proxy.expected_git_sha, proxy.case_id = '/missing', SHA, 'fallback-exec-success'
        proxy.server = types.SimpleNamespace(stop=threading.Event())
        with self.assertRaisesRegex(fixture.AcceptanceError, 'not_observed'):
            proxy.wait_outer_git_ack(0)


class LiveDescriptorProof(unittest.TestCase):
    def exercise(self, *, wrong_hash=False, wrong_generation=False, changed_file=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, ack_path = root / 'selected-git', root / 'ack.json'
            data = b'exact selected executable bytes' * 100
            binary.write_bytes(data)
            expected_sha = hashlib.sha256(data).hexdigest()
            ack = observer.GitAcknowledgment(ack_path, binary,
                                             '0' * 64 if wrong_hash else expected_sha,
                                             'fallback-exec-success')
            with binary.open('rb') as stream:
                before = os.fstat(stream.fileno())
                token = ':'.join(map(str, (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)))
                held = {token: {'fd': stream.fileno(), 'stat': before, 'path': str(binary)}}
                row = {'pid': 123, 'start_ticks': 456, 'state': 'S'}
                refreshed = dict(row, start_ticks=789) if wrong_generation else row.copy()
                current = (types.SimpleNamespace(st_dev=before.st_dev, st_ino=before.st_ino,
                                                st_size=before.st_size, st_mtime_ns=before.st_mtime_ns + 1)
                           if changed_file else before)
                real_stat = os.stat
                def stat(path, *args, **kwargs):
                    return current if str(path) == '/proc/123/exe' else real_stat(path, *args, **kwargs)
                with mock.patch.object(observer, 'identity', return_value=refreshed), mock.patch.object(observer.os, 'stat', side_effect=stat):
                    ack.observe(row, {'identity': token}, held, {'pid': 1, 'start_ticks': 2})
                self.assertEqual(os.lseek(stream.fileno(), 0, os.SEEK_CUR), 0)
                if wrong_hash or wrong_generation or changed_file:
                    self.assertIsNone(ack.receipt)
                    self.assertFalse(ack_path.exists())
                else:
                    self.assertEqual(json.loads(ack_path.read_text()), ack.receipt)
                    self.assertEqual(ack.receipt['sha256'], expected_sha)
                    self.assertFalse(ack_path.with_name('ack.json.pending').exists())
                    # The unchanged final descriptor hash still sees all bytes.
                    self.assertEqual(hashlib.file_digest(stream, 'sha256').hexdigest(), expected_sha)

    def test_verified_live_descriptor_publishes_complete_ack_and_preserves_offset(self):
        self.exercise()

    def test_wrong_hash_does_not_ack(self):
        self.exercise(wrong_hash=True)

    def test_reused_pid_does_not_ack(self):
        self.exercise(wrong_generation=True)

    def test_changed_executable_metadata_does_not_ack(self):
        self.exercise(changed_file=True)


if __name__ == '__main__':
    unittest.main()
