"""Typed model-v2 helpers; generic process framing and native provider decoding stay separate.

Use ``@model_handler`` below ``@app.handle("model_transport", "default", "model.stream")``.
Emit one terminal ``completed`` or ``backend_error`` event and return exactly ``{}``.
The generic runtime still owns shutdown validation, result delivery and process cleanup.
"""

import threading
from dataclasses import replace
from functools import wraps

from .model_errors import BackendError, RetryDeadline
from .model_errors import project_http_headers, redact_url, validate_backend_error
from .model_errors import (
    _boolean, _clone_json, _enum, _i64, _integer, _invalid, _number, _optional,
    _shape, _string,
)

__all__ = [
    "BackendError", "RetryDeadline", "model_handler", "validate_event",
    "validate_backend_error", "project_http_headers", "redact_url",
]


def _strings(value):
    if type(value) is not list:
        _invalid()
    for item in value:
        _string(item)


def _item(value):
    # Deliberately not a Python reimplementation of ResponseItem's provider decoder.
    # The Rust consumer owns known/Other variants and strips forged execution provenance.
    _shape(value, {"type": _string}, strict=False)


def _usage(value):
    _shape(value, {name: _i64 for name in (
        "input_tokens", "cached_input_tokens", "output_tokens",
        "reasoning_output_tokens", "total_tokens",
    )}, {"cache_write_input_tokens": _i64, "codex_rollout_budget_units": _optional(_number)})
    value.setdefault("cache_write_input_tokens", 0)


def _buffering(value):
    _shape(value, {"use_cases": _strings, "reasons": _strings, "show_buffering_ui": _boolean},
           {"retry_model": _optional(_string)})


def _usage_metadata(value):
    _shape(value, {}, {"amount": _optional(_string)}, strict=False)


def _moderation(value):
    _shape(value, {"metadata": lambda _: None}, strict=False)


def _verifications(value):
    _strings(value)
    for item in value:
        _enum(item, {"trusted_access_for_cyber"})


def _rate_window(value):
    _shape(value, {"used_percent": _number}, {
        "window_minutes": _optional(_i64), "resets_at": _optional(_i64),
    }, strict=False)


def _credits(value):
    _shape(value, {"has_credits": _boolean, "unlimited": _boolean},
           {"balance": _optional(_string)}, strict=False)


def _spend(value):
    _shape(value, {"limit": _string, "used": _string, "resets_at": _i64,
                   "remaining_percent": lambda x: _integer(x, -(1 << 31), (1 << 31) - 1)}, strict=False)


def _rate_limits(value):
    _shape(value, {}, {
        "limit_id": _optional(_string), "limit_name": _optional(_string),
        "normal_model_slug": _optional(_string), "primary": _optional(_rate_window),
        "secondary": _optional(_rate_window), "credits": _optional(_credits),
        "individual_limit": _optional(_spend), "spend_control_reached": _optional(_boolean),
        "plan_type": _optional(_string), "rate_limit_reached_type": _optional(_string),
    }, strict=False)


_EVENT_FIELDS = {
    "created": ({}, {"response_id": _optional(_string)}),
    "output_item_added": ({"item": _item}, {}),
    "output_item_done": ({"item": _item}, {}),
    "output_text_delta": ({"delta": _string}, {}),
    "tool_call_input_delta": ({"item_id": _string, "delta": _string}, {"call_id": _optional(_string)}),
    "completed": ({"response_id": _string}, {
        "token_usage": _optional(_usage), "usage_metadata": _optional(_usage_metadata),
        "end_turn": _optional(_boolean),
    }),
    "reasoning_summary_delta": ({"delta": _string, "summary_index": _i64}, {}),
    "reasoning_summary_done": ({"item_id": _string, "text": _string, "summary_index": _i64}, {}),
    "reasoning_content_delta": ({"delta": _string, "content_index": _i64}, {}),
    "reasoning_summary_part_added": ({"summary_index": _i64}, {}),
    "server_model": ({"model": _string}, {}),
    "model_verifications": ({"verifications": _verifications}, {}),
    "turn_moderation_metadata": ({"metadata": _moderation}, {}),
    "server_reasoning_included": ({"included": _boolean}, {}),
    "rate_limits": ({"rate_limits": _rate_limits}, {}),
    "models_etag": ({"etag": _string}, {}),
    "safety_buffering": ({"buffering": _buffering}, {}),
    "backend_error": ({"error": lambda _: None}, {}),
}


def validate_event(value):
    """Validate strict model fields and structural payloads; return detached JSON.

    Nested provider/protocol payloads retain native compatibility. This helper does
    not claim to sanitize provider execution provenance; the native decoder owns it.
    """
    value = _clone_json(value)
    if type(value) is not dict or type(value.get("type")) is not str:
        _invalid()
    spec = _EVENT_FIELDS.get(value["type"])
    if spec is None:
        _invalid()
    required, optional = spec
    _shape(value, {"type": _string} | required, optional)
    if value["type"] == "backend_error":
        value["error"] = validate_backend_error(value["error"])
    return value


def model_handler(handler):
    """Wrap a synchronous handler with serialized, strict terminal model discipline.

    Only explicit BackendError exceptions become typed backend events. Other exceptions
    propagate into the generic runtime's sanitized invocation error. Handlers must join
    their worker threads before returning; this wrapper cannot own arbitrary plugin work.
    """
    @wraps(handler)
    def invoke(params, context):
        _shape(params, {"thread_id": _string, "request": lambda x: _shape(x, {}, strict=False)})
        lock = threading.Lock()
        terminal = closed = failed = False

        def emit(event):
            nonlocal terminal, failed
            with lock:
                if terminal or closed or failed:
                    failed = True
                    raise ValueError("invalid model event lifecycle")
                try:
                    event = validate_event(event)
                    context.emit(event)
                    terminal = event["type"] in ("completed", "backend_error")
                except BaseException:
                    failed = True
                    raise

        try:
            try:
                result = handler(params, replace(context, emit=emit))
            except BackendError as error:
                emit({"type": "backend_error", "error": error.to_dict()})
                result = {}
            with lock:
                closed = True
                if failed or not terminal or type(result) is not dict or result:
                    raise ValueError("invalid model terminal result")
            return {}
        finally:
            with lock:
                closed = True

    return invoke
