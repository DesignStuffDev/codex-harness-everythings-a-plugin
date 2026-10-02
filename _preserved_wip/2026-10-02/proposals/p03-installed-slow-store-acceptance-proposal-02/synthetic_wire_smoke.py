"""Bounded synthetic relay protocol checks; no real host, install, storage or GUI."""

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time


FAKE_NATIVE = '''#!/usr/bin/env python3
import base64, json, os, time
from pathlib import Path
p = Path(__file__).resolve().parent
mode = (p / "mode").read_text()
def emit(value):
 print(json.dumps(value), flush=True)
with (p / "native-pid").open("x") as f: f.write(str(os.getpid()))
assert json.loads(input())["type"] == "initialize"
emit({"type":"ready","api_version":1,"session":{"mode":"multiplexed","version":1}})
header, body, raw = None, bytearray(), bytearray()
while True:
 line = input()
 value = json.loads(line)
 if value["type"] == "shutdown":
  break
 raw.extend((line + "\\n").encode())
 if value["type"] == "request_start": header = value
 elif value["type"] == "chunk": body.extend(base64.b64decode(value["data"], validate=True))
 elif value["type"] == "end":
  assert header is not None and len(body) == header["bytes"]
  import hashlib
  (p / "native-observed.json").write_text(json.dumps({"body_sha256":hashlib.sha256(body).hexdigest(),"wire_sha256":hashlib.sha256(raw).hexdigest()}))
  reply = json.dumps({"ok":{"append_items":None}},separators=(",",":")).encode()
  emit({"type":"result_start","id":header["id"],"bytes":len(reply)})
  emit({"type":"chunk","id":header["id"],"index":0,"data":base64.b64encode(reply).decode()})
  emit({"type":"end","id":header["id"],"chunks":1})
emit({"type":"shutdown_complete"})
os.close(1)
(p / "native-eof-ready").touch()
if mode == "late_error":
 deadline = time.monotonic() + 5
 while not (p / "native-exit-release").exists():
  assert time.monotonic() < deadline
  time.sleep(0.01)
'''


def require(value, message):
    if not value:
        raise AssertionError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wait_for(predicate, message, seconds=5):
    deadline = time.monotonic() + seconds
    while not predicate():
        require(time.monotonic() < deadline, message)
        time.sleep(0.01)


def encode(value):
    return json.dumps(value, separators=(",", ":")).encode() + b"\n"


