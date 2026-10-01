"""Authenticated gateway search forwarding; subprocess fixture is not real-engine proof."""

import http.client
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import test_desktop


class DesktopSearchTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.gateway = test_desktop.Gateway(
            {"codex_bin": str(test_desktop.fixture(self.temporary.name)), "cwd": self.temporary.name}
        )

    def tearDown(self):
        self.gateway.close()
        self.temporary.cleanup()

    def request(self, path, body=None, origin=None, authorized=True):
        connection = http.client.HTTPConnection("127.0.0.1", self.gateway.server.server_port, timeout=5)
        connection.request("POST" if body is not None else "GET", path, json.dumps(body) if body is not None else None, {
            "Origin": origin or self.gateway.origin,
            "Authorization": "Bearer " + (self.gateway.token if authorized else "invalid"),
        })
        response = connection.getresponse()
        status, data = response.status, response.read()
        connection.close()
        return status, data

    def test_only_authenticated_search_is_forwarded_and_errors_are_preserved(self):
        params = {"query": "needle", "roots": [self.temporary.name], "cancellationToken": "page:1"}
        frame = {"method": "fuzzyFileSearch", "params": params}
        failure = {"error": {"code": -32000, "message": "selected search failed"}}
        with patch.object(self.gateway.bridge, "rpc", return_value=failure) as rpc:
            self.assertEqual(self.request("/rpc", frame, authorized=False)[0], 403)
            self.assertEqual(self.request("/rpc", frame, origin="https://untrusted.example")[0], 403)
            rpc.assert_not_called()
            status, data = self.request("/rpc", frame)
            self.assertEqual((status, json.loads(data)), (200, failure))
            rpc.assert_called_once_with("fuzzyFileSearch", params)
        self.assertEqual(self.request("/rpc", {"method": "command/exec", "params": {}})[0], 400)

    def test_default_root_and_new_static_resources_are_packaged_routes(self):
        status, data = self.request("/status")
        self.assertEqual(status, 200)
        self.assertEqual(json.loads(data)["cwd"], str(Path(self.temporary.name).resolve()))
        self.assertEqual(json.loads(data)["path_style"], "windows" if os.name == "nt" else "posix")
        for route in ("/file-search.js", "/file-picker.js"):
            status, data = self.request(route)
            self.assertEqual(status, 200)
            self.assertGreater(len(data), 100)
