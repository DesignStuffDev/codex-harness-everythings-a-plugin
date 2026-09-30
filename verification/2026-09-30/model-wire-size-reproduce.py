"""Compare actual native/model-component large requests using local inference only."""

import base64
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import threading
import time
import uuid
import zlib

WORK = Path(__file__).resolve().parent
REPO = Path("/workspace/codex-harness-everythings-a-plugin")
CLI = Path("/workspace/verified-component-checkpoint-20260930/codex")
HOST = Path("/workspace/verified-component-checkpoint-20260930/codex-component")
MAX_FRAME = 4 * 1024 * 1024
requests = []
commands = []


def fingerprint(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1048576), b""):
            value.update(chunk)
    stat = path.stat()
    return {
        "path": str(path),
        "sha256": value.hexdigest(),
        "bytes": stat.st_size,
        "mtime_ns": stat.st_mtime_ns,
    }


class Provider(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_GET(self):
        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        body = self.rfile.read(int(self.headers["Content-Length"]))
        parsed = json.loads(body)
        index = len(requests)
        path = WORK / f"native-request-{index}.json"
        path.write_bytes(body)
        requests.append(
            {
                "path": str(path),
                "bytes": len(body),
                "sha256": hashlib.sha256(body).hexdigest(),
                "input_items": len(parsed["input"]),
                "url_path": self.path,
            }
        )
        response_id = "resp_" + uuid.uuid4().hex
        item = {
            "type": "message",
            "id": "msg_" + uuid.uuid4().hex,
            "role": "assistant",
            "content": [
                {"type": "output_text", "text": "native fixture accepted request"}
            ],
            "phase": "final_answer",
        }
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        for event in [
            {"type": "response.created", "response": {"id": response_id}},
            {"type": "response.output_item.done", "output_index": 0, "item": item},
            {
                "type": "response.completed",
                "response": {
                    "id": response_id,
                    "end_turn": True,
                    "usage": {"input_tokens": 1, "output_tokens": 1, "total_tokens": 2},
                },
            },
        ]:
            self.wfile.write(("data: " + json.dumps(event) + "\n\n").encode())
            self.wfile.flush()


server = ThreadingHTTPServer(("127.0.0.1", 0), Provider)
threading.Thread(target=server.serve_forever, daemon=True).start()
(WORK / "project").mkdir(exist_ok=True)


def run(name, command, home):
    began = time.monotonic()
    process = subprocess.run(
        [str(x) for x in command],
        cwd=WORK / "project",
        env=dict(os.environ, CODEX_HOME=str(home), PYTHONPATH=""),
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        timeout=90,
    )
    (WORK / (name + ".stdout")).write_text(process.stdout)
    (WORK / (name + ".stderr")).write_text(process.stderr)
    commands.append(
        {
            "name": name,
            "command": [str(x) for x in command],
            "codex_home": str(home),
            "status": process.returncode,
            "elapsed_seconds": time.monotonic() - began,
            "stdout": str(WORK / (name + ".stdout")),
            "stderr": str(WORK / (name + ".stderr")),
        }
    )
    (WORK / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
    return process


def configure(home):
    home.mkdir()
    (home / "config.toml").write_text(f"""model = "gpt-5.1-codex"
model_provider = "fixture"
model_context_window = 100000000
model_auto_compact_token_limit = 100000000
[model_providers.fixture]
name = "Local model wire audit"
base_url = "http://127.0.0.1:{server.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
""")


FIXTURE = """import json,os,pathlib,sys
def send(value): print(json.dumps(value,separators=(',',':')),flush=True)
init=json.loads(sys.stdin.readline())
state=pathlib.Path(init['state_dir'])
(state/'initialized.json').write_text(json.dumps({'pid':os.getpid()}))
send({'type':'ready','api_version':1})
line=sys.stdin.readline()
if not line: sys.exit(0)
request=json.loads(line)
(state/'received.json').write_text(json.dumps({'bytes':len(line.encode()),'method':request['method']}))
send({'type':'event','id':request['id'],'event':{'type':'completed','response_id':'resp_fixture','end_turn':True}})
send({'type':'result','id':request['id'],'result':{}})
assert json.loads(sys.stdin.readline())['type']=='shutdown'
"""


def install(home, label):
    package = WORK / (label + "-package")
    package.mkdir()
    entry = package / "plugin"
    entry.write_text(f"#!{sys.executable}\n" + FIXTURE)
    entry.chmod(0o755)
    (package / "codex-component.json").write_text(
        json.dumps(
            {
                "api_version": 1,
                "id": "test.model-size",
                "version": "0.1.0",
                "entrypoint": "plugin",
                "args": [],
                "dependencies": {},
                "components": [
                    {
                        "kind": "model_transport",
                        "name": "default",
                        "contract_version": 1,
                        "metadata": {},
                    }
                ],
            }
        )
    )
    assert (
        run(
            label + "-install", [HOST, "--codex-home", home, "install", package], home
        ).returncode
        == 0
    )
    assert (
        run(
            label + "-select",
            [
                HOST,
                "--codex-home",
                home,
                "select",
                "model_transport",
                "default",
                "test.model-size",
            ],
            home,
        ).returncode
        == 0
    )
    shutil.rmtree(package)


def png_image():
    width, height = 1200, 1000
    pixels = os.urandom(width * height * 3)
    raw = b"".join(
        b"\0" + pixels[y * width * 3 : (y + 1) * width * 3] for y in range(height)
    )

    def chunk(kind, data):
        return (
            struct.pack("!I", len(data))
            + kind
            + data
            + struct.pack("!I", zlib.crc32(kind + data) & 0xFFFFFFFF)
        )

    image = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack("!IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 1))
        + chunk(b"IEND", b"")
    )
    (WORK / "fixture.png").write_bytes(image)
    return "data:image/png;base64," + base64.b64encode(image).decode()


before = {"codex": fingerprint(CLI), "manager": fingerprint(HOST)}
(WORK / "binaries-before.json").write_text(json.dumps(before, indent=2) + "\n")
cases = []
try:
    for label in ("aggregate", "image"):
        base = WORK / (label + "-base")
        configure(base)
        bootstrap = run(
            label + "-bootstrap",
            [
                CLI,
                "exec",
                "--skip-git-repo-check",
                "--json",
                "Bootstrap audit fixture.",
            ],
            base,
        )
        assert bootstrap.returncode == 0, bootstrap.stderr
        thread_id = next(
            json.loads(line)["thread_id"]
            for line in bootstrap.stdout.splitlines()
            if json.loads(line).get("type") == "thread.started"
        )
        rollout = next((base / "sessions").rglob("*.jsonl"))
        old = [json.loads(line) for line in rollout.read_text().splitlines()]
        ordinal = max(record.get("ordinal", 0) for record in old)
        additions = []
        if label == "aggregate":
            for index in range(280):
                role = "user" if index % 2 == 0 else "assistant"
                additions.append(
                    {
                        "type": "message",
                        "id": f"msg_audit_{index}",
                        "role": role,
                        "content": [
                            {
                                "type": "input_text"
                                if role == "user"
                                else "output_text",
                                "text": f"AUDIT_{index}: " + "x" * 16000,
                            }
                        ],
                    }
                )
        else:
            additions.append(
                {
                    "type": "message",
                    "id": "msg_audit_image",
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": "Inspect this fixture image."},
                        {
                            "type": "input_image",
                            "image_url": png_image(),
                            "detail": "high",
                        },
                    ],
                }
            )
        with rollout.open("a") as stream:
            for index, item in enumerate(additions, 1):
                stream.write(
                    json.dumps(
                        {
                            "timestamp": old[-1]["timestamp"],
                            "ordinal": ordinal + index,
                            "type": "response_item",
                            "payload": item,
                        },
                        separators=(",", ":"),
                    )
                    + "\n"
                )
        native = WORK / (label + "-native")
        plugin = WORK / (label + "-plugin")
        shutil.copytree(base, native)
        shutil.copytree(base, plugin)
        native_count = len(requests)
        command = [
            CLI,
            "exec",
            "resume",
            "--skip-git-repo-check",
            "--json",
            thread_id,
            "Confirm the retained audit context.",
        ]
        accepted = run(label + "-native", command, native)
        assert accepted.returncode == 0, accepted.stderr
        assert len(requests) == native_count + 1, requests
        request = requests[-1]
        assert request["bytes"] > MAX_FRAME, request
        sent = json.loads(Path(request["path"]).read_text())
        if label == "aggregate":
            assert sum("AUDIT_" in json.dumps(item) for item in sent["input"]) == 280
        else:
            encoded = next(
                part["image_url"]
                for item in sent["input"]
                for part in item.get("content", [])
                if part.get("type") == "input_image"
            )
            assert encoded == additions[0]["content"][1]["image_url"]
        install(plugin, label)
        native_count = len(requests)
        rejected = run(label + "-component", command, plugin)
        assert rejected.returncode != 0, rejected.stdout
        assert "frame limit" in rejected.stdout + rejected.stderr, (
            rejected.stdout,
            rejected.stderr,
        )
        assert len(requests) == native_count, "unexpected native inference fallback"
        state = plugin / "components/state/test.model-size"
        pid = json.loads((state / "initialized.json").read_text())["pid"]
        assert not (state / "received.json").exists(), (
            "oversized request reached plugin"
        )
        cases.append(
            {
                "case": label,
                "thread_id": thread_id,
                "native_status": accepted.returncode,
                "component_status": rejected.returncode,
                "native_request": request,
                "seeded_response_items": len(additions),
                "seeded_rollout": str(rollout),
                "equivalent_histories_copied_before_native_resume": True,
                "component_request_received": False,
                "no_native_fallback": True,
                "fixture_pid": pid,
                "fixture_reaped": not Path(f"/proc/{pid}").exists(),
                "component_stdout": rejected.stdout,
                "component_stderr": rejected.stderr,
            }
        )
    after = {"codex": fingerprint(CLI), "manager": fingerprint(HOST)}
    assert before == after, (before, after)
    sources = {
        relative: fingerprint(REPO / relative)
        for relative in (
            "codex-rs/core/src/component_model.rs",
            "codex-rs/core/src/client.rs",
            "codex-rs/component-host/src/process.rs",
            "codex-rs/component-api/src/lib.rs",
        )
    }
    report = {
        "passed": True,
        "proves_gap": True,
        "contract": "model_transport v1; preserved storage-v1-era CLI, current selected-model source retains same per-call 4MiB cap",
        "inference": "local deterministic HTTP fixture only; no external model/credentials",
        "frame_bytes": MAX_FRAME,
        "binaries_before": before,
        "binaries_after": after,
        "sources_audited": sources,
        "cases": cases,
        "limitations": [
            "Histories are controlled fixtures seeded into native rollout files; this tests recovered model input and transport preservation, not provider acceptance/context-window limits.",
            "Model context limits are raised only in isolated acceptance homes to prevent intentional compaction from changing the transport-size comparison.",
        ],
    }
    (WORK / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(WORK / "report.json")
finally:
    server.shutdown()
    server.server_close()
