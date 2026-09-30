"""Exercise built child processes outside both the plugin and harness sources."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SDK = Path(__file__).resolve().parents[1]


class PackagedPluginTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="external-codex-plugin-")
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.source = self.directory / "developer-source"
        self.package = self.directory / "package"
        self.environment = dict(os.environ, PYTHONPATH=str(SDK))

    def build(self, example="calculator", plugin=None):
        shutil.copytree(SDK / "examples" / example, self.source)
        if plugin:
            (self.source / "plugin.py").write_text(plugin)
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "codex_component_sdk",
                "build",
                str(self.source),
                "--output",
                str(self.package),
            ],
            cwd=self.directory,
            env=self.environment,
            capture_output=True,
            text=True,
            timeout=15,
        )
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        shutil.rmtree(self.source)

    def exchange(
        self, kind="tool", name="calculator", method="invoke", params=None, config=None
    ):
        initialization = {
            "type": "initialize",
            "api_version": 1,
            "plugin_id": "example.test",
            "config": config or {},
            "state_dir": str(self.directory / "state"),
        }
        request = {
            "type": "request",
            "id": 1,
            "component": {"kind": kind, "name": name},
            "method": method,
            "params": params or {"arguments": {"values": [1, 2, 3]}},
        }
        result = self.run_package(
            "".join(
                json.dumps(frame) + "\n"
                for frame in [initialization, request, {"type": "shutdown"}]
            )
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return [json.loads(line) for line in result.stdout.splitlines()], result.stderr

    def run_package(self, frames):
        return subprocess.run(
            [sys.executable, str(self.package / "plugin.pyz")],
            input=frames,
            cwd=self.directory,
            env=dict(os.environ, PYTHONPATH=""),
            capture_output=True,
            text=True,
            timeout=10,
        )

    def test_packaged_tool_streams_and_exits_without_source(self):
        self.build()
        frames, errors = self.exchange()
        self.assertEqual(
            (frames, errors),
            (
                [
                    {"type": "ready", "api_version": 1},
                    {
                        "type": "event",
                        "id": 1,
                        "event": {
                            "type": "calculation_completed",
                            "operation": "sum",
                            "count": 3,
                        },
                    },
                    {
                        "type": "result",
                        "id": 1,
                        "result": {"text": "6.0", "success": True},
                    },
                ],
                "",
            ),
        )

    def test_context_component(self):
        self.build()
        frames, _ = self.exchange(
            kind="context",
            name="calculator_help",
            method="contribute",
            params={"thread_id": "thread-1"},
        )
        self.assertEqual(
            frames[-1],
            {
                "type": "result",
                "id": 1,
                "result": {
                    "text": "The calculator tool computes sums and products of up to 100 finite numbers."
                },
            },
        )

    def test_handler_failure_does_not_disclose_exception_value(self):
        self.build(
            plugin='from codex_component_sdk import Plugin\napp = Plugin()\n@app.handle("tool", "calculator", "invoke")\ndef handler(params, context):\n    raise RuntimeError("private credential value")\n'
        )
        frames, errors = self.exchange()
        self.assertEqual(
            frames[-1],
            {"type": "error", "id": 1, "message": "component handler failed"},
        )
        self.assertNotIn("private credential value", errors)

    def test_unknown_handler_is_public_error(self):
        self.build()
        frames, _ = self.exchange(name="missing")
        self.assertEqual(
            frames[-1],
            {"type": "error", "id": 1, "message": "unsupported component or method"},
        )

    def test_invalid_handshake_exits_without_reflecting_input(self):
        self.build()
        result = self.run_package(
            '{"type":"initialize","api_version":99,"secret":"private credential"}\n'
        )
        self.assertEqual((result.returncode, result.stdout), (2, ""))
        self.assertNotIn("private credential", result.stderr)

    def test_frame_limit_and_malformed_json(self):
        self.build()
        for raw in ("x" * (4 * 1024 * 1024 + 1), "not JSON\n"):
            result = self.run_package(raw)
            self.assertEqual((result.returncode, result.stdout), (2, ""))

    def test_output_limit_returns_bounded_error(self):
        self.build(
            plugin='from codex_component_sdk import Plugin\napp = Plugin()\n@app.handle("tool", "calculator", "invoke")\ndef handler(params, context):\n    return "x" * (4 * 1024 * 1024)\n'
        )
        frames, _ = self.exchange()
        self.assertEqual(
            frames[-1],
            {
                "type": "error",
                "id": 1,
                "message": "response exceeds protocol frame limit",
            },
        )

    def test_shutdown_persists_explicit_state_and_prints_do_not_corrupt_protocol(self):
        self.build(
            plugin='from codex_component_sdk import Plugin\napp = Plugin()\n@app.handle("tool", "calculator", "invoke")\ndef handler(params, context):\n    print("diagnostic")\n    return {}\n@app.on_shutdown\ndef close(initialization):\n    initialization.state_dir.mkdir()\n    (initialization.state_dir / "closed").write_text("yes")\n'
        )
        frames, errors = self.exchange()
        self.assertEqual(
            (frames[-1], errors, (self.directory / "state" / "closed").read_text()),
            ({"type": "result", "id": 1, "result": {}}, "diagnostic\n", "yes"),
        )

    def test_active_handler_observes_buffered_shutdown_before_returning(self):
        self.build(
            plugin='from codex_component_sdk import Plugin\napp = Plugin()\n@app.handle("tool", "calculator", "invoke")\ndef handler(params, context):\n    first = context.watch_shutdown()\n    assert first is context.watch_shutdown()\n    assert first.wait(5), "shutdown never reached active handler"\n    return {"drained": True}\n'
        )
        frames, errors = self.exchange()
        self.assertEqual(
            (frames[-1], errors),
            ({"type": "result", "id": 1, "result": {"drained": True}}, ""),
        )

    def test_active_handler_rejects_new_requests_during_shutdown(self):
        self.build(
            plugin='from codex_component_sdk import Plugin\napp = Plugin()\n@app.handle("tool", "calculator", "invoke")\ndef handler(params, context):\n    assert context.watch_shutdown().wait(5)\n    return {}\n'
        )
        frames = [
            {
                "type": "initialize",
                "api_version": 1,
                "plugin_id": "test",
                "config": {},
                "state_dir": str(self.directory),
            },
            {
                "type": "request",
                "id": 1,
                "component": {"kind": "tool", "name": "calculator"},
                "method": "invoke",
                "params": {},
            },
            {"type": "request", "id": 2, "secret": "private credential"},
        ]
        result = self.run_package("".join(json.dumps(frame) + "\n" for frame in frames))
        self.assertEqual(result.returncode, 2)
        self.assertEqual(
            json.loads(result.stdout.splitlines()[-1]),
            {"type": "error", "id": 1, "message": "invalid shutdown control frame"},
        )
        self.assertNotIn("private credential", result.stderr)

    def test_model_fixture_streaming_sequence(self):
        self.build("model-fixture")
        frames, _ = self.exchange(
            kind="model_transport",
            name="default",
            method="model.stream",
            params={"thread_id": "thread-1", "request": {}},
            config={"text": "Hello"},
        )
        self.assertEqual(
            [frame.get("event", {}).get("type") for frame in frames[1:-1]],
            [
                "created",
                "output_item_added",
                "output_text_delta",
                "output_item_done",
                "completed",
            ],
        )
        self.assertEqual(frames[-1], {"type": "result", "id": 1, "result": {}})
        again, _ = self.exchange(
            kind="model_transport",
            name="default",
            method="model.stream",
            params={"thread_id": "thread-1", "request": {}},
            config={"text": "Hello"},
        )
        self.assertNotEqual(
            frames[1]["event"]["response_id"], again[1]["event"]["response_id"]
        )
        self.assertNotEqual(
            frames[2]["event"]["item"]["id"], again[2]["event"]["item"]["id"]
        )
        self.assertEqual(
            frames[3]["event"]["delta"],
            frames[4]["event"]["item"]["content"][0]["text"],
        )

    def test_template_build_and_run(self):
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "codex_component_sdk",
                "init",
                str(self.source),
                "--id",
                "custom.greeting",
            ],
            cwd=self.directory,
            env=self.environment,
            capture_output=True,
            text=True,
            timeout=10,
        )
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        subprocess.run(
            [
                sys.executable,
                "-m",
                "codex_component_sdk",
                "build",
                str(self.source),
                "--output",
                str(self.package),
            ],
            cwd=self.directory,
            env=self.environment,
            check=True,
            capture_output=True,
            timeout=10,
        )
        shutil.rmtree(self.source)
        frames, _ = self.exchange(
            name="greeting", params={"arguments": {"name": "Codex"}}
        )
        self.assertEqual(
            frames[-1],
            {
                "type": "result",
                "id": 1,
                "result": {"text": "Hello, Codex!", "success": True},
            },
        )

    def test_packaged_static_resources_and_nested_output(self):
        shutil.copytree(SDK / "examples" / "calculator", self.source)
        resources = self.source / "example_ui" / "static"
        resources.mkdir(parents=True)
        (resources.parent / "__init__.py").write_text("")
        (resources / "index.html").write_text("<p>Packaged UI</p>")
        (self.source / "plugin.py").write_text(
            'from codex_component_sdk import Plugin\nfrom importlib.resources import files\napp = Plugin()\n@app.handle("tool", "calculator", "invoke")\ndef handler(params, context):\n    return files("example_ui").joinpath("static/index.html").read_text()\n'
        )
        nested_output = self.source / "dist"
        subprocess.run(
            [
                sys.executable,
                "-m",
                "codex_component_sdk",
                "build",
                str(self.source),
                "--output",
                str(nested_output),
            ],
            cwd=self.directory,
            env=self.environment,
            check=True,
            capture_output=True,
            timeout=10,
        )
        shutil.move(nested_output, self.package)
        shutil.rmtree(self.source)
        frames, _ = self.exchange()
        self.assertEqual(
            frames[-1], {"type": "result", "id": 1, "result": "<p>Packaged UI</p>"}
        )

    @unittest.skipIf(os.name == "nt", "Windows host invokes Python for zipapps")
    def test_unix_package_entrypoint_is_directly_executable(self):
        self.build()
        result = subprocess.run(
            [str(self.package / "plugin.pyz")],
            input='{"type":"initialize","api_version":1,"plugin_id":"test","config":{},"state_dir":"/tmp"}\n{"type":"shutdown"}\n',
            cwd=self.directory,
            env=dict(os.environ, PYTHONPATH=""),
            capture_output=True,
            text=True,
            timeout=10,
        )
        self.assertEqual(
            (result.returncode, result.stdout, result.stderr),
            (0, '{"type":"ready","api_version":1}\n', ""),
        )


if __name__ == "__main__":
    unittest.main()
