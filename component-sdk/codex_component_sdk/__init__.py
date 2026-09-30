"""Process protocol v1 SDK for independently installed Codex engine components."""

import contextlib
import json
import os
import sys
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

API_VERSION = 1
MAX_FRAME_BYTES = 4 * 1024 * 1024


class PluginError(Exception):
    """An intentional, public error. Never include credentials or private data."""


@dataclass(frozen=True)
class Initialization:
    """Invocation configuration and the host-owned durable state directory."""

    plugin_id: str
    config: dict[str, Any]
    state_dir: Path


@dataclass(frozen=True)
class RequestContext:
    """Per-request state and an event emitter that writes immediately to the host."""

    initialization: Initialization
    emit: Callable[[Any], None]
    watch_shutdown: Callable[[], threading.Event]


class Plugin:
    """Register component handlers and serve one bounded NDJSON request per process.

    Long-lived handlers can watch_shutdown() and finish work before returning.
    Forced cancellation can still terminate the child without running hooks.
    """

    def __init__(self):
        self._handlers = {}
        self._start = None
        self._shutdown = None

    def handle(self, kind: str, name: str, method: str):
        """Register a function accepting (JSON params, RequestContext)."""
        key = (kind, name, method)

        def register(handler):
            if key in self._handlers:
                raise ValueError(f"duplicate component handler: {key}")
            self._handlers[key] = handler
            return handler

        return register

    def on_start(self, callback):
        """Register a startup hook accepting Initialization; called before ready."""
        self._start = callback
        return callback

    def on_shutdown(self, callback):
        """Register a cleanup hook accepting Initialization; cancellation may skip it."""
        self._shutdown = callback
        return callback

    def run(self):
        """Serve stdin/stdout. Protocol failures exit 2 without reflecting input."""
        output = sys.stdout.buffer
        initialization = None
        input_buffer = bytearray()

        def send(frame):
            data = json.dumps(frame, allow_nan=False, separators=(",", ":")).encode(
                "utf-8"
            )
            if len(data) + 1 > MAX_FRAME_BYTES:
                raise PluginError("response exceeds protocol frame limit")
            output.write(data + b"\n")
            output.flush()

        def receive():
            # One reader owns stdin at a time. Keep buffering here, so the
            # optional control thread can exit with the process without holding
            # Python's buffered-stdin lock during interpreter shutdown.
            while b"\n" not in input_buffer and len(input_buffer) <= MAX_FRAME_BYTES:
                chunk = os.read(sys.stdin.fileno(), 65536)
                if not chunk:
                    break
                input_buffer.extend(chunk)
            newline = input_buffer.find(b"\n")
            end = newline + 1 if newline >= 0 else len(input_buffer)
            data = bytes(input_buffer[:end])
            del input_buffer[:end]
            if not data:
                return None
            if len(data) > MAX_FRAME_BYTES or not data.endswith(b"\n"):
                raise ValueError("invalid frame size")
            frame = json.loads(
                data, parse_constant=lambda _: (_ for _ in ()).throw(ValueError())
            )
            if not isinstance(frame, dict):
                raise ValueError("invalid frame")
            return frame

        shutdown_requested = threading.Event()
        control_thread = None
        control_error = None

        def read_shutdown():
            nonlocal control_error
            try:
                frame = receive()
                if frame is not None and frame.get("type") != "shutdown":
                    raise ValueError("expected shutdown")
            except Exception as error:
                control_error = error
            finally:
                shutdown_requested.set()

        def watch_shutdown():
            """Opt into terminal control while a long-lived handler is active.

            The event means shutdown, EOF or invalid control; the SDK rejects
            invalid control before acknowledging a successful handler result.
            Handlers must stop admitting work and complete cleanup on this event.
            """
            nonlocal control_thread
            if control_thread is None:
                control_thread = threading.Thread(target=read_shutdown, daemon=True)
                control_thread.start()
            return shutdown_requested

        try:
            frame = receive()
            if (
                frame is None
                or frame.get("type") != "initialize"
                or type(frame.get("api_version")) is not int
                or frame["api_version"] != API_VERSION
                or not isinstance(frame.get("plugin_id"), str)
                or not isinstance(frame.get("config"), dict)
                or not isinstance(frame.get("state_dir"), str)
                or not Path(frame["state_dir"]).is_absolute()
            ):
                raise ValueError("invalid initialization")
            initialization = Initialization(
                frame["plugin_id"], frame["config"], Path(frame["state_dir"])
            )
            with contextlib.redirect_stdout(sys.stderr):
                if self._start:
                    self._start(initialization)
            send({"type": "ready", "api_version": API_VERSION})
            frame = receive()
            if frame is None or frame.get("type") == "shutdown":
                return
            component = frame.get("component")
            if (
                frame.get("type") != "request"
                or type(frame.get("id")) is not int
                or not isinstance(component, dict)
                or not all(
                    isinstance(component.get(key), str) for key in ("kind", "name")
                )
                or not isinstance(frame.get("method"), str)
            ):
                raise ValueError("invalid request")
            request_id = frame["id"]
            try:
                handler = self._handlers.get(
                    (component["kind"], component["name"], frame["method"])
                )
                if handler is None:
                    raise PluginError("unsupported component or method")
                context = RequestContext(
                    initialization,
                    lambda event: send(
                        {"type": "event", "id": request_id, "event": event}
                    ),
                    watch_shutdown,
                )
                with contextlib.redirect_stdout(sys.stderr):
                    result = handler(frame.get("params"), context)
                if control_error is not None:
                    raise PluginError("invalid shutdown control frame")
                send({"type": "result", "id": request_id, "result": result})
            except Exception as error:
                message = (
                    str(error).encode("utf-8")[:128].decode("utf-8", errors="ignore")
                    if isinstance(error, PluginError)
                    else "component handler failed"
                )
                print(
                    f"component handler exception: {type(error).__name__}",
                    file=sys.stderr,
                )
                send({"type": "error", "id": request_id, "message": message})
            if control_thread is None:
                read_shutdown()
            else:
                control_thread.join()
            if control_error is not None:
                raise ValueError("invalid shutdown control frame")
        except Exception as error:
            print(
                f"component protocol failure: {type(error).__name__}", file=sys.stderr
            )
            raise SystemExit(2) from None
        finally:
            if initialization and self._shutdown:
                try:
                    with contextlib.redirect_stdout(sys.stderr):
                        self._shutdown(initialization)
                except Exception as error:
                    print(
                        f"component cleanup failure: {type(error).__name__}",
                        file=sys.stderr,
                    )
