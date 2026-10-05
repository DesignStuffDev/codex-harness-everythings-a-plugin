"""Real local Git capsule I/O; planner result mocked, installed-host gate separate."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
from types import SimpleNamespace
import unittest
from unittest.mock import patch

PROJECT = Path(__file__).resolve().parents[1] / "examples" / "upstream-maintenance"
sys.path.insert(0, str(PROJECT))
import source_capsule as capsule

sys.path.pop(0)


class SourceCapsuleTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.repo = self.root / "objects.git"
        subprocess.run(
            ["git", "init", "--bare", str(self.repo)], check=True, capture_output=True
        )
        self.state = self.root / "state"
        self.state.mkdir()
        self.stop = threading.Event()
        self.base = self.commit({"plain": b"base", "gone": b"deleted"})
        self.upstream = self.commit({"plain": b"upstream", "new": b"added"}, self.base)
        self.custom = self.commit(
            {"plain": b"custom", "custom-only": b"keep"}, self.base
        )
        self.index = self.root / "index.json"
        self.index.write_text(json.dumps({"source": {"upstream_revision": self.base}}))
        self.request = dict(
            contract_version=1,
            purpose="prepare_source_inputs",
            expected_plan_id="a" * 64,
            limits=dict(
                max_changed_paths=1024,
                max_unique_blob_bytes=capsule.MAX_BYTES,
                max_manifest_bytes=capsule.MAX_MANIFEST,
            ),
            review_request=dict(
                repository=str(self.repo),
                candidate_revision=self.upstream,
                composition_revision=self.custom,
                lineage_index=str(self.index),
                lineage_sha256=hashlib.sha256(self.index.read_bytes()).hexdigest(),
            ),
        )
        planner = patch.object(
            capsule,
            "review_request",
            return_value=dict(
                status="review_required",
                plan_id="a" * 64,
                report=dict(unresolved=["fixture"]),
            ),
        )
        planner.start()
        self.addCleanup(planner.stop)

    def git(self, *args, data=None):
        env = dict(
            os.environ,
            GIT_AUTHOR_NAME="Fixture",
            GIT_AUTHOR_EMAIL="fixture@example.invalid",
            GIT_COMMITTER_NAME="Fixture",
            GIT_COMMITTER_EMAIL="fixture@example.invalid",
            GIT_AUTHOR_DATE="2000-01-01T00:00:00Z",
            GIT_COMMITTER_DATE="2000-01-01T00:00:00Z",
        )
        return subprocess.run(
            ["git", "-C", str(self.repo), *args],
            input=data,
            env=env,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        ).stdout

    def commit(self, files, parent=None, mode="100644"):
        rows = []
        for name, raw in sorted(files.items()):
            oid = self.git("hash-object", "-w", "--stdin", data=raw).strip().decode()
            rows.append(f"{mode} blob {oid}\t{name}\n")
        tree = self.git("mktree", data="".join(rows).encode()).strip().decode()
        return (
            self.git(
                "commit-tree",
                tree,
                *(["-p", parent] if parent else []),
                data=b"fixture\n",
            )
            .strip()
            .decode()
        )

    def prepare(self, emit=lambda event: None):
        return capsule.prepare_capsule(self.request, self.state, self.stop, emit)

    def test_real_objects_complete_capsule_and_corruption_detection(self):
        before = {
            str(p.relative_to(self.repo)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in self.repo.rglob("*")
            if p.is_file()
        }
        result = self.prepare()
        self.assertEqual(result["status"], "sealed")
        job = self.state / "upstream-capsules" / result["job_id"]
        manifest = json.loads((job / "MANIFEST.json").read_bytes())
        self.assertEqual(
            [row["path"] for row in manifest["paths"]], ["gone", "new", "plain"]
        )
        for blob in manifest["blobs"]:
            self.assertEqual(
                (job / "blobs" / blob["sha256"]).read_bytes(),
                self.git("cat-file", "blob", blob["git_oid"]),
            )
        self.assertEqual(
            before,
            {
                str(p.relative_to(self.repo)): hashlib.sha256(
                    p.read_bytes()
                ).hexdigest()
                for p in self.repo.rglob("*")
                if p.is_file()
            },
        )
        inspected = capsule.inspect_capsule(job)
        result["durability"] = "not_attested_by_inspection"
        self.assertEqual(inspected, result)
        (job / "blobs" / manifest["blobs"][0]["sha256"]).write_bytes(b"corrupt")
        self.assertEqual(
            capsule.inspect_capsule(job)["status"], "corrupt_or_unsupported"
        )

    def test_stale_plan_and_digest_cannot_create_job(self):
        for field in ("plan", "digest"):
            with self.subTest(field=field):
                if field == "plan":
                    self.request["expected_plan_id"] = "b" * 64
                else:
                    self.request["expected_plan_id"] = "a" * 64
                    self.request["review_request"]["lineage_sha256"] = "0" * 64
                self.assertFalse(self.prepare()["source_materials_ready"])
                self.assertFalse((self.state / "upstream-capsules").exists())

    def test_byte_budget_and_unsupported_mode_reject_before_writes(self):
        self.request["limits"]["max_unique_blob_bytes"] = 1
        self.assertEqual(self.prepare()["diagnostic_code"], "budget_exceeded")
        self.request["limits"]["max_unique_blob_bytes"] = capsule.MAX_BYTES
        self.request["review_request"]["candidate_revision"] = self.commit(
            {"link": b"../../outside"}, self.base, "120000"
        )
        self.assertEqual(self.prepare()["diagnostic_code"], "unsupported_source_entry")
        self.assertFalse((self.state / "upstream-capsules").exists())

    def test_cancel_after_intent_retains_incomplete_job(self):
        result = self.prepare(lambda event: self.stop.set())
        self.assertFalse(result["source_materials_ready"])
        job = self.state / "upstream-capsules" / result["job_id"]
        self.assertTrue((job / "INTENT.json").is_file())
        self.assertTrue((job / "TERMINAL.json").is_file())
        self.assertFalse((job / "SEALED.json").exists())
        self.assertEqual(
            capsule.inspect_capsule(job)["status"], "incomplete_or_uncertain"
        )

    def test_late_cancel_retains_sealed_result(self):
        def emit(event):
            if event["stage"] == "source_inputs_sealed":
                self.stop.set()

        result = self.prepare(emit)
        self.assertTrue(self.stop.is_set())
        self.assertEqual(result["status"], "sealed")

    def test_post_rename_fsync_failure_is_not_a_successful_seal(self):
        sync = capsule.sync_directory

        def fail_once(directory):
            if (directory / "SEALED.json").exists() and not (
                directory / "TERMINAL.json"
            ).exists():
                raise OSError("injected directory flush failure")
            sync(directory)

        with patch.object(capsule, "sync_directory", side_effect=fail_once):
            result = self.prepare()
        self.assertFalse(result["source_materials_ready"])
        job = self.state / "upstream-capsules" / result["job_id"]
        self.assertTrue((job / "SEALED.json").exists())
        self.assertEqual(
            capsule.inspect_capsule(job)["status"], "incomplete_or_uncertain"
        )

    def test_malformed_seal_and_missing_intent_are_not_inspected_as_success(self):
        result = self.prepare()
        job = self.state / "upstream-capsules" / result["job_id"]
        seal = (job / "SEALED.json").read_bytes()
        (job / "SEALED.json").write_text("[]")
        self.assertFalse(capsule.inspect_capsule(job)["source_materials_ready"])
        (job / "SEALED.json").write_bytes(seal)
        (job / "INTENT.json").unlink()
        self.assertFalse(capsule.inspect_capsule(job)["source_materials_ready"])

    def test_resealed_oversized_job_identity_cannot_expand_inspector_output(self):
        result = self.prepare()
        job = self.state / "upstream-capsules" / result["job_id"]
        manifest = json.loads((job / "MANIFEST.json").read_bytes())
        manifest["job_id"] = "x" * 10000
        raw = json.dumps(manifest).encode()
        (job / "MANIFEST.json").write_bytes(raw)
        seal = json.loads((job / "SEALED.json").read_bytes())
        seal["manifest_sha256"] = hashlib.sha256(raw).hexdigest()
        (job / "SEALED.json").write_text(json.dumps(seal))
        inspected = capsule.inspect_capsule(job)
        self.assertFalse(inspected["source_materials_ready"])
        self.assertLess(len(json.dumps(inspected)), 8192)

    def test_git_owner_failure_terminates_preparation(self):
        with patch.object(
            capsule.LocalObjects,
            "command",
            side_effect=capsule.OwnedGitError("owned_git_cleanup_unconfirmed"),
        ) as command:
            result = self.prepare()
        self.assertEqual(result["diagnostic_code"], "owned_git_cleanup_unconfirmed")
        self.assertEqual(command.call_count, 1)
        self.assertFalse((self.state / "upstream-capsules").exists())

    def test_existing_job_collision_preserves_every_file(self):
        first = self.prepare()
        job = self.state / "upstream-capsules" / first["job_id"]
        before = {
            str(p.relative_to(job)): p.read_bytes()
            for p in job.rglob("*")
            if p.is_file()
        }
        with patch.object(
            capsule.uuid, "uuid4", return_value=SimpleNamespace(hex=job.name)
        ):
            result = self.prepare()
        self.assertFalse(result["source_materials_ready"])
        self.assertIsNone(result["job_id"])
        self.assertEqual(
            before,
            {
                str(p.relative_to(job)): p.read_bytes()
                for p in job.rglob("*")
                if p.is_file()
            },
        )

    def test_same_revision_is_not_an_update_capsule(self):
        self.request["review_request"]["candidate_revision"] = self.base
        self.assertEqual(self.prepare()["diagnostic_code"], "later_revision_required")
        self.assertFalse((self.state / "upstream-capsules").exists())


if __name__ == "__main__":
    unittest.main()
