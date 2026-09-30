"""Exercise installed components in the real CLI, then restore native inference.

Inference is deterministic test data. Engine turns, tools, context, persistence,
resume, streaming adapters, installation and subprocess lifecycle are real.
"""

import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import threading
import time

SDK = Path(__file__).resolve().parents[1]
PLUGIN = """from codex_component_sdk import Plugin
import json
app = Plugin()
@app.handle("model_transport", "default", "model.stream")
def stream(params, context):
    state = context.initialization.state_dir
    index = len(list(state.glob("request-*.json")))
    (state / f"request-{index}.json").write_text(json.dumps(params))
    context.emit({"type": "created", "response_id": f"fixture-{index}"})
    if index == 0:
        item = {"type":"function_call", "id":"fc_acceptance", "call_id":"calculate-acceptance",
                "name":"calculator", "arguments":""}
        context.emit({"type":"output_item_added", "item":item})
        arguments = json.dumps({"operation":"product", "values":[6,7]})
        context.emit({"type":"tool_call_input_delta", "item_id":"fc_acceptance",
                      "call_id":"calculate-acceptance", "delta":arguments})
        context.emit({"type":"output_item_done", "item":dict(item,arguments=arguments)})
    else:
        text = "external-model acceptance complete" if index == 1 else "external-model resumed"
        item = {"type":"message", "id":f"msg_fixture_{index}", "role":"assistant",
                "content":[], "phase":"final_answer"}
        context.emit({"type":"output_item_added", "item":item})
        for offset in range(0,len(text),8):
            context.emit({"type":"output_text_delta", "delta":text[offset:offset+8]})
        context.emit({"type":"output_item_done", "item":dict(item,content=[{"type":"output_text","text":text}])})
    context.emit({"type":"completed", "response_id":f"fixture-{index}", "end_turn":True})
    return {}
"""


