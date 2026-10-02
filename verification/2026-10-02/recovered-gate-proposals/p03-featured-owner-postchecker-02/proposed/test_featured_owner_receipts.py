"""Pure synthetic receipt tests. No native processes, network, or runtime fixtures."""

import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import featured_owner_receipts as m

SOURCES = {key: "a" * (40 if key in ("source_commit", "source_tree") else 64) for key in m.SOURCE_KEYS}


def process(**updates):
    row = dict(receipt_version=1, task_pending=0, task_joined=0, task_panicked=0,
               task_join_failed=0, optional_fetch_failed=0, task_ownership_clean=True)
    row.update(updates)
    return row


def scope(**updates):
    row = dict(receipt_version=1, task_ownership="Joined", optional_fetch_outcome="Some(Cancelled)",
               task_ownership_clean=True)
    row.update(updates)
    return row


def line(event, fields, fmt=False):
    if fmt:
        values = dict(event=event, **fields)
        return ("INFO " + m.TARGET + ": " + " ".join(
            key + "=" + (str(value).lower() if type(value) is bool else str(value))
            for key, value in values.items())).encode()
    return json.dumps(dict(target=m.TARGET, fields=dict(event=event, **fields))).encode()


def captured(lines, case_id=m.CASE_IDS[0]):
    capture = m.FeaturedCapture()
    for item in lines:
        capture.consume(item)
    raw = b"\n".join(lines) + b"\n"
    stderr = dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest(), eof=True, errors=[])
    return capture.report(case_id, stderr, SOURCES), stderr


