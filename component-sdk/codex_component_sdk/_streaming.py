"""Strict, bounded physical framing for negotiated single-invocation messages."""

import base64
import binascii
import json
import tempfile
import threading

CHUNK_BYTES = 192 * 1024
U64_MAX = (1 << 64) - 1
SESSION = {"mode": "streaming", "version": 1}


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate protocol field")
        result[key] = value
    return result


def reject_constant(_):
    raise ValueError("nonfinite protocol value")


def strict_json(data):
    return json.loads(data, object_pairs_hook=unique_object, parse_constant=reject_constant)


def selected_contract(initialize, raw, contracts):
    if "session" not in initialize and "component" not in initialize:
        return None
    initialize = strict_json(raw)
    if set(initialize) != {"type", "api_version", "plugin_id", "config", "state_dir", "component", "session"}:
        raise ValueError("invalid streaming initialization fields")
    selection = initialize["component"]
    session = initialize["session"]
    if (
        not isinstance(selection, dict)
        or set(selection) != {"kind", "name", "contract_version"}
        or selection["kind"] != "model_transport"
        or type(selection["contract_version"]) is not int
        or selection["contract_version"] != 2
        or not isinstance(selection["name"], str)
        or not isinstance(session, dict)
        or session != SESSION
        or type(session.get("version")) is not int
        or contracts is None
        or selection not in contracts
    ):
        raise ValueError("unsupported streaming component selection")
    return selection


def _integer(value, *, positive=False):
    return type(value) is int and (1 if positive else 0) <= value <= U64_MAX


class EnvelopeCodec:
    """One request reader and serialized output; temporary bodies have no size cap."""

    def __init__(self, receive, send_frame):
        self.receive = receive
        self.send_frame = send_frame
        self.lock = threading.Lock()
        self.next_id = 1
        self.failed = False
        self.terminal_sent = False

    def request(self):
        frame = self.receive(strict=True)
        if (
            not isinstance(frame, dict)
            or set(frame) != {"type", "id", "bytes"}
            or frame["type"] != "message_start"
            or type(frame["id"]) is not int
            or frame["id"] != 1
            or not _integer(frame["bytes"], positive=True)
        ):
            raise ValueError("invalid streaming request start")
        expected = frame["bytes"]
        received = 0
        chunks = 0
        with tempfile.TemporaryFile() as body:
            while True:
                frame = self.receive(strict=True)
                if not isinstance(frame, dict):
                    raise ValueError("truncated streaming request")
                if frame.get("type") == "end":
                    if (
                        set(frame) != {"type", "id", "chunks"}
                        or type(frame["id"]) is not int or frame["id"] != 1
                        or not _integer(frame["chunks"])
                        or frame["chunks"] != chunks or received != expected
                    ):
                        raise ValueError("invalid streaming request end")
                    body.seek(0)
                    return json.load(body, parse_constant=reject_constant)
                if (
                    set(frame) != {"type", "id", "index", "data"}
                    or frame["type"] != "chunk"
                    or type(frame["id"]) is not int or frame["id"] != 1
                    or not _integer(frame["index"]) or frame["index"] != chunks
                    or not isinstance(frame["data"], str)
                    or len(frame["data"]) > CHUNK_BYTES // 3 * 4
                ):
                    raise ValueError("invalid streaming request chunk")
                try:
                    data = base64.b64decode(frame["data"], validate=True)
                except (ValueError, binascii.Error):
                    raise ValueError("invalid streaming chunk encoding") from None
                if not data or len(data) > CHUNK_BYTES or received + len(data) > expected:
                    raise ValueError("invalid streaming chunk length")
                body.write(data)
                received += len(data)
                chunks += 1
                if chunks > U64_MAX:
                    raise ValueError("streaming chunk count overflow")

    def send(self, envelope):
        with self.lock:
            if self.failed or self.terminal_sent or self.next_id > U64_MAX:
                raise ValueError("streaming output is closed")
            # Complete encoding before the start frame. A failure here may still
            # be returned as the next normal error envelope.
            with tempfile.TemporaryFile() as body:
                encoder = json.JSONEncoder(allow_nan=False, separators=(",", ":"))
                for text in encoder.iterencode(envelope):
                    # Encoder strings can be large; encode them in bounded slices.
                    for offset in range(0, len(text), 8192):
                        body.write(text[offset:offset + 8192].encode("utf-8"))
                size = body.tell()
                if not 0 < size <= U64_MAX:
                    raise ValueError("invalid streaming output size")
                body.seek(0)
                try:
                    self.send_frame({"type": "message_start", "id": self.next_id, "bytes": size})
                    index = 0
                    while data := body.read(CHUNK_BYTES):
                        self.send_frame({"type": "chunk", "id": self.next_id, "index": index, "data": base64.b64encode(data).decode("ascii")})
                        index += 1
                    self.send_frame({"type": "end", "id": self.next_id, "chunks": index})
                except BaseException:
                    # A partially emitted body cannot be replaced by an error.
                    self.failed = True
                    raise
                self.next_id += 1
                self.terminal_sent = envelope.get("type") in ("result", "error")
