"""Model-v2 error values and public HTTP metadata; no backend policy inference."""

import base64
import binascii
import json
import math
import re
from collections.abc import Mapping
from dataclasses import dataclass
from urllib.parse import urlsplit, urlunsplit


def _invalid():
    raise ValueError("invalid model value")


def _clone_json(value):
    """Detach pure JSON without coercing dictionary keys or non-JSON values."""
    def check(item):
        if type(item) is dict:
            for key, child in item.items():
                if type(key) is not str:
                    _invalid()
                check(child)
        elif type(item) is list:
            for child in item:
                check(child)
        elif item is not None and type(item) not in (str, bool, int, float):
            _invalid()

    try:
        check(value)
        encoded = json.dumps(value, ensure_ascii=False, allow_nan=False)
        return json.loads(encoded.encode("utf-8"))
    except (TypeError, ValueError, OverflowError, RecursionError):
        raise ValueError("invalid model JSON") from None


def _string(value):
    if type(value) is not str:
        _invalid()


def _boolean(value):
    if type(value) is not bool:
        _invalid()


def _integer(value, minimum, maximum):
    if type(value) is not int or not minimum <= value <= maximum:
        _invalid()


def _i64(value):
    _integer(value, -(1 << 63), (1 << 63) - 1)


def _u64(value):
    _integer(value, 0, (1 << 64) - 1)


def _number(value):
    if type(value) not in (int, float) or (type(value) is float and not math.isfinite(value)):
        _invalid()
    # serde_json::Number represents integer values as signed/unsigned 64-bit numbers.
    if type(value) is int:
        _integer(value, -(1 << 63), (1 << 64) - 1)


def _status(value):
    _integer(value, 100, 999)


def _optional(validate):
    def optional(value):
        if value is not None:
            validate(value)
    return optional


def _enum(value, choices):
    if type(value) is not str or value not in choices:
        _invalid()


def _shape(value, required, optional=None, *, strict=True):
    optional = optional or {}
    if type(value) is not dict or not required.keys() <= value.keys():
        _invalid()
    if strict and value.keys() - required.keys() - optional.keys():
        _invalid()
    for key, validate in (required | optional).items():
        if key in value:
            validate(value[key])


def _deadline(value):
    _shape(value, {"unix_seconds": _u64, "subsec_nanos": lambda x: _integer(x, 0, 999_999_999)})


@dataclass(frozen=True)
class RetryDeadline:
    """Absolute same-machine Unix deadline; do not recreate it after a transport wait."""

    unix_seconds: int
    subsec_nanos: int = 0

    def __post_init__(self):
        _deadline(self.to_dict())

    def to_dict(self):
        return {"unix_seconds": self.unix_seconds, "subsec_nanos": self.subsec_nanos}


_HEADER_BASE = {
    "x-request-id", "x-oai-request-id", "cf-ray", "x-openai-authorization-error",
    "x-error-json", "x-codex-active-limit", "x-codex-promo-message",
    "x-codex-rate-limit-reached-type", "x-codex-credits-has-credits",
    "x-codex-credits-unlimited", "x-codex-credits-balance",
}
_LOWER_ASCII = str.maketrans("ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz")
_HEADER_NAME = re.compile(r"[!#$%&'*+.^_`|~0-9a-z-]+\Z")


def _header_value(value):
    return type(value) is str and all(char == "\t" or 32 <= ord(char) <= 126 for char in value)


def _header_names(headers):
    active = headers.get("x-codex-active-limit", "").strip() or "codex"
    prefix = "x-" + active.translate(_LOWER_ASCII).replace("_", "-")
    return _HEADER_BASE | {prefix + "-limit-name"} | {
        f"{prefix}-{window}-{field}"
        for window in ("primary", "secondary")
        for field in ("used-percent", "window-minutes", "reset-at")
    }


def _project_error_json(value):
    try:
        raw = base64.b64decode(value, validate=True)
        if base64.b64encode(raw).decode("ascii") != value:
            return None
        decoded = _clone_json(json.loads(raw.decode("utf-8"), parse_constant=lambda _: _invalid()))
        code = decoded["error"]["code"]
        if type(code) is not str:
            return None
        minimal = json.dumps({"error": {"code": code}}, ensure_ascii=False, separators=(",", ":"))
        return base64.b64encode(minimal.encode("utf-8")).decode("ascii")
    except (binascii.Error, KeyError, TypeError, ValueError, UnicodeError, RecursionError):
        return None


def project_http_headers(headers):
    """Keep first values for consumed public metadata; preserve None versus an empty map."""
    if headers is None:
        return None
    pairs = headers.items() if isinstance(headers, Mapping) else headers
    first = {}
    try:
        for name, value in pairs:
            if type(name) is not str:
                continue
            name = name.translate(_LOWER_ASCII)
            if name in first or not _HEADER_NAME.fullmatch(name):
                continue
            # Remember invalid first values too: native HeaderMap.get never selects a later value.
            if type(value) is bytes:
                try:
                    value = value.decode("ascii")
                except UnicodeError:
                    value = None
            first[name] = value
    except (TypeError, ValueError):
        raise ValueError("invalid HTTP metadata") from None
    visible = {name: value for name, value in first.items() if _header_value(value)}
    allowed = _header_names(visible)
    result = {name: value for name, value in visible.items() if name in allowed}
    if "x-error-json" in result:
        projected = _project_error_json(result["x-error-json"])
        if projected is None:
            del result["x-error-json"]
        else:
            result["x-error-json"] = projected
    return result