class FeaturedReceiptsTest(unittest.TestCase):
    def valid(self, fmt=False):
        return captured([line(m.PROCESS_EVENT, process(), fmt), line(m.SCOPE_EVENT, scope(), fmt)])

    def check(self, capture, stderr):
        return m.validate_capture(capture, m.CASE_IDS[0], stderr, SOURCES)

    def test_json_and_fmt_tracing(self):
        for fmt in (False, True):
            with self.subTest(fmt=fmt):
                report = self.check(*self.valid(fmt))
                self.assertTrue(report["featured_task_ownership_receipts_clean"])
                self.assertFalse(report["whole_host_clean"])
                self.assertFalse(report["http_cleanup_proven"])

    def test_optional_failure_and_zero_snapshot_allowed(self):
        report = self.check(*captured([line(m.PROCESS_EVENT, process(optional_fetch_failed=1)),
                                     line(m.SCOPE_EVENT, scope(optional_fetch_outcome="Some(Failed)"))]))
        self.assertEqual(report["zero_joined_process_snapshots"], 1)
        self.assertEqual(report["joined_scope_receipts"], 1)

    def test_idle_is_clean_but_not_started_work_proof(self):
        report = self.check(*captured([line(m.PROCESS_EVENT, process()),
                                     line(m.SCOPE_EVENT, scope(task_ownership="Idle", optional_fetch_outcome="None"))]))
        self.assertEqual(report["idle_scope_receipts"], 1)
        self.assertEqual(report["joined_scope_receipts"], 0)
        self.assertTrue(report["started_work_requires_independent_gate"])

    def test_repeated_clean_receipts_counted(self):
        report = self.check(*captured([line(m.PROCESS_EVENT, process()), line(m.SCOPE_EVENT, scope()),
                                     line(m.PROCESS_EVENT, process(task_joined=1))]))
        self.assertEqual(report["process_receipt_count"], 2)
        self.assertEqual(report["scope_receipt_count"], 1)

    def test_earlier_bad_receipt_not_hidden_by_later_clean(self):
        cap, stderr = captured([line(m.PROCESS_EVENT, process(task_pending=1)),
                                line(m.PROCESS_EVENT, process()), line(m.SCOPE_EVENT, scope())])
        self.assertIn("process_unclean", cap["errors"])
        with self.assertRaises(m.ReceiptError):
            self.check(cap, stderr)

    def test_missing_each_kind(self):
        for event, fields in ((m.PROCESS_EVENT, process()), (m.SCOPE_EVENT, scope())):
            with self.subTest(event=event), self.assertRaises(m.ReceiptError):
                self.check(*captured([line(event, fields)]))

    def test_exact_integer_and_boolean_types(self):
        for row in (process(receipt_version=True), process(task_joined=True),
                    process(task_pending=0.0), process(task_ownership_clean=1),
                    process(task_joined=-1)):
            with self.subTest(row=row), self.assertRaises(m.ReceiptError):
                m.parse_line(line(m.PROCESS_EVENT, row))
        with self.assertRaises(m.ReceiptError):
            m.parse_line(line(m.SCOPE_EVENT, scope(task_ownership_clean=1)))

    def test_missing_field_and_unsupported_version(self):
        row = process()
        del row["task_pending"]
        for fields in (row, process(receipt_version=2)):
            with self.assertRaises(m.ReceiptError):
                m.parse_line(line(m.PROCESS_EVENT, fields))

    def test_all_process_unclean_fields(self):
        for key, value in (("task_pending", 1), ("task_panicked", 1),
                           ("task_join_failed", 1), ("task_ownership_clean", False)):
            with self.subTest(key=key), self.assertRaises(m.ReceiptError):
                m.parse_line(line(m.PROCESS_EVENT, process(**{key: value})))

    def test_scope_combinations(self):
        for outcome in ("Some(Succeeded)", "Some(Failed)", "Some(Cancelled)"):
            self.assertEqual(m.parse_line(line(m.SCOPE_EVENT, scope(optional_fetch_outcome=outcome)))[0], "scope")
        for ownership, outcome in (("Pending", "None"), ("Panicked", "None"),
                                   ("Joined", "None"), ("Idle", "Some(Succeeded)"),
                                   ("Joined", "Some(Other)")):
            with self.subTest(ownership=ownership, outcome=outcome), self.assertRaises(m.ReceiptError):
                m.parse_line(line(m.SCOPE_EVENT, scope(task_ownership=ownership, optional_fetch_outcome=outcome)))

    def test_malformed_json_and_duplicates(self):
        duplicate = line(m.PROCESS_EVENT, process()).replace(b'"task_pending": 0', b'"task_pending": 0, "task_pending": 1')
        for value in ((b'{"event":"' + m.PROCESS_EVENT.encode()), duplicate,
                      b'\xff', line(m.PROCESS_EVENT, process(), True) + b' task_pending=0'):
            with self.subTest(value=value), self.assertRaises(m.ReceiptError):
                m.parse_line(value)

    def test_wrong_target_matching_candidate_rejected(self):
        data = json.loads(line(m.PROCESS_EVENT, process()))
        data["target"] = "different::target"
        with self.assertRaisesRegex(m.ReceiptError, "malformed_matching_event"):
            m.parse_line(json.dumps(data).encode())
        self.assertIsNone(m.parse_line(b"INFO unrelated ordinary log"))
        self.assertIsNone(m.parse_line(b'{ordinary non-JSON CLI error text'))

    def test_matching_malformed_event_rejected(self):
        with self.assertRaises(m.ReceiptError):
            m.parse_line(("INFO " + m.TARGET + ": event=\"" + m.PROCESS_EVENT).encode())

    def test_matching_fmt_incomplete_duplicate_and_suffix_rejected(self):
        original = line(m.PROCESS_EVENT, process(), True)
        for suffix in (b" task_pending=", b" event=", b" task_pending=\"unterminated"):
            with self.subTest(suffix=suffix), self.assertRaises(m.ReceiptError):
                m.parse_line(original + suffix)
        value = original.replace(("event=" + m.PROCESS_EVENT).encode(),
                                 ("event=\"" + m.PROCESS_EVENT + "\"garbage").encode())
        with self.assertRaises(m.ReceiptError):
            m.parse_line(value)

    def test_escaped_event_duplicate_cannot_be_hidden(self):
        raw = line(m.PROCESS_EVENT, process()).replace(
            m.PROCESS_EVENT.encode(), b"\\u0066" + m.PROCESS_EVENT[1:].encode())
        raw = raw.replace(b'"task_pending": 0', b'"task_pending": 0, "task_pending": 1')
        with self.assertRaisesRegex(m.ReceiptError, "duplicate_json_key"):
            m.parse_line(raw)

    def test_input_bounds_stop_without_raw_log_retention(self):
        for name, value, lines, code in (
            ("MAX_LINE_BYTES", 2, [b"abc"], "line_byte_limit"),
            ("MAX_INPUT_BYTES", 2, [b"ab", b"c"], "input_byte_limit"),
            ("MAX_RECORDS", 1, [b"a", b"b"], "record_limit"),
            ("MAX_EVENTS", 1, [line(m.PROCESS_EVENT, process()), line(m.SCOPE_EVENT, scope())], "event_limit"),
        ):
            with self.subTest(name=name), patch.object(m, name, value):
                cap, _ = captured(lines)
                self.assertEqual(cap["errors"], [code])
                self.assertFalse(cap["capture_complete"])
                self.assertNotIn("raw", cap)

    def test_capture_failure_does_not_stop_original_receipt_consumer(self):
        # Exercise only the additive pure consumer, never PipeDrain/process startup.
        import held_production_host as held
        receipts = held.Receipts({"session_loop": None, "mcp": None})
        with patch.object(m, "MAX_LINE_BYTES", 1):
            receipts.consume(b"too long")
        fields = {key: {int: 0, bool: False, str: "synthetic"}[kind]
                  for key, kind in held.CURATED.items()}
        receipts.consume(json.dumps({"target": held.TARGET,
                         "fields": dict(event=held.EVENT, **fields)}).encode())
        self.assertEqual(len(receipts.curated), 1)
        self.assertEqual(receipts.errors, [])
        self.assertEqual(receipts.featured.errors, ["line_byte_limit"])

    def test_pipe_failure_cannot_pass(self):
        cap, stderr = self.valid()
        for key, value in (("eof", False), ("errors", ["pipe_eof_not_observed"])):
            altered = dict(stderr, **{key: value})
            with self.assertRaises(m.ReceiptError):
                self.check(cap, altered)

    def test_capture_binding_and_count_tampering_rejected(self):
        cap, stderr = self.valid()
        for key, value in (("input_records", True), ("input_records", m.MAX_RECORDS + 1),
                           ("input_bytes_without_delimiters", 0), ("capture_complete", 1),
                           ("http_cleanup_proven", True)):
            with self.subTest(key=key), self.assertRaises(m.ReceiptError):
                self.check(dict(cap, **{key: value}), stderr)
        altered = copy.deepcopy(cap)
        altered["events"][1]["record_index"] = altered["events"][0]["record_index"]
        with self.assertRaises(m.ReceiptError):
            self.check(altered, stderr)
        altered = copy.deepcopy(cap)
        altered["source_bindings"]["collector_sha256"] = "b" * 64
        with self.assertRaisesRegex(m.ReceiptError, "source_binding_mismatch"):
            self.check(altered, stderr)

    def test_json_read_limit_and_duplicate(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "data.json"
            path.write_bytes(b'{"a":1,"a":2}')
            with self.assertRaises(m.ReceiptError):
                m.read_json(path)
            path.write_bytes(b"[] " * 4)
            with patch.object(m, "MAX_JSON_BYTES", 8), self.assertRaises(m.ReceiptError):
                m.read_json(path)

    def make_matrix(self, root):
        rows = []
        def save(path, data):
            raw = json.dumps(data, sort_keys=True).encode()
            path.write_bytes(raw)
            return hashlib.sha256(raw).hexdigest()
        for case_id in m.CASE_IDS:
            directory = root / case_id
            directory.mkdir()
            cap, stderr = captured([line(m.PROCESS_EVENT, process()), line(m.SCOPE_EVENT, scope())], case_id)
            cap_hash = save(directory / "featured-receipts.json", cap)
            original_hash = save(directory / "receipts.json", {"schema": "held-host-normalized-receipts-v1"})
            case_hash = save(directory / "case-result.json", {
                "schema": "held-production-host-case-v1", "case_id": case_id,
                "curated_runtime_gates_passed": True, "forced_fixture_cleanup": False,
                "failure": None, "cleanup_errors": [], "stderr": stderr, "featured_receipt_sha256": cap_hash,
                "binary_sha256": SOURCES["binary_sha256"], "binary_sha256_after": SOURCES["binary_sha256"],
                "source_commit": SOURCES["source_commit"], "source_tree": SOURCES["source_tree"],
            })
            rows.append(dict(case_id=case_id, curated_runtime_gates_passed=True,
                             case_report_sha256=case_hash, receipt_sha256=original_hash))
        save(root / "acceptance.json", {"schema": "held-production-host-matrix-v1",
             "all_eight_curated_runtime_gates_passed": True, "cases": rows})

    def test_full_eight_synthetic_artifact_bindings(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.make_matrix(root)
            report = m.check_matrix(root, SOURCES)
            self.assertTrue(report["all_eight_featured_task_receipt_gates_passed"])
            self.assertEqual([row["case_id"] for row in report["cases"]], list(m.CASE_IDS))
            self.assertFalse(report["whole_host_clean"])

    def test_missing_capture_not_retrofitted(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.make_matrix(root)
            (root / m.CASE_IDS[0] / "featured-receipts.json").unlink()
            with self.assertRaisesRegex(m.ReceiptError, "input_file_unavailable"):
                m.check_matrix(root, SOURCES)

    def test_tampered_sidecar_not_accepted(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.make_matrix(root)
            path = root / m.CASE_IDS[0] / "featured-receipts.json"
            path.write_bytes(path.read_bytes() + b"\n")
            with self.assertRaisesRegex(m.ReceiptError, "featured_artifact_binding"):
                m.check_matrix(root, SOURCES)

    def test_original_failure_not_promoted(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.make_matrix(root)
            path = root / "acceptance.json"
            data = json.loads(path.read_text())
            data["all_eight_curated_runtime_gates_passed"] = False
            path.write_text(json.dumps(data))
            with self.assertRaisesRegex(m.ReceiptError, "original_matrix_not_passed"):
                m.check_matrix(root, SOURCES)

    def test_matrix_order_duplicate_and_count_rejected(self):
        for mode in ("reverse", "duplicate", "missing"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                self.make_matrix(root)
                path = root / "acceptance.json"
                data = json.loads(path.read_text())
                if mode == "reverse":
                    data["cases"].reverse()
                elif mode == "duplicate":
                    data["cases"][1] = data["cases"][0]
                else:
                    data["cases"].pop()
                path.write_text(json.dumps(data))
                with self.assertRaises(m.ReceiptError):
                    m.check_matrix(root, SOURCES)

    def test_malformed_expected_sources_rejected(self):
        for sources in ({}, dict(SOURCES, fixture_sha256=True), dict(SOURCES, source_commit="invalid")):
            with self.assertRaisesRegex(m.ReceiptError, "source_bindings"):
                m.validate_sources(sources)

    def test_case_source_schema_and_original_hash_mutations_rejected(self):
        for mode, error in (("binary_after", "case_source_binding"), ("source_tree", "case_source_binding"),
                            ("case_schema", "case_schema"), ("receipt_schema", "original_receipt_schema"),
                            ("case_hash", "original_artifact_binding"), ("receipt_hash", "original_artifact_binding")):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                self.make_matrix(root)
                matrix_path = root / "acceptance.json"
                matrix = json.loads(matrix_path.read_text())
                is_receipt = mode.startswith("receipt_")
                name = "receipts.json" if is_receipt else "case-result.json"
                path = root / m.CASE_IDS[0] / name
                data = json.loads(path.read_text())
                key = {"binary_after": "binary_sha256_after", "source_tree": "source_tree",
                       "case_schema": "schema", "receipt_schema": "schema",
                       "case_hash": "synthetic_mutation", "receipt_hash": "synthetic_mutation"}[mode]
                data[key] = "invalid"
                raw = json.dumps(data).encode()
                path.write_bytes(raw)
                if not mode.endswith("_hash"):
                    matrix["cases"][0]["receipt_sha256" if is_receipt else "case_report_sha256"] = hashlib.sha256(raw).hexdigest()
                    matrix_path.write_text(json.dumps(matrix))
                with self.assertRaisesRegex(m.ReceiptError, error):
                    m.check_matrix(root, SOURCES)


if __name__ == "__main__":
    unittest.main()
