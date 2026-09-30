"""Prepare independent plugins for manual GUI checks against the real app-server.

This test model deliberately requests a harmless native command approval and
holds a cancellable stream. It is never a substitute app-server or product UI.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

SDK = Path(__file__).resolve().parents[1]
PLUGIN = """from codex_component_sdk import Plugin
import json
import os
import threading
import time
from uuid import uuid4

app = Plugin()

@app.handle("model_transport", "default", "model.stream")
def stream(params, context):
    nonce = uuid4().hex
    state = context.initialization.state_dir
    (state / f"request-{nonce}.json").write_text(json.dumps(params, indent=2))
    def emit(event):
        with (state / f"events-{nonce}.jsonl").open("a") as log:
            log.write(json.dumps(event) + "\\n")
        context.emit(event)
    inputs = params["request"]["input"]
    latest_user = max((index for index,item in enumerate(inputs)
                       if item.get("type") == "message" and item.get("role") == "user"), default=-1)
    latest = inputs[latest_user] if latest_user >= 0 else {}
    content = latest.get("content", [])
    prompt = content if isinstance(content,str) else "\\n".join(part.get("text", "") for part in content)
    outputs = [item for item in inputs[latest_user + 1:]
               if item.get("type") == "function_call_output"
               and item.get("call_id", "").startswith("gui-approval-")]
    emit({"type":"created", "response_id":f"response_{nonce}"})
    if "approval" in prompt.lower() and not outputs:
        arguments = json.dumps({"cmd":"printf gui-native-approval",
            "sandbox_permissions":"require_escalated", "justification":"Verify local GUI approval flow",
            "max_output_tokens":100, "yield_time_ms":1000})
        item = {"type":"function_call", "id":f"fc_{nonce}", "call_id":f"gui-approval-{nonce}",
                "name":"exec_command", "arguments":""}
        emit({"type":"output_item_added", "item":item})
        emit({"type":"tool_call_input_delta", "item_id":item["id"], "call_id":item["call_id"], "delta":arguments})
        emit({"type":"output_item_done", "item":dict(item,arguments=arguments)})
    else:
        item = {"type":"message", "id":f"msg_{nonce}", "role":"assistant", "content":[], "phase":"final_answer"}
        emit({"type":"output_item_added", "item":item})
        if "cancel" in prompt.lower() and not outputs:
            (state / f"blocked-{params['thread_id']}.json").write_text(json.dumps({"pid":os.getpid(),"invocation":nonce}))
            emit({"type":"output_text_delta", "delta":"Waiting for Stop: this model process is deliberately blocked."})
            threading.Event().wait()
        if outputs:
            output = outputs[-1].get("output", "")
            text = "Native command approval result: " + (output if isinstance(output,str) else json.dumps(output))
        else:
            text = "GUI fixture response: " + prompt[-3000:]
        for offset in range(0,len(text),12):
            emit({"type":"output_text_delta", "delta":text[offset:offset + 12]})
            time.sleep(0.025)
        emit({"type":"output_item_done", "item":dict(item,content=[{"type":"output_text","text":text}])})
    emit({"type":"completed", "response_id":f"response_{nonce}", "end_turn":True})
    return {}
"""


def fingerprint(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return {
        "path": str(path),
        "sha256": digest.hexdigest(),
        "mtime_ns": path.stat().st_mtime_ns,
        "bytes": path.stat().st_size,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True, type=Path)
    parser.add_argument("--codex", required=True, type=Path)
    parser.add_argument("--work-dir", required=True, type=Path)
    args = parser.parse_args()
    host, codex = args.host.resolve(strict=True), args.codex.resolve(strict=True)
    work = args.work_dir.absolute()
    if SDK.parent == work or SDK.parent in work.parents:
        parser.error("fixture directory must be outside the harness source")
    work.mkdir(parents=True)
    codex_home, workspace = work / "codex-home", work / "project"
    codex_home.mkdir()
    workspace.mkdir()
    before = {"host": fingerprint(host), "codex": fingerprint(codex)}
    (work / "binaries-before.json").write_text(json.dumps(before, indent=2) + "\n")
    commands = []

    def run(command, build=False):
        result = subprocess.run(
            command,
            cwd=workspace,
            env=dict(os.environ, PYTHONPATH=str(SDK) if build else ""),
            capture_output=True,
            text=True,
            timeout=30,
        )
        commands.append(
            {
                "command": [str(part) for part in command],
                "status": result.returncode,
                "stdout": result.stdout,
                "stderr": result.stderr,
            }
        )
        (work / "setup-commands.json").write_text(json.dumps(commands, indent=2) + "\n")
        if result.returncode:
            raise RuntimeError(result.stderr)

    management = [str(host), "--codex-home", str(codex_home)]
    for example in ("desktop", "model-fixture"):
        source, package = work / (example + "-source"), work / (example + "-package")
        shutil.copytree(SDK / "examples" / example, source)
        if example == "model-fixture":
            (source / "plugin.py").write_text(PLUGIN)
            manifest_path = source / "codex-component.json"
            manifest = json.loads(manifest_path.read_text())
            manifest["id"] = "test.gui-model"
            manifest_path.write_text(json.dumps(manifest, indent=2))
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
    run(management + ["select", "model_transport", "default", "test.gui-model"])
    (codex_home / "config.toml").write_text(f"""model = "gpt-5.1-codex"
model_provider = "gui_fixture"
approval_policy = "on-request"
sandbox_mode = "read-only"
[features]
unified_exec = true
code_mode = false
code_mode_only = false
[model_providers.gui_fixture]
name = "GUI acceptance deterministic fixture"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
requires_openai_auth = false
[projects.{json.dumps(str(workspace))}]
trust_level = "trusted"
""")
    after = {"host": fingerprint(host), "codex": fingerprint(codex)}
    if before != after:
        raise RuntimeError("host changed during fixture preparation")
    setup = {
        "ready": True,
        "gui_started": False,
        "source_removed": True,
        "inference": "deterministic test plugin",
        "cwd": str(workspace),
        "codex_home": str(codex_home),
        "binaries_before": before,
        "launch_command": management
        + ["launch", "desktop", "--codex-bin", str(codex), "--port", "0"],
        "model_state": str(codex_home / "components" / "state" / "test.gui-model"),
        "prompts": {
            "approval": "Exercise approval",
            "cancel": "Exercise cancel",
            "normal": "Verify normal response",
        },
    }
    (work / "fixture-ready.json").write_text(json.dumps(setup, indent=2) + "\n")
    print(work / "fixture-ready.json")


if __name__ == "__main__":
    main()