def _headers(value):
    if type(value) is not dict:
        _invalid()
    if any(type(name) is not str or not _HEADER_NAME.fullmatch(name) or not _header_value(item)
           for name, item in value.items()):
        _invalid()
    if value.keys() - _header_names(value):
        _invalid()


def _url_parts(value):
    _string(value)
    if any(ord(char) < 32 or ord(char) == 127 for char in value):
        _invalid()
    try:
        parts = urlsplit(value)
        if not re.fullmatch(r"[a-zA-Z][a-zA-Z0-9+.-]*", parts.scheme):
            _invalid()
        if parts.scheme in ("http", "https", "ftp", "ws", "wss") and not parts.hostname:
            _invalid()
        parts.port  # Validate numeric port syntax/range; port zero is valid.
        return parts
    except ValueError:
        raise ValueError("invalid public URL") from None


def redact_url(value):
    """Remove user-info, query and fragment from an HTTP diagnostic URL, or omit invalid URLs."""
    if value is None:
        return None
    try:
        parts = _url_parts(value)
        path = parts.path or ("/" if parts.netloc else "")
        return urlunsplit((parts.scheme, parts.netloc.rsplit("@", 1)[-1], path, "", ""))
    except ValueError:
        return None


def _public_url(value):
    parts = _url_parts(value)
    if "@" in parts.netloc or "?" in value or "#" in value:
        _invalid()


_ROUTE_CLASSES = {
    "proxy_resolution_unavailable", "connect_timeout", "proxy_authentication_required",
    "tls_error", "invalid_proxy_config", "unsupported_proxy_scheme", "resolver_error",
}
_CONNECTION_FLAGS = ("is_timeout", "is_connect", "is_builder", "is_body", "is_request")


def _connection(value):
    _shape(value, {"message": _string} | {flag: _boolean for flag in _CONNECTION_FLAGS}, {
        "status": _optional(_status),
        "failure_class": _optional(lambda x: _enum(x, _ROUTE_CLASSES)),
    })
    message = "connection failed"
    if value.get("failure_class") is not None:
        name = value["failure_class"]
        message += " (" + ("proxy_407" if name == "proxy_authentication_required" else name) + ")"
    if value.get("status") is not None:
        message += f": HTTP {value['status']}"
    value["message"] = message


def _misalignment(value):
    _shape(value, {}, {
        "error_type": _optional(_string), "detailed_explanation": _optional(_string),
        "steer": _optional(lambda x: _shape(x, {"message": _string}, strict=False)),
    }, strict=False)


def _transport(value):
    if type(value) is not dict:
        _invalid()
    kind = value.get("kind")
    _string(kind)
    required, optional = {}, {}
    if kind == "policy":
        required = {"reason": lambda x: _enum(x, {"unavailable", "destination", "revoked", "unsupported_transport"})}
    elif kind == "http":
        required = {"status": _status}
        optional = {"url": _optional(_public_url), "headers": _optional(_headers),
                    "body": _optional(_string), "retry_at": _optional(_deadline)}
    elif kind == "connection":
        required = {"connection": _connection}
    elif kind in ("network", "build"):
        required = {"message": _string}
    elif kind == "response_too_large":
        required = {"max_bytes": _u64}
    elif kind not in ("retry_limit", "timeout"):
        _invalid()
    _shape(value, {"kind": _string} | required, optional)
    if kind == "http" and value.get("headers") is not None:
        value["headers"] = project_http_headers(value["headers"])


def validate_backend_error(value):
    """Validate the exact error envelope and detach it without formatting private values."""
    value = _clone_json(value)
    if type(value) is not dict:
        _invalid()
    kind = value.get("kind")
    _string(kind)
    required, optional = {}, {}
    if kind == "api":
        required = {"status": _status, "message": _string}
    elif kind in {"stream", "rate_limit", "invalid_request", "invalid_prompt", "cyber_policy", "bio_policy"}:
        required = {"message": _string}
    elif kind in ("retryable", "rate_limit_exceeded"):
        required, optional = {"message": _string}, {"retry_at": _optional(_deadline)}
    elif kind == "server_overloaded":
        optional = {"retry_at": _optional(_deadline)}
    elif kind == "misalignment_policy_violation":
        required, optional = {"message": _string}, {"misalignment": _optional(_misalignment)}
    elif kind == "transport":
        required = {"transport": _transport}
    elif kind not in {"content_filter", "context_window_exceeded", "quota_exceeded", "usage_not_included", "flex_unavailable"}:
        _invalid()
    _shape(value, {"kind": _string} | required, optional)
    return value


class BackendError(Exception):
    """An explicit typed backend outcome; repr/str never contain its diagnostic DTO."""

    def __init__(self, payload):
        self._payload = validate_backend_error(payload)
        super().__init__("model backend error")

    def to_dict(self):
        return validate_backend_error(self._payload)

    def __repr__(self):
        return "BackendError(<redacted>)"