def fingerprint(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return {
        "sha256": digest.hexdigest(),
        "mtime_ns": path.stat().st_mtime_ns,
        "bytes": path.stat().st_size,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--codex", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument(
        "--thread-store-package",
        type=Path,
        help="Already independently built native storage package to exercise in the real engine",
    )
    args = parser.parse_args()
    host, codex = args.host.resolve(strict=True), args.codex.resolve(strict=True)
    work = args.work_dir.absolute()
    if SDK.parent == work or SDK.parent in work.parents:
        parser.error("work directory must be outside the harness source")
    work.mkdir(parents=True)
    codex_home = work / "codex-home"
    codex_home.mkdir()
    workspace = work / "project"
    workspace.mkdir()
    before = {"host": fingerprint(host), "codex": fingerprint(codex)}
    commands, native_requests, storage_children = [], [], []
    storage_id = None
    storage_active = False
    if args.thread_store_package:
        package_manifest = json.loads(
            (args.thread_store_package / "codex-component.json").read_text()
        )
        storage_id = package_manifest["id"]
        assert any(
            component["kind"] == "thread_store" and component["name"] == "default"
            for component in package_manifest["components"]
        ), package_manifest

    class Provider(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            self.send_response(404)
            self.end_headers()

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            native_requests.append({"path": self.path, "body": body})
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            for event in [
                {"type": "response.created", "response": {"id": "native-acceptance"}},
                {
                    "type": "response.output_item.done",
                    "output_index": 0,
                    "item": {
                        "type": "message",
                        "id": "msg_native",
                        "role": "assistant",
                        "content": [
                            {"type": "output_text", "text": "native transport restored"}
                        ],
                    },
                },
                {
                    "type": "response.completed",
                    "response": {
                        "id": "native-acceptance",
                        "status": "completed",
                        "usage": {
                            "input_tokens": 1,
                            "output_tokens": 1,
                            "total_tokens": 2,
                        },
                    },
                },
            ]:
                self.wfile.write(("data: " + json.dumps(event) + "\n\n").encode())
                self.wfile.flush()

    server = ThreadingHTTPServer(("127.0.0.1", 0), Provider)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    (codex_home / "config.toml").write_text(f"""model = "gpt-5.1-codex"
model_provider = "acceptance"
[model_providers.acceptance]
name = "Offline acceptance fixture"
base_url = "http://127.0.0.1:{server.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
""")
    environment = dict(os.environ, CODEX_HOME=str(codex_home), PYTHONPATH="")

    def run(command, *, build=False, timeout=90):
        stopped = threading.Event()
        observed = {}

        def watch_storage():
            # Linux acceptance evidence: record only installed native executable
            # paths under this isolated test home, never unrelated command lines.
            prefix = str(codex_home / "components" / "objects") + os.sep
            while not stopped.is_set():
                for process in Path("/proc").glob("[0-9]*"):
                    try:
                        executable = os.readlink(process / "exe")
                    except OSError:
                        continue
                    if executable.startswith(prefix):
                        observed[int(process.name)] = executable
                stopped.wait(0.025)

        monitor = None
        if storage_active and str(command[0]) == str(codex):
            monitor = threading.Thread(target=watch_storage, daemon=True)
            monitor.start()
        try:
            result = subprocess.run(
                command,
                cwd=workspace,
                env=dict(environment, PYTHONPATH=str(SDK)) if build else environment,
                stdin=subprocess.DEVNULL,
                capture_output=True,
                text=True,
                timeout=timeout,
            )
        finally:
            stopped.set()
            if monitor:
                monitor.join()
        for pid, executable in observed.items():
            deadline = time.monotonic() + 3
            while Path(f"/proc/{pid}/exe").exists() and time.monotonic() < deadline:
                time.sleep(0.025)
            assert not Path(f"/proc/{pid}/exe").exists(), (
                f"storage child {pid} survived CLI shutdown"
            )
            storage_children.append(
                {"pid": pid, "executable": executable, "terminated": True}
            )
        entry = {
            "command": [str(value) for value in command],
            "status": result.returncode,
            "stdout": result.stdout,
            "stderr": result.stderr,
        }
        if monitor:
            entry["storage_children"] = [
                {"pid": pid, "executable": executable}
                for pid, executable in observed.items()
            ]
        commands.append(entry)
        (work / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
        if result.returncode:
            raise AssertionError(
                f"command failed: {command}\n{result.stderr}\n{result.stdout}"
            )
        if monitor:
            assert observed, (
                "selected storage process was not observed during the CLI turn"
            )
        return result.stdout

    management = [str(host), "--codex-home", str(codex_home)]
    try:
        for example in ("calculator", "model-fixture"):
            source, package = (
                work / (example + "-source"),
                work / (example + "-package"),
            )
            shutil.copytree(SDK / "examples" / example, source)
            if example == "model-fixture":
                (source / "plugin.py").write_text(PLUGIN)
            run(
                [
                    sys.executable,
                    "-m",
                    "codex_component_sdk",
                    "build",
                    str(source),
                    "--output",
                    str(package),
                ],
                build=True,
            )
            shutil.rmtree(source)
            run(management + ["install", str(package)])
        run(
            management
            + ["select", "model_transport", "default", "example.model-fixture"]
        )
        if storage_id:
            run(management + ["install", str(args.thread_store_package.resolve())])
            run(management + ["select", "thread_store", "default", storage_id])
            storage_active = True
        first = run(
            [
                str(codex),
                "exec",
                "--skip-git-repo-check",
                "--json",
                "Use calculator for six times seven; confirm this acceptance turn.",
            ]
        )
        first_events = [json.loads(line) for line in first.splitlines()]
        thread_id = next(
            event["thread_id"]
            for event in first_events
            if event.get("type") == "thread.started"
        )
        assert "external-model acceptance complete" in first, first
        assert any(event.get("type") == "turn.completed" for event in first_events), (
            first
        )
        state = codex_home / "components" / "state" / "example.model-fixture"
        initial = json.loads((state / "request-0.json").read_text())
        second = json.loads((state / "request-1.json").read_text())
        assert any(
            tool.get("name") == "calculator" for tool in initial["request"]["tools"]
        ), initial["request"]["tools"]
        assert "The calculator tool computes sums and products" in json.dumps(
            initial["request"]
        )
        output = next(
            item
            for item in second["request"]["input"]
            if item.get("type") == "function_call_output"
            and item.get("call_id") == "calculate-acceptance"
        )
        assert "42" in json.dumps(output), output
        assert native_requests == [], native_requests
        resumed = run(
            [
                str(codex),
                "exec",
                "resume",
                "--skip-git-repo-check",
                "--json",
                thread_id,
                "Continue the persisted acceptance session.",
            ]
        )
        assert "external-model resumed" in resumed, resumed
        recovered = json.loads((state / "request-2.json").read_text())
        assert recovered["thread_id"] == initial["thread_id"] == thread_id
        assert "calculate-acceptance" in json.dumps(recovered["request"]["input"])
        assert "external-model acceptance complete" in json.dumps(
            recovered["request"]["input"]
        )
        assert native_requests == [], native_requests
        run(management + ["reset", "model_transport", "default"])
        restored = run(
            [
                str(codex),
                "exec",
                "resume",
                "--skip-git-repo-check",
                "--json",
                thread_id,
                "Continue using native transport.",
            ]
        )
        assert "native transport restored" in restored, restored
        assert len(native_requests) == 1, native_requests
        assert "external-model resumed" in json.dumps(
            native_requests[0]["body"]["input"]
        )
        if storage_id:
            assert len(storage_children) >= 3, storage_children
            run(management + ["reset", "thread_store", "default"])
            storage_active = False
            native_store = run(
                [
                    str(codex),
                    "exec",
                    "resume",
                    "--skip-git-repo-check",
                    "--json",
                    thread_id,
                    "Recover the externally stored history using native storage.",
                ]
            )
            assert "native transport restored" in native_store, native_store
            assert len(native_requests) == 2, native_requests
            assert "external-model resumed" in json.dumps(
                native_requests[1]["body"]["input"]
            )
            assert "native transport restored" in json.dumps(
                native_requests[1]["body"]["input"]
            )
        after = {"host": fingerprint(host), "codex": fingerprint(codex)}
        assert before == after, {"before": before, "after": after}
        (work / "native-requests.json").write_text(
            json.dumps(native_requests, indent=2) + "\n"
        )
        report = {
            "passed": True,
            "inference": "deterministic external process + loopback native fixture; no live model",
            "verified": [
                "independent builds",
                "source removed",
                "real CLI turn",
                "external tool execution",
                "context contribution",
                "model event streaming",
                "persisted session resume",
                "zero native inference while replacement selected",
                "native restoration with retained history",
            ],
            "thread_id": thread_id,
            "binaries_before": before,
            "binaries_after": after,
            "commands_file": str(work / "commands.json"),
            "native_requests_file": str(work / "native-requests.json"),
        }
        if storage_id:
            report["verified"] += [
                "selected native storage process",
                "cold storage process restart and resume",
                "storage child termination after each CLI process",
                "native storage restoration with retained history",
            ]
            report["thread_store_plugin"] = storage_id
            report["storage_children"] = storage_children
        (work / "acceptance.json").write_text(json.dumps(report, indent=2) + "\n")
        print(work / "acceptance.json")
    finally:
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    main()
