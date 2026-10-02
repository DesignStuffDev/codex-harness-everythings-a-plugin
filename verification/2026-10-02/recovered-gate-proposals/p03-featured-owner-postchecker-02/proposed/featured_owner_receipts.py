#!/usr/bin/env python3
"""Additive featured-task receipt capture/checking; never whole-host proof."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

TARGET = "codex_core_plugins::lifecycle"
PROCESS_EVENT = "featured_warmup_process_shutdown_observed"
SCOPE_EVENT = "featured_warmup_scope_shutdown_observed"
PATHS = ("exec-success", "exec-error", "app-server-eof", "app-server-sigterm")
CASE_IDS = tuple(mode + "-" + path for mode in ("git", "fallback") for path in PATHS)
MAX_INPUT_BYTES = 16 * 1024 * 1024
MAX_LINE_BYTES = 1024 * 1024
MAX_RECORDS = 65536
MAX_EVENTS = 128
MAX_ERRORS = 16
MAX_JSON_BYTES = 4 * 1024 * 1024
PROCESS_FIELDS = {
    "receipt_version": int, "task_pending": int, "task_joined": int,
    "task_panicked": int, "task_join_failed": int,
    "optional_fetch_failed": int, "task_ownership_clean": bool,
}
SCOPE_FIELDS = {
    "receipt_version": int, "task_ownership": str,
    "optional_fetch_outcome": str, "task_ownership_clean": bool,
}
ERROR_CODES = frozenset({
    "input_type", "input_byte_limit", "line_byte_limit", "record_limit",
    "event_limit", "malformed_utf8", "malformed_json", "duplicate_json_key",
    "malformed_matching_event", "duplicate_fmt_key", "matching_field_missing",
    "matching_field_type", "receipt_version", "process_unclean", "scope_unclean",
})
SOURCE_KEYS = frozenset({"fixture_sha256", "collector_sha256", "binary_sha256", "source_commit", "source_tree"})


class ReceiptError(Exception):
    """Safe fixed error code, with no source line or runtime values."""


def need(condition, code):
    if not condition:
        raise ReceiptError(code)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        need(key not in result, "duplicate_json_key")
        result[key] = value
    return result


def json_value(data):
    try:
        return json.loads(data, object_pairs_hook=unique_object)
    except (ValueError, UnicodeDecodeError, RecursionError):
        raise ReceiptError("malformed_json") from None


def clean_row(kind, row):
    fields = PROCESS_FIELDS if kind == "process" else SCOPE_FIELDS
    need(type(row) is dict and set(row) == set(fields), "matching_field_missing")
    for key, expected in fields.items():
        need(type(row[key]) is expected, "matching_field_type")
        if expected is int:
            need(row[key] >= 0, "matching_field_type")
    need(row["receipt_version"] == 1, "receipt_version")
    if kind == "process":
        need(row["task_pending"] == row["task_panicked"] == row["task_join_failed"] == 0
             and row["task_ownership_clean"] is True, "process_unclean")
        # Retained-scope snapshots may have joined=0; optional failures can be clean.
    else:
        allowed = ((row["task_ownership"] == "Idle" and row["optional_fetch_outcome"] == "None")
                   or (row["task_ownership"] == "Joined" and row["optional_fetch_outcome"] in
                       ("Some(Succeeded)", "Some(Failed)", "Some(Cancelled)")))
        need(allowed and row["task_ownership_clean"] is True, "scope_unclean")


def parse_line(line):
    """Parse ordinary JSON/fmt tracing; return only allowlisted typed fields."""
    try:
        text = re.sub(r"\x1b\[[0-9;]*m", "", line.decode("utf-8"))
    except UnicodeDecodeError:
        raise ReceiptError("malformed_utf8") from None
    # Decode Unicode escapes only for conservative candidate detection. Strict JSON
    # parsing still owns validation; duplicate keys must not hide an escaped event.
    candidate_text = re.sub(r"\\u([0-9a-fA-F]{4})", lambda match: chr(int(match.group(1), 16)), text)
    candidate = PROCESS_EVENT in candidate_text or SCOPE_EVENT in candidate_text
    is_fmt = not text.lstrip().startswith("{")
    if not is_fmt:
        try:
            record = json_value(text)
        except ReceiptError:
            if candidate or TARGET in candidate_text:
                raise
            return None  # Unrelated JSON-looking CLI text is not a featured event.
        if type(record) is not dict or record.get("target") != TARGET:
            need(not candidate, "malformed_matching_event")
            return None
        fields = record.get("fields")
        need(type(fields) is dict, "malformed_matching_event")
    else:
        if not re.search(r"(?<![A-Za-z0-9_:])" + re.escape(TARGET) + r"(?=:|\s)", text):
            need(not candidate, "malformed_matching_event")
            return None
        markers = list(re.finditer(r'(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=', text))
        matches = list(re.finditer(
            r'(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=(?:"([^"\n]*)"|((?!")[^\s]+))(?=\s|$)', text))
        pairs = [(m.group(1), m.group(2) if m.group(2) is not None else m.group(3)) for m in matches]
        fields = dict(pairs)
        if candidate or fields.get("event") in (PROCESS_EVENT, SCOPE_EVENT):
            need([match.start() for match in matches] == [marker.start() for marker in markers],
                 "malformed_matching_event")
            need(len(fields) == len(pairs), "duplicate_fmt_key")
    event = fields.get("event")
    if event not in (PROCESS_EVENT, SCOPE_EVENT):
        need(not candidate, "malformed_matching_event")
        return None
    kind = "process" if event == PROCESS_EVENT else "scope"
    wanted = PROCESS_FIELDS if kind == "process" else SCOPE_FIELDS
    row = {}
    for key, expected in wanted.items():
        need(key in fields, "matching_field_missing")
        value = fields[key]
        if is_fmt and expected is int and type(value) is str and re.fullmatch(r"[0-9]{1,20}", value):
            value = int(value)
        elif is_fmt and expected is bool and value in ("true", "false"):
            value = value == "true"
        need(type(value) is expected, "matching_field_type")
        row[key] = value
    clean_row(kind, row)
    return kind, row


class FeaturedCapture:
    """Called synchronously by the original stderr drain; stores no raw logs."""

    def __init__(self):
        self.input_bytes = self.input_records = 0
        self.events, self.errors = [], []
        self.stopped = False

    def consume(self, line):
        # Fail closed once any event/input is invalid. The original drain still runs.
        if self.stopped:
            return
        try:
            need(type(line) is bytes, "input_type")
            self.input_bytes += len(line)  # Delimiters are omitted by original PipeDrain.
            self.input_records += 1
            need(self.input_bytes <= MAX_INPUT_BYTES, "input_byte_limit")
            need(len(line) <= MAX_LINE_BYTES, "line_byte_limit")
            need(self.input_records <= MAX_RECORDS, "record_limit")
            found = parse_line(line)
            if found is not None:
                need(len(self.events) < MAX_EVENTS, "event_limit")
                kind, fields = found
                self.events.append({"kind": kind, "record_index": self.input_records, "fields": fields})
        except ReceiptError as error:
            self.errors.append(str(error))
            self.stopped = True

    def report(self, case_id, stderr, source_bindings):
        # Do not copy arbitrary report/log values. Only fixed safe fields are retained.
        binding = {key: stderr.get(key) for key in ("bytes", "sha256", "eof")} if type(stderr) is dict else None
        pipe_clean = (type(stderr) is dict and stderr.get("eof") is True and stderr.get("errors") == [])
        return {
            "schema": "featured-owner-capture-v1", "case_id": case_id,
            "source_bindings": source_bindings,
            "stderr_binding": binding, "pipe_clean": pipe_clean,
            "capture_complete": not self.stopped and pipe_clean,
            "input_bytes_without_delimiters": self.input_bytes,
            "input_records": self.input_records, "events": self.events, "errors": self.errors,
            "http_cleanup_proven": False, "mcp_cleanup_proven": False, "whole_host_clean": False,
        }


def validate_sources(sources):
    need(type(sources) is dict and set(sources) == SOURCE_KEYS, "source_bindings")
    for key, value in sources.items():
        length = 40 if key in ("source_commit", "source_tree") else 64
        need(type(value) is str and re.fullmatch(r"[0-9a-f]{" + str(length) + "}", value), "source_bindings")


def validate_capture(capture, case_id, stderr, expected_sources):
    validate_sources(expected_sources)
    need(type(capture) is dict and capture.get("schema") == "featured-owner-capture-v1", "capture_schema")
    validate_sources(capture.get("source_bindings"))
    need(capture["source_bindings"] == expected_sources, "source_binding_mismatch")
    need(capture.get("case_id") == case_id and case_id in CASE_IDS, "capture_case")
    need(type(stderr) is dict and stderr.get("eof") is True and stderr.get("errors") == [], "stderr_incomplete")
    need(type(stderr.get("bytes")) is int and stderr["bytes"] >= 0, "stderr_binding")
    need(type(stderr.get("sha256")) is str and re.fullmatch(r"[0-9a-f]{64}", stderr["sha256"]), "stderr_binding")
    binding = capture.get("stderr_binding")
    need(type(binding) is dict and type(binding.get("bytes")) is int and
         type(binding.get("sha256")) is str and binding.get("eof") is True and
         binding == {key: stderr[key] for key in ("bytes", "sha256", "eof")}, "stderr_binding")
    need(capture.get("pipe_clean") is True and capture.get("capture_complete") is True, "capture_incomplete")
    errors = capture.get("errors")
    need(type(errors) is list and len(errors) <= MAX_ERRORS and errors == [], "capture_errors")
    records, count_bytes = capture.get("input_records"), capture.get("input_bytes_without_delimiters")
    need(type(records) is int and 0 <= records <= MAX_RECORDS, "record_limit")
    need(type(count_bytes) is int and 0 <= count_bytes <= MAX_INPUT_BYTES, "input_byte_limit")
    # PipeDrain strips one newline per complete line and accepts an EOF final line.
    need(0 <= stderr["bytes"] - count_bytes <= records, "stderr_byte_count")
    for field in ("http_cleanup_proven", "mcp_cleanup_proven", "whole_host_clean"):
        need(capture.get(field) is False, "scope_overclaim")
    events = capture.get("events")
    need(type(events) is list and len(events) <= MAX_EVENTS, "event_limit")
    process, scope, last_index = [], [], 0
    for event in events:
        need(type(event) is dict and set(event) == {"kind", "record_index", "fields"}, "event_schema")
        index = event["record_index"]
        need(type(index) is int and last_index < index <= records, "event_record_order")
        last_index = index
        need(event["kind"] in ("process", "scope"), "event_kind")
        clean_row(event["kind"], event["fields"])
        (process if event["kind"] == "process" else scope).append(event["fields"])
    need(bool(process), "missing_featured_process_receipt")
    need(bool(scope), "missing_featured_scope_receipt")
    return {
        "case_id": case_id, "featured_task_ownership_receipts_clean": True,
        "process_receipt_count": len(process), "scope_receipt_count": len(scope),
        "idle_scope_receipts": sum(row["task_ownership"] == "Idle" for row in scope),
        "joined_scope_receipts": sum(row["task_ownership"] == "Joined" for row in scope),
        "zero_joined_process_snapshots": sum(row["task_joined"] == 0 for row in process),
        "process_snapshot_counts_are_lifetime_totals": False,
        "started_work_requires_independent_gate": True,
        "http_cleanup_proven": False, "mcp_cleanup_proven": False, "whole_host_clean": False,
    }


def read_json(path):
    need(path.is_file() and not path.is_symlink(), "input_file_unavailable")
    with path.open("rb") as stream:
        data = stream.read(MAX_JSON_BYTES + 1)
    need(len(data) <= MAX_JSON_BYTES, "json_byte_limit")
    return json_value(data), hashlib.sha256(data).hexdigest()


def check_matrix(root, expected_sources):
    validate_sources(expected_sources)
    matrix, matrix_hash = read_json(root / "acceptance.json")
    need(type(matrix) is dict and matrix.get("schema") == "held-production-host-matrix-v1", "matrix_schema")
    need(matrix.get("all_eight_curated_runtime_gates_passed") is True, "original_matrix_not_passed")
    rows = matrix.get("cases")
    need(type(rows) is list and len(rows) == 8 and
         all(type(row) is dict for row in rows), "matrix_case_count")
    need([row.get("case_id") for row in rows] == list(CASE_IDS), "matrix_case_order")
    results = []
    for matrix_row in rows:
        case_id = matrix_row["case_id"]
        need(matrix_row.get("curated_runtime_gates_passed") is True, "original_case_not_passed")
        case, case_hash = read_json(root / case_id / "case-result.json")
        original_receipts, receipt_hash = read_json(root / case_id / "receipts.json")
        capture, capture_hash = read_json(root / case_id / "featured-receipts.json")
        need(case_hash == matrix_row.get("case_report_sha256") and
             receipt_hash == matrix_row.get("receipt_sha256"), "original_artifact_binding")
        need(type(case) is dict and case.get("schema") == "held-production-host-case-v1" and
             case.get("case_id") == case_id, "case_schema")
        need(case.get("curated_runtime_gates_passed") is True and
             case.get("forced_fixture_cleanup") is False and case.get("failure") is None and
             case.get("cleanup_errors") == [], "original_case_not_passed")
        need(case.get("binary_sha256") == case.get("binary_sha256_after") == expected_sources["binary_sha256"] and
             case.get("source_commit") == expected_sources["source_commit"] and
             case.get("source_tree") == expected_sources["source_tree"], "case_source_binding")
        need(type(original_receipts) is dict and
             original_receipts.get("schema") == "held-host-normalized-receipts-v1", "original_receipt_schema")
        need(capture_hash == case.get("featured_receipt_sha256"), "featured_artifact_binding")
        result = validate_capture(capture, case_id, case.get("stderr"), expected_sources)
        result.update(case_report_sha256=case_hash, original_receipt_sha256=receipt_hash,
                      featured_receipt_sha256=capture_hash, stderr_sha256=case["stderr"]["sha256"])
        results.append(result)
    return {
        "schema": "featured-owner-held-matrix-postcheck-v1", "status": "featured_task_receipt_gates_passed",
        "all_eight_featured_task_receipt_gates_passed": True,
        "original_matrix_sha256": matrix_hash, "cases": results,
        "source_bindings": expected_sources,
        "original_frozen_fixture_results_reinterpreted": False,
        "http_cleanup_proven": False, "mcp_cleanup_proven": False, "whole_host_clean": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    for key in sorted(SOURCE_KEYS):
        parser.add_argument("--" + key.replace("_", "-"), required=True)
    args = parser.parse_args()
    try:
        result = check_matrix(args.matrix_dir, {key: getattr(args, key) for key in SOURCE_KEYS})
    except ReceiptError as error:
        result = {"schema": "featured-owner-held-matrix-postcheck-v1", "status": "failed",
                  "all_eight_featured_task_receipt_gates_passed": False, "error": str(error),
                  "http_cleanup_proven": False, "mcp_cleanup_proven": False, "whole_host_clean": False}
    except OSError:
        result = {"schema": "featured-owner-held-matrix-postcheck-v1", "status": "failed",
                  "all_eight_featured_task_receipt_gates_passed": False, "error": "input_io_error",
                  "http_cleanup_proven": False, "mcp_cleanup_proven": False, "whole_host_clean": False}
    try:
        with args.output.open("x", encoding="utf-8") as stream:
            json.dump(result, stream, sort_keys=True, indent=2)
            stream.write("\n")
    except OSError:
        print("featured postcheck: output unavailable", file=sys.stderr)
        return 2
    return 0 if result["all_eight_featured_task_receipt_gates_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
