"""Real local Git/merge I/O; source planner mocked, installed-host gate separate."""

import hashlib
import json
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import test_source_capsule as fixtures

PROJECT = Path(__file__).resolve().parents[1] / "examples" / "upstream-maintenance"
sys.path.insert(0, str(PROJECT))
import candidate_overlay as overlay

sys.path.pop(0)

BASE = b"alpha\none\ntwo\nthree\nfour\nfive\nomega\n"
UPSTREAM = BASE.replace(b"alpha", b"upstream")
CUSTOM = BASE.replace(b"omega", b"custom")


class CandidateOverlayTests(unittest.TestCase):
    def setUp(self):
        self.fixture = fixtures.SourceCapsuleTests()
        self.addCleanup(self.fixture.doCleanups)
        self.fixture.setUp()

    def inputs(self, base=None, upstream=None, custom=None, modes=None):
        f = self.fixture
        modes = modes or ("100644", "100644", "100644")
        base = {"plain": BASE} if base is None else base
        upstream = {"plain": UPSTREAM} if upstream is None else upstream
        custom = {"plain": CUSTOM} if custom is None else custom
        f.base = f.commit(base, mode=modes[0])
        f.upstream = f.commit(upstream, f.base, modes[1])
        f.custom = f.commit(custom, f.base, modes[2])
        f.index.write_text(json.dumps({"source": {"upstream_revision": f.base}}))
        f.request["review_request"].update(
            candidate_revision=f.upstream,
            composition_revision=f.custom,
            lineage_sha256=hashlib.sha256(f.index.read_bytes()).hexdigest(),
        )
        source = f.prepare()
        self.assertEqual(source["status"], "sealed")
        self.source = f.state / "upstream-capsules" / source["job_id"]
        self.request = dict(
            contract_version=1,
            purpose="prepare_candidate_overlay",
            source_capsule_id=source["job_id"],
            expected_manifest_sha256=source["manifest_sha256"],
            limits=dict(
                max_changed_paths=1024,
                max_output_bytes=67108864,
                max_manifest_bytes=8388608,
            ),
        )

    def prepare(self, emit=lambda event: None):
        result = overlay.prepare_overlay(
            self.request, self.fixture.state, self.fixture.stop, emit
        )
        for key in ("candidate_assembled", "update_allowed", "activation_allowed"):
            self.assertIs(result[key], False)
        return result

    def job(self, result):
        return self.fixture.state / "upstream-overlays" / result["job_id"]

    def rows(self, result):
        return json.loads((self.job(result) / "OVERLAY.json").read_bytes())["paths"]

    def files(self, directory):
        return {
            str(p.relative_to(directory)): p.read_bytes()
            for p in directory.rglob("*")
            if p.is_file()
        }

    def test_disjoint_real_merge_preserves_both_changes(self):
        self.inputs()
        source_before = self.files(self.source)
        result = self.prepare()
        self.assertEqual(result["status"], "prepared")
        (row,) = self.rows(result)
        self.assertEqual(row["strategy"], "three_way_merge")
        raw = (self.job(result) / "blobs" / row["output"]["sha256"]).read_bytes()
        self.assertEqual(raw, UPSTREAM.replace(b"omega", b"custom"))
        self.assertEqual(
            row["output"]["git_oid"],
            hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest(),
        )
        self.assertEqual(
            overlay.inspect_overlay(self.job(result))["integrity"], "verified"
        )
        self.assertEqual(source_before, self.files(self.source))

    def test_genuine_overlap_is_reviewable_without_conflict_output(self):
        self.inputs(custom={"plain": BASE.replace(b"alpha", b"custom")})
        result = self.prepare()
        self.assertEqual(result["status"], "review_required")
        (row,) = self.rows(result)
        self.assertIsNone(row["output"])
        self.assertEqual(row["diagnostic_code"], "merge_not_clean")
        self.assertTrue((self.job(result) / "SEALED.json").is_file())
        self.assertEqual(
            overlay.inspect_overlay(self.job(result))["status"], "review_required"
        )

    def test_added_and_already_adapted_paths_choose_correct_inputs(self):
        self.inputs(
            upstream={"plain": UPSTREAM, "new": b"added\n"}, custom={"plain": UPSTREAM}
        )
        result = self.prepare()
        self.assertEqual(result["status"], "prepared")
        rows = {row["path"]: row for row in self.rows(result)}
        self.assertEqual(rows["new"]["strategy"], "choose_upstream")
        self.assertEqual(rows["plain"]["strategy"], "unchanged")
        for name, expected in (("new", b"added\n"), ("plain", UPSTREAM)):
            self.assertEqual(
                (
                    self.job(result) / "blobs" / rows[name]["output"]["sha256"]
                ).read_bytes(),
                expected,
            )

    def test_deletions_and_mode_changes_require_review(self):
        for kwargs in (
            dict(upstream={}),
            dict(custom={}),
            dict(modes=("100644", "100755", "100644")),
        ):
            with self.subTest(kwargs=kwargs):
                self.inputs(**kwargs)
                result = self.prepare()
                self.assertEqual(result["status"], "review_required")
                (row,) = self.rows(result)
                self.assertIsNone(row["output"])
                self.assertTrue(row["diagnostic_code"])

    def test_add_add_and_binary_merges_require_review(self):
        for base, upstream, custom in (
            ({}, {"plain": b"up\n"}, {"plain": b"ours\n"}),
            ({"plain": b"base\0"}, {"plain": b"up\0"}, {"plain": b"ours\0"}),
        ):
            with self.subTest(base=base):
                self.inputs(base, upstream, custom)
                result = self.prepare()
                self.assertEqual(result["status"], "review_required")
                self.assertTrue(all(row["output"] is None for row in self.rows(result)))

    def test_stale_hash_and_unknown_contract_create_no_job(self):
        self.inputs()
        original = dict(self.request)
        for field, value in (
            ("expected_manifest_sha256", "0" * 64),
            ("contract_version", 2),
        ):
            with self.subTest(field=field):
                self.request = dict(original, **{field: value})
                self.assertEqual(self.prepare()["status"], "incomplete_or_unavailable")
                self.assertFalse((self.fixture.state / "upstream-overlays").exists())

    def test_path_output_and_manifest_budgets_cannot_seal_success(self):
        self.inputs(upstream={"plain": UPSTREAM, "new": b"added\n"})
        original = dict(self.request["limits"])
        for limit in original:
            with self.subTest(limit=limit):
                self.request["limits"] = dict(original, **{limit: 1})
                result = self.prepare()
                self.assertEqual(result["status"], "incomplete_or_unavailable")
                if result.get("job_id"):
                    self.assertNotIn(
                        overlay.inspect_overlay(self.job(result))["status"],
                        ("prepared", "review_required"),
                    )

    def test_cancellation_after_admission_retains_incomplete_job(self):
        self.inputs()
        events = []

        def stop_after_admission(event):
            events.append(event)
            if event["stage"] == "source_overlay_admitted":
                self.fixture.stop.set()

        result = self.prepare(stop_after_admission)
        self.assertTrue(any(e["stage"] == "source_overlay_admitted" for e in events))
        self.assertEqual(result["status"], "incomplete_or_unavailable")
        job = self.job(result)
        self.assertTrue((job / "INTENT.json").is_file())
        self.assertTrue((job / "TERMINAL.json").is_file())
        self.assertFalse((job / "SEALED.json").exists())
        self.assertNotIn(
            overlay.inspect_overlay(job)["status"], ("prepared", "review_required")
        )

    def test_corrupt_or_missing_source_is_never_prepared(self):
        self.inputs()
        manifest = json.loads((self.source / "MANIFEST.json").read_bytes())
        blob = self.source / "blobs" / manifest["blobs"][0]["sha256"]
        blob.write_bytes(b"corrupt")
        self.assertEqual(self.prepare()["status"], "incomplete_or_unavailable")
        blob.unlink()
        self.assertEqual(self.prepare()["status"], "incomplete_or_unavailable")
        self.assertFalse((self.fixture.state / "upstream-overlays").exists())

    def test_overlay_inspector_rejects_corrupt_blob_and_missing_intent(self):
        self.inputs()
        result = self.prepare()
        job = self.job(result)
        blob = job / "blobs" / self.rows(result)[0]["output"]["sha256"]
        original = blob.read_bytes()
        blob.write_bytes(b"corrupt")
        self.assertNotIn(
            overlay.inspect_overlay(job)["status"], ("prepared", "review_required")
        )
        blob.write_bytes(original)
        (job / "INTENT.json").unlink()
        self.assertNotIn(
            overlay.inspect_overlay(job)["status"], ("prepared", "review_required")
        )

    def test_uuid_collision_preserves_existing_job_exactly(self):
        self.inputs()
        first = self.prepare()
        job = self.job(first)
        before = self.files(job)
        with patch.object(
            overlay.uuid, "uuid4", return_value=SimpleNamespace(hex=job.name)
        ):
            result = self.prepare()
        self.assertEqual(result["status"], "incomplete_or_unavailable")
        self.assertIsNone(result.get("job_id"))
        self.assertEqual(before, self.files(job))

    def test_pre_cancelled_request_creates_no_overlay_job(self):
        self.inputs()
        self.fixture.stop.set()
        self.assertEqual(self.prepare()["status"], "incomplete_or_unavailable")
        self.assertFalse((self.fixture.state / "upstream-overlays").exists())

    def test_changed_private_merge_input_cannot_be_sealed(self):
        self.inputs()
        before = self.files(self.source)

        def mutate(event):
            if event["stage"] == "source_overlay_merge_starting":
                job = self.fixture.state / "upstream-overlays" / event["job_id"]
                (job / "merge-inputs/base").write_bytes(b"changed after admission\n")

        result = self.prepare(mutate)
        self.assertEqual(result["status"], "incomplete_or_unavailable")
        self.assertEqual(result["diagnostic_code"], "merge_input_changed")
        self.assertFalse((self.job(result) / "SEALED.json").exists())
        self.assertEqual(before, self.files(self.source))

    def test_resealed_receipt_contradictions_are_not_integrity_verified(self):
        self.inputs()
        job = self.job(self.prepare())
        originals = {
            name: (job / name).read_bytes()
            for name in ("OVERLAY.json", "INTENT.json", "SEALED.json")
        }
        for field in (
            "candidate_assembled",
            "update_allowed",
            "activation_allowed",
            "request_sha256",
            "merge_returncode",
            "application_base",
        ):
            with self.subTest(field=field):
                manifest, intent, seal = (
                    json.loads(originals[name])
                    for name in ("OVERLAY.json", "INTENT.json", "SEALED.json")
                )
                if field == "request_sha256":
                    manifest[field] = intent[field] = "0" * 64
                elif field == "merge_returncode":
                    manifest["paths"][0][field] = 1
                elif field == "application_base":
                    manifest[field] = "arbitrary_tree"
                else:
                    manifest[field] = True
                raw, intent_raw = overlay.canonical(manifest), overlay.canonical(intent)
                seal.update(
                    overlay_sha256=hashlib.sha256(raw).hexdigest(),
                    intent_sha256=hashlib.sha256(intent_raw).hexdigest(),
                )
                (job / "OVERLAY.json").write_bytes(raw)
                (job / "INTENT.json").write_bytes(intent_raw)
                (job / "SEALED.json").write_bytes(overlay.canonical(seal))
                self.assertEqual(
                    overlay.inspect_overlay(job)["status"], "corrupt_or_unsupported"
                )

    def test_post_seal_rename_fsync_failure_retains_terminal_failure(self):
        self.inputs()
        sync = fixtures.capsule.sync_directory

        def fail_once(directory):
            if (directory / "SEALED.json").exists() and not (
                directory / "TERMINAL.json"
            ).exists():
                raise OSError("injected seal directory flush failure")
            sync(directory)

        with patch.object(fixtures.capsule, "sync_directory", side_effect=fail_once):
            result = self.prepare()
        self.assertEqual(result["status"], "incomplete_or_unavailable")
        job = self.job(result)
        self.assertTrue((job / "SEALED.json").exists())
        self.assertTrue((job / "TERMINAL.json").exists())
        self.assertEqual(
            overlay.inspect_overlay(job)["status"], "incomplete_or_uncertain"
        )

    def test_git_cleanup_unconfirmed_is_terminal_not_a_merge_conflict(self):
        self.inputs()
        with patch.object(
            overlay.owned_git,
            "capture",
            side_effect=overlay.owned_git.OwnedGitError(
                "owned_git_cleanup_unconfirmed"
            ),
        ) as capture:
            result = self.prepare()
        self.assertEqual(capture.call_count, 1)
        self.assertEqual(result["status"], "incomplete_or_unavailable")
        self.assertEqual(result["diagnostic_code"], "owned_git_cleanup_unconfirmed")
        job = self.job(result)
        self.assertTrue((job / "TERMINAL.json").exists())
        self.assertFalse((job / "SEALED.json").exists())

    def test_merge_input_limit_does_not_restrict_direct_source_selection(self):
        large = b"x" * (128 * 1024 + 1) + b"\n"
        self.inputs(
            {"plain": large}, {"plain": large + b"up\n"}, {"plain": large + b"custom\n"}
        )
        with patch.object(overlay.owned_git, "capture") as capture:
            result = self.prepare()
        capture.assert_not_called()
        self.assertEqual(result["status"], "review_required")
        self.assertEqual(
            self.rows(result)[0]["diagnostic_code"], "merge_input_limit_requires_review"
        )
        self.inputs({"plain": large}, {"plain": large + b"up\n"}, {"plain": large})
        result = self.prepare()
        self.assertEqual(result["status"], "prepared")
        self.assertEqual(self.rows(result)[0]["output"]["bytes"], len(large + b"up\n"))


if __name__ == "__main__":
    unittest.main()
