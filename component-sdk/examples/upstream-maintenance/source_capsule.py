"""Immutable, sparse source-input preparation; never candidate assembly or activation."""

import hashlib
import json
import os
from pathlib import Path
import re
import time
import uuid

from maintenance_review import LocalObjects, canonical, exact_file, review_request
from owned_git import OwnedGitError

MAX_BLOB = 4 * 1024 * 1024
MAX_BYTES = 64 * 1024 * 1024
MAX_MANIFEST = 8 * 1024 * 1024
HEX = re.compile(r"[0-9a-f]{64}\Z")
OID = re.compile(r"[0-9a-f]{40}\Z")


class CapsuleError(ValueError):
    pass


def outcome(status, **fields):
    return dict(
        status=status,
        candidate_assembled=False,
        update_allowed=False,
        activation_allowed=False,
        self_contained_candidate=False,
        **fields,
    )


def regular_bytes(path, limit):
    if path.is_symlink() or not path.is_file() or path.resolve() != path.absolute():
        raise CapsuleError("regular_artifact_required")
    with path.open("rb") as stream:
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise CapsuleError("budget_exceeded")
    return raw


def sync_directory(directory):
    fd = os.open(directory, os.O_RDONLY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def atomic_file(directory, name, raw, check):
    temporary = directory / (name + ".tmp")
    with temporary.open("xb") as stream:
        os.chmod(temporary, 0o600)
        for offset in range(0, len(raw), 65536):
            check()
            stream.write(raw[offset : offset + 65536])
        stream.flush()
        os.fsync(stream.fileno())
    check()
    os.replace(temporary, directory / name)
    sync_directory(directory)


def inspect_capsule(job):
    """Verify present artifacts, not prior crash durability, without host or Git."""
    job = Path(job).absolute()
    try:
        if job.is_symlink() or not job.is_dir() or job.resolve() != job:
            raise CapsuleError("regular_job_directory_required")
        if (job / "TERMINAL.json").exists() or not (job / "SEALED.json").exists():
            return outcome("incomplete_or_uncertain", source_materials_ready=False)
        seal = json.loads(regular_bytes(job / "SEALED.json", 8192))
        raw = regular_bytes(job / "MANIFEST.json", MAX_MANIFEST)
        intent_raw = regular_bytes(job / "INTENT.json", 8192)
        if (
            not isinstance(seal, dict)
            or seal.get("schema") != "source-input-capsule-seal-v1"
        ):
            raise CapsuleError("unsupported_receipt_version")
        if (
            seal["manifest_sha256"] != hashlib.sha256(raw).hexdigest()
            or seal["intent_sha256"] != hashlib.sha256(intent_raw).hexdigest()
        ):
            raise CapsuleError("corrupt_capsule")
        manifest, intent = json.loads(raw), json.loads(intent_raw)
        if (
            not isinstance(manifest, dict)
            or not isinstance(intent, dict)
            or manifest.get("schema") != "source-input-capsule-v1"
            or intent.get("schema") != "source-input-capsule-intent-v1"
            or not re.fullmatch(r"[0-9a-f]{32}", job.name)
            or manifest["job_id"] != job.name
            or intent["job_id"] != job.name
            or manifest["commits"] != intent["commits"]
            or manifest["trees"] != intent["trees"]
            or manifest["limits"] != intent["limits"]
            or manifest["review"]["plan_id"] != intent["plan_id"]
            or manifest["request_sha256"] != intent["request_sha256"]
        ):
            raise CapsuleError("corrupt_capsule")
        for mapping in (manifest["commits"], manifest["trees"]):
            if set(mapping) != {"base", "upstream", "custom"} or any(
                not OID.fullmatch(v) for v in mapping.values()
            ):
                raise CapsuleError("corrupt_capsule")
        limits = manifest["limits"]
        if (
            not isinstance(limits, dict)
            or any(
                type(limits[k]) is not int or not 1 <= limits[k] <= v
                for k, v in {
                    "max_changed_paths": 1024,
                    "max_unique_blob_bytes": MAX_BYTES,
                    "max_manifest_bytes": MAX_MANIFEST,
                }.items()
            )
            or len(raw) > limits["max_manifest_bytes"]
            or not isinstance(manifest["blobs"], list)
            or not isinstance(manifest["paths"], list)
            or len(manifest["blobs"]) > 3072
            or len(manifest["paths"]) > limits["max_changed_paths"]
        ):
            raise CapsuleError("budget_exceeded")
        total, available, used, paths = 0, set(), set(), set()
        for blob in manifest["blobs"]:
            oid, digest = blob["git_oid"], blob["sha256"]
            if (
                not OID.fullmatch(oid)
                or not HEX.fullmatch(digest)
                or type(blob["bytes"]) is not int
            ):
                raise CapsuleError("corrupt_capsule")
            data = regular_bytes(job / "blobs" / digest, MAX_BLOB)
            total += len(data)
            if (
                total > limits["max_unique_blob_bytes"]
                or len(data) != blob["bytes"]
                or hashlib.sha256(data).hexdigest() != digest
                or hashlib.sha1(
                    b"blob " + str(len(data)).encode() + b"\0" + data
                ).hexdigest()
                != oid
                or oid in available
            ):
                raise CapsuleError("corrupt_capsule")
            available.add(oid)
        for row in manifest["paths"]:
            path = row["path"]
            if not isinstance(path, str) or not 1 <= len(path) <= 4096 or path in paths:
                raise CapsuleError("corrupt_capsule")
            paths.add(path)
            for side in ("base", "upstream", "custom"):
                entry = row[side]
                if entry is not None:
                    if (
                        entry["type"] != "blob"
                        or entry["mode"] not in ("100644", "100755")
                        or entry["blob"] not in available
                    ):
                        raise CapsuleError("corrupt_capsule")
                    used.add(entry["blob"])
        if used != available:
            raise CapsuleError("corrupt_capsule")
        return outcome(
            "sealed",
            source_materials_ready=True,
            durability="not_attested_by_inspection",
            job_id=job.name,
            manifest_sha256=seal["manifest_sha256"],
            changed_paths=len(manifest["paths"]),
            retained_blob_bytes=total,
        )
    except (OSError, ValueError, TypeError, KeyError, AttributeError, RecursionError):
        return outcome("corrupt_or_unsupported", source_materials_ready=False)


def prepare_capsule(request, state_dir, stop, emit=lambda event: None):
    """Write only a new private job; every incomplete attempt remains inspectable."""
    job, sealed = None, False
    deadline = time.monotonic() + 90

    def check():
        if stop.is_set() or time.monotonic() >= deadline:
            raise CapsuleError("cancelled_or_deadline_reached")

    try:
        if (
            not isinstance(request, dict)
            or set(request)
            != {
                "contract_version",
                "purpose",
                "review_request",
                "expected_plan_id",
                "limits",
            }
            or type(request["contract_version"]) is not int
            or request["contract_version"] != 1
            or request["purpose"] != "prepare_source_inputs"
            or not isinstance(request["expected_plan_id"], str)
            or not HEX.fullmatch(request["expected_plan_id"])
        ):
            raise CapsuleError("invalid_request")
        limits = request["limits"]
        ceilings = {
            "max_changed_paths": 1024,
            "max_unique_blob_bytes": MAX_BYTES,
            "max_manifest_bytes": MAX_MANIFEST,
        }
        if (
            not isinstance(limits, dict)
            or set(limits) != set(ceilings)
            or any(
                type(limits[k]) is not int or not 1 <= limits[k] <= v
                for k, v in ceilings.items()
            )
        ):
            raise CapsuleError("invalid_limits")
        check()
        review = review_request(request["review_request"], stop, full_report=True)
        if (
            review.get("status") != "review_required"
            or review.get("plan_id") != request["expected_plan_id"]
        ):
            raise CapsuleError("plan_changed_or_unavailable")
        if "report" not in review or review.get("detailed_report_omitted"):
            raise CapsuleError("full_report_unavailable")
        check()
        source = request["review_request"]
        index = json.loads(
            exact_file(source["lineage_index"], source["lineage_sha256"])
        )
        base = index["source"]["upstream_revision"]
        chosen, custom = source["candidate_revision"], source["composition_revision"]
        if base == chosen:
            raise CapsuleError("later_revision_required")
        objects = LocalObjects(source["repository"], stop)
        objects.deadline = min(objects.deadline, deadline)
        commits = {"base": base, "upstream": chosen, "custom": custom}
        trees = {name: objects.commit_tree(oid) for name, oid in commits.items()}
        objects.command("merge-base", "--is-ancestor", base, chosen)
        entries = {name: objects.tree(oid) for name, oid in trees.items()}
        changed = sorted(
            path
            for path in entries["base"].keys() | entries["upstream"].keys()
            if entries["base"].get(path) != entries["upstream"].get(path)
        )
        if len(changed) > limits["max_changed_paths"]:
            raise CapsuleError("budget_exceeded")
        paths, sizes = [], {}
        for path in changed:
            check()
            if not 1 <= len(path) <= 4096:
                raise CapsuleError("unsupported_source_entry")
            row = dict(path=path)
            for name, values in entries.items():
                entry = values.get(path)
                if entry is not None:
                    if entry["type"] != "blob" or entry["mode"] not in (
                        "100644",
                        "100755",
                    ):
                        raise CapsuleError("unsupported_source_entry")
                    oid = entry["blob"]
                    if oid not in sizes:
                        sizes[oid] = int(objects.command("cat-file", "-s", oid))
                        if (
                            not 0 <= sizes[oid] <= MAX_BLOB
                            or sum(sizes.values()) > limits["max_unique_blob_bytes"]
                        ):
                            raise CapsuleError("budget_exceeded")
                row[name] = entry
            paths.append(row)
        request_hash = hashlib.sha256(canonical(request)).hexdigest()
        manifest = dict(
            schema="source-input-capsule-v1",
            job_id="0" * 32,
            commits=commits,
            limits=limits,
            request_sha256=request_hash,
            gate_inventory_complete=False,
            trees=trees,
            composition_scope="committed_tree_only",
            review=review,
            source_materials_ready=True,
            candidate_assembled=False,
            activation_allowed=False,
            paths=paths,
            blobs=[
                dict(git_oid=k, sha256="0" * 64, bytes=v)
                for k, v in sorted(sizes.items())
            ],
        )
        if len(canonical(manifest)) > limits["max_manifest_bytes"]:
            raise CapsuleError("budget_exceeded")
        check()
        state = Path(state_dir).absolute()
        if state.is_symlink() or not state.is_dir() or state.resolve() != state:
            raise CapsuleError("regular_state_directory_required")
        root = state / "upstream-capsules"
        root.mkdir(mode=0o700, exist_ok=True)
        if root.is_symlink() or not root.is_dir() or root.stat().st_mode & 0o077:
            raise CapsuleError("private_state_directory_required")
        candidate = root / uuid.uuid4().hex
        candidate.mkdir(mode=0o700)
        job = candidate
        sync_directory(state)
        sync_directory(root)
        manifest["job_id"] = job.name
        intent_raw = canonical(
            dict(
                schema="source-input-capsule-intent-v1",
                job_id=job.name,
                request_sha256=request_hash,
                plan_id=review["plan_id"],
                limits=limits,
                commits=commits,
                trees=trees,
            )
        )
        atomic_file(job, "INTENT.json", intent_raw, check)
        (job / "blobs").mkdir(mode=0o700)
        emit(
            dict(
                stage="source_inputs_admitted",
                job_id=job.name,
                changed_paths=len(paths),
            )
        )
        for blob in manifest["blobs"]:
            check()
            data = objects.blob(blob["git_oid"])
            if (
                len(data) != blob["bytes"]
                or hashlib.sha1(
                    b"blob " + str(len(data)).encode() + b"\0" + data
                ).hexdigest()
                != blob["git_oid"]
            ):
                raise CapsuleError("input_changed")
            blob["sha256"] = hashlib.sha256(data).hexdigest()
            atomic_file(job / "blobs", blob["sha256"], data, check)
        raw = canonical(manifest)
        atomic_file(job, "MANIFEST.json", raw, check)
        atomic_file(
            job,
            "SEALED.json",
            canonical(
                dict(
                    schema="source-input-capsule-seal-v1",
                    manifest_sha256=hashlib.sha256(raw).hexdigest(),
                    intent_sha256=hashlib.sha256(intent_raw).hexdigest(),
                )
            ),
            check,
        )
        sealed = True
        result = inspect_capsule(job)
        if result["status"] != "sealed":
            raise CapsuleError("seal_validation_failed")
        result["durability"] = "directory_fsync_completed"
        emit(dict(stage="source_inputs_sealed", job_id=job.name))
        return result
    except (
        OSError,
        ValueError,
        TypeError,
        KeyError,
        RecursionError,
        OwnedGitError,
    ) as error:
        if sealed:
            return inspect_capsule(job)
        code = (
            str(error)
            if isinstance(error, (CapsuleError, OwnedGitError))
            else "input_or_io_unavailable"
        )
        if job is not None:
            try:
                atomic_file(
                    job,
                    "TERMINAL.json",
                    canonical(
                        dict(status="incomplete_or_uncertain", diagnostic_code=code)
                    ),
                    lambda: None,
                )
            except OSError:
                pass
            # A visible seal followed by failed fsync must not become a success.
            # Inspection never attests historical durability; TERMINAL preserves this known failure.
        return outcome(
            "incomplete_or_unavailable",
            source_materials_ready=False,
            diagnostic_code=code,
            job_id=job.name if job is not None else None,
        )
