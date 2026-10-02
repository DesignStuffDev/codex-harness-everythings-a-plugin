#!/usr/bin/env python3
"""Bounded acceptance-only storage relay; never a product implementation.

Accept one complete append at this installed fixture boundary, retain it before
native forwarding, then deliver exact original frames after a private release.
This does NOT prove admission inside native StorageService before release.
"""

import asyncio
import base64
import ctypes
import hashlib
import json
import os
from pathlib import Path
import sys
import signal
import time


FRAME = 4 * 1024 * 1024
BODY = 4 * 1024 * 1024
BATCH = 16 * 1024 * 1024
MAX_PENDING = 64


def require(value, message):
    if not value:
        raise RuntimeError(message)


def receipt(directory, name, **fields):
    value = dict(event=name, monotonic_ns=time.monotonic_ns(), relay_pid=os.getpid(), **fields)
    target = directory / (name + ".json")
    temporary = directory / (name + ".tmp")
    with temporary.open("x") as stream:
        json.dump(value, stream, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    temporary.replace(target)


class Assembly:
    def __init__(self):
        self.pending = {}

    def feed(self, value):
        kind = value["type"]
        if kind in ("request_start", "result_start"):
            ident = value["id"]
            require(ident not in self.pending, "duplicate pending ID")
            require(len(self.pending) < MAX_PENDING, "too many pending messages")
            require(0 <= value["bytes"] <= BODY, "fixture logical payload limit")
            self.pending[ident] = (value, bytearray(), 0)
        elif kind == "chunk":
            header, body, chunks = self.pending[value["id"]]
            require(value["index"] == chunks, "noncontiguous chunk")
            decoded = base64.b64decode(value["data"], validate=True)
            require(0 < len(decoded) <= 192 * 1024, "invalid chunk length")
            body.extend(decoded)
            require(len(body) <= header["bytes"], "payload exceeds declaration")
            require(sum(len(item[1]) for item in self.pending.values()) <= BATCH, "fixture aggregate decoded payload limit")
            self.pending[value["id"]] = (header, body, chunks + 1)
        elif kind == "end":
            header, body, chunks = self.pending.pop(value["id"])
            require(chunks == value["chunks"] and len(body) == header["bytes"], "incomplete payload")
            return header, json.loads(body), hashlib.sha256(body).hexdigest()
        return None


async def line(reader):
    value = await reader.readline()
    require(not value or (len(value) <= FRAME and value.endswith(b"\n")), "invalid frame length")
    return value


async def run(config):
    directory = Path(config["gate_directory"])
    require(directory.is_absolute() and directory.is_dir(), "invalid gate directory")
    package = Path(__file__).resolve().parent
    native = package / config["native_entrypoint"]
    require(native.is_file() and native.parent == package, "invalid native entrypoint")
    digest = hashlib.sha256()
    with native.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    require(digest.hexdigest() == config["native_sha256"], "native hash changed")
    loop = asyncio.get_running_loop()
    incoming = asyncio.StreamReader(limit=FRAME)
    protocol = asyncio.StreamReaderProtocol(incoming)
    transport, _ = await loop.connect_read_pipe(lambda: protocol, sys.stdin.buffer)
    child = None
    tasks = []
    selected = None
    acked = False
    shutdown_complete = False
    # A cold restart is pass-through; it cannot silently start another gate.
    used = (directory / "accepted.json").exists()
    parent = os.getpid()
    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
    libc.prctl.restype = ctypes.c_int

    def parent_death():
        # This Linux fixture creates its only child before starting any Python
        # worker threads. Keep native in our group and bind it to our lifetime.
        if libc.prctl(1, signal.SIGKILL, 0, 0, 0) != 0 or os.getppid() != parent:
            os._exit(126)

    try:
        child = await asyncio.create_subprocess_exec(
            str(native), stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE,
            stderr=sys.stderr, limit=FRAME, preexec_fn=parent_death,
        )
        require(os.getpgid(child.pid) == os.getpgrp(), "native did not inherit fixture process group")
        receipt(directory, "started", native_pid=child.pid, process_group=os.getpgrp(), native_sha256=config["native_sha256"])

        async def host_to_native():
            nonlocal used, selected
            assembler = Assembly()
            buffered, buffered_bytes = [], 0
            while raw := await line(incoming):
                value = json.loads(raw)
                if used or value["type"] == "initialize":
                    child.stdin.write(raw)
                    await child.stdin.drain()
                    continue
                # Buffer only the current complete wire batch. Once a target is
                # admitted, stop reading; OS pipe backpressure bounds later work.
                buffered.append(raw)
                buffered_bytes += len(raw)
                require(buffered_bytes <= BATCH, "fixture wire batch limit")
                complete = assembler.feed(value)
                if complete:
                    header, body, digest = complete
                    append = body.get("append_items") if isinstance(body, dict) else None
                    if (header.get("component", {}).get("kind") == "thread_store"
                            and header.get("method") == "thread_store/call"
                            and not header.get("is_control") and isinstance(append, dict)
                            and config["prompt"] in json.dumps(append, ensure_ascii=False)):
                        selected = header["id"]
                        used = True
                        receipt(directory, "accepted", request_id=selected,
                                thread_id=append.get("thread_id"), payload_sha256=digest,
                                logical_bytes=header["bytes"], batch_bytes=buffered_bytes,
                                admission_boundary="installed_fixture_before_native_forward")
                        deadline = time.monotonic() + 95
                        while not (directory / "release").exists():
                            require(time.monotonic() < deadline, "fixture gate deadline expired")
                            await asyncio.sleep(0.025)
                        receipt(directory, "release_observed", request_id=selected)
                        for frame in buffered:
                            child.stdin.write(frame)
                            await child.stdin.drain()
                        receipt(directory, "forwarded", request_id=selected)
                        buffered.clear()
                        buffered_bytes = 0
                if not used and not assembler.pending:
                    for frame in buffered:
                        child.stdin.write(frame)
                        await child.stdin.drain()
                    buffered.clear()
                    buffered_bytes = 0
            require(not buffered and (used or not assembler.pending), "truncated host request")
            child.stdin.close()
            await child.stdin.wait_closed()

        async def native_to_host():
            nonlocal acked, shutdown_complete
            assembler = Assembly()
            while raw := await line(child.stdout):
                value = json.loads(raw)
                complete = assembler.feed(value)
                if complete and complete[0]["id"] == selected:
                    require(complete[1] == {"ok": {"append_items": None}}, "held native append did not succeed")
                    acked = True
                    receipt(directory, "native_append_reply", request_id=selected)
                if value["type"] == "error" and value.get("id") == selected:
                    raise RuntimeError("held native append transport error")
                if value["type"] == "shutdown_complete":
                    shutdown_complete = True
                    receipt(directory, "native_shutdown_complete", held_append_acknowledged=acked)
                sys.stdout.buffer.write(raw)
                sys.stdout.buffer.flush()
            require(not assembler.pending, "truncated native response")

        tasks = [asyncio.create_task(host_to_native()), asyncio.create_task(native_to_host())]
        completed, _ = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
        for task in completed:
            task.result()
        # A native EOF is terminal. Host stdin may remain open while the host
        # waits for child exit; keep its exact task until explicit cancellation.
        if tasks[1] not in completed:
            await tasks[1]
        status = await asyncio.wait_for(child.wait(), timeout=5)
        require(status == 0 and shutdown_complete, "native cleanup not confirmed")
        if selected is not None:
            require(acked, "accepted append lacks native acknowledgement")
        receipt(directory, "finished", native_status=status, native_shutdown_complete=shutdown_complete, held_append_acknowledged=acked)
    finally:
        transport.close()
        for task in tasks:
            if not task.done():
                task.cancel()
        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)
        if child is not None and child.returncode is None:
            child.kill()
            await child.wait()
        if child is not None:
            child.stdin.close()
            try:
                await asyncio.wait_for(child.stdin.wait_closed(), timeout=2)
            except (BrokenPipeError, ConnectionResetError):
                pass


def main():
    os.umask(0o077)
    config = json.loads(Path(__file__).with_name("relay-config.json").read_text())
    try:
        asyncio.run(asyncio.wait_for(run(config), timeout=300))
    except BaseException:
        # Raw configuration, payloads, provider errors and inherited environment
        # are never printed. Full operation outcomes are intentionally unknown.
        print("Slow-storage acceptance relay failed; accepted outcomes are unknown", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