def run_case(root, relay_source, label, mode, expected_legacy_failure=False):
    work = root / label
    work.mkdir(mode=0o700)
    gate = work / "gate"
    gate.mkdir(mode=0o700)
    relay = work / "slow_store_relay.py"
    shutil.copy2(relay_source, relay)
    native = work / "synthetic-native.py"
    native.write_text(FAKE_NATIVE)
    native.chmod(0o700)
    (work / "mode").write_text(mode)
    prompt = "synthetic-gate-target"
    (work / "relay-config.json").write_text(json.dumps({"native_entrypoint":native.name,"native_sha256":digest(native),"gate_directory":str(gate),"prompt":prompt}))
    result = {"case":label,"mode":mode,"relay_sha256":digest(relay),"synthetic_native_sha256":digest(native),"passed":False}
    process = None
    with (work / "relay.stdout").open("wb") as stdout, (work / "relay.stderr").open("wb") as stderr:
        try:
            process = subprocess.Popen([sys.executable,str(relay)],stdin=subprocess.PIPE,stdout=stdout,stderr=stderr,start_new_session=True,env=dict(os.environ,PYTHONDONTWRITEBYTECODE="1"))
            process.stdin.write(encode({"type":"initialize","api_version":1,"session":{"mode":"multiplexed","version":1},"plugin_id":"synthetic.fixture","config":{},"state_dir":str(work)}))
            process.stdin.flush()
            wait_for(lambda:(gate / "started.json").exists(),"relay did not start")
            if mode == "gated":
                payload = json.dumps({"append_items":{"thread_id":"synthetic-thread","items":[{"text":prompt}]}},separators=(",",":")).encode()
                frames = [encode({"type":"request_start","id":1,"component":{"kind":"thread_store","name":"default"},"method":"thread_store/call","is_control":False,"bytes":len(payload)}),encode({"type":"chunk","id":1,"index":0,"data":base64.b64encode(payload).decode()}),encode({"type":"end","id":1,"chunks":1})]
                process.stdin.write(b"".join(frames) + encode({"type":"shutdown"}))
                process.stdin.flush()
                wait_for(lambda:(gate / "accepted.json").exists(),"target not admitted at fixture")
                # Short synthetic hold only: this is NOT the real >45-second gate.
                time.sleep(0.2)
                require(not (gate / "forwarded.json").exists() and not (work / "native-observed.json").exists(),"native saw a held request")
                (gate / "release").touch()
            else:
                process.stdin.write(encode({"type":"shutdown"}))
                process.stdin.flush()
            if mode == "late_error":
                wait_for(lambda:(work / "native-eof-ready").exists(),"synthetic native did not close stdout")
                # The native's stdout pipe is at EOF, but its process deliberately
                # stays alive. Inject a host parser error during child.wait().
                time.sleep(0.1)
                process.stdin.write(b"malformed-late-json\n")
                process.stdin.flush()
                time.sleep(0.2)
                (work / "native-exit-release").touch()
            result["relay_status"] = process.wait(timeout=10)
            process.stdin.close()
            child_pid = int((work / "native-pid").read_text())
            require(not Path(f"/proc/{child_pid}").exists(),"exact synthetic native child remains, including zombie")
            result["exact_synthetic_native_absent"] = True
            if mode == "late_error" and not expected_legacy_failure:
                require(result["relay_status"] != 0 and not (gate / "finished.json").exists(),"late directional error falsely reported success")
                failure = json.loads((gate / "failed.json").read_text())
                require(failure["primary_error_type"] == "JSONDecodeError", "late input error was not preserved")
                require(any(x["direction"] == "host_to_native" and x["result"] == "error" for x in failure["directional_outcomes"]), "input task failure not observed")
                result["failure_receipt"] = failure
            else:
                require(result["relay_status"] == 0, "synthetic success case failed")
                finished = json.loads((gate / "finished.json").read_text())
                result["finished_receipt"] = finished
                if expected_legacy_failure:
                    result["legacy_false_success_reproduced"] = True
                else:
                    require(finished["all_directional_results_observed"],"missing task settlement receipt")
                    require(len(finished["directional_outcomes"]) == 2, "both directional outcomes were not recorded")
                    require(any(x.get("reason") == "native_eof" for x in finished["directional_outcomes"]), "pending input was not explicitly canceled on native EOF")
                if mode == "gated":
                    observed = json.loads((work / "native-observed.json").read_text())
                    require(observed == {"body_sha256":hashlib.sha256(payload).hexdigest(),"wire_sha256":hashlib.sha256(b"".join(frames)).hexdigest()},"forwarded bytes differ")
                    require(finished["held_append_acknowledged"],"native append not acknowledged")
                    result["exact_wire_forwarding"] = observed
            result["passed"] = True
        finally:
            if process is not None and process.poll() is None:
                result["emergency_cleanup"] = True
                os.killpg(process.pid,signal.SIGKILL)
                process.wait(timeout=5)
            if process is not None and process.stdin is not None:
                process.stdin.close()
            result["passed"] = result["passed"] and not result.get("emergency_cleanup",False)
            (work / "result.json").write_text(json.dumps(result,indent=2)+"\n")
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--relay",type=Path,required=True)
    parser.add_argument("--legacy-relay",type=Path,required=True)
    parser.add_argument("--work-dir",type=Path,required=True)
    args=parser.parse_args()
    root=args.work_dir.resolve()
    require(not root.exists() and not any((p/".git").exists() for p in root.parents),"fresh outside-checkout directory required")
    os.umask(0o077)
    root.mkdir(parents=True,mode=0o700)
    initial={str(p.resolve()):digest(p) for p in (Path(__file__),args.relay,args.legacy_relay)}
    report={"passed":False,"kind":"synthetic_wire_only","real_host":False,"real_storage":False,"installation":False,"browser":False,"scripts":initial,"cases":[]}
    try:
        for label,mode in (("gated-forward","gated"),("clean-native-eof","clean"),("late-input-error","late_error")):
            report["cases"].append(run_case(root,args.relay,label,mode))
        report["cases"].append(run_case(root,args.legacy_relay,"preserved-original-negative-control","late_error",True))
        require(all(digest(Path(p))==expected for p,expected in initial.items()),"source changed during smoke")
        report["passed"]=all(case["passed"] for case in report["cases"])
    finally:
        (root/"report.json").write_text(json.dumps(report,indent=2)+"\n")
    require(report["passed"],"synthetic smoke failed")
    print(root/"report.json")


if __name__=="__main__":
    main()
