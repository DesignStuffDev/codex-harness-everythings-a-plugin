"""Bounded source transformation artifacts; never a runnable or approved update."""

import hashlib
import json
import os
from pathlib import Path
import re
import time
import uuid

import owned_git
from source_capsule import CapsuleError, MAX_BLOB, MAX_BYTES, MAX_MANIFEST
from source_capsule import atomic_file, canonical, inspect_capsule, outcome
from source_capsule import regular_bytes, sync_directory

LIMITS = dict(
    max_changed_paths=1024, max_output_bytes=MAX_BYTES, max_manifest_bytes=MAX_MANIFEST
)
GATES = dict(
    candidate_build="pending",
    custom_plugin_runtime="pending",
    security_and_contract_review="pending",
    coordinated_versions="pending",
    state_migration_review="pending",
    ui_headless_regression="pending",
    external_bootstrap_rollback="pending",
)
VERSION = dict(
    package="0.3.0",
    component_api=1,
    tool_contract=1,
    source_capsule_schema="source-input-capsule-v1",
)
MAX_MERGE_INPUT = 128 * 1024


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def blob_identity(raw):
    return dict(
        sha256=digest(raw),
        bytes=len(raw),
        git_oid=hashlib.sha1(
            b"blob " + str(len(raw)).encode() + b"\0" + raw
        ).hexdigest(),
    )


def valid_limits(limits):
    return (
        isinstance(limits, dict)
        and set(limits) == set(LIMITS)
        and all(
            type(limits[k]) is int and 1 <= limits[k] <= v for k, v in LIMITS.items()
        )
    )


def classify(row):
    base, upstream, custom = (row[k] for k in ("base", "upstream", "custom"))
    if upstream is None:
        return "review_required", "upstream_deletion_requires_review", None
    if base is not None and custom is None:
        return "review_required", "custom_deletion_requires_review", None
    if base is not None and any(e["mode"] != base["mode"] for e in (upstream, custom)):
        return "review_required", "mode_change_requires_review", None
    if custom == base:
        return "choose_upstream", None, "upstream"
    if custom == upstream or upstream == base:
        return "unchanged", None, "custom"
    if base is None:
        return "review_required", "add_add_requires_review", None
    return "three_way_merge", None, None


def source_input(state, job_id, expected):
    if (
        not isinstance(job_id, str)
        or not re.fullmatch(r"[0-9a-f]{32}", job_id)
        or not isinstance(expected, str)
        or not re.fullmatch(r"[0-9a-f]{64}", expected)
    ):
        raise CapsuleError("invalid_capsule_binding")
    job = state / "upstream-capsules" / job_id
    verified = inspect_capsule(job)
    if verified["status"] != "sealed" or verified["manifest_sha256"] != expected:
        raise CapsuleError("source_capsule_unavailable_or_changed")
    raw = regular_bytes(job / "MANIFEST.json", MAX_MANIFEST)
    if digest(raw) != expected:
        raise CapsuleError("source_capsule_unavailable_or_changed")
    return job, json.loads(raw)


def inspect_overlay(job):
    """Current integrity only; no Git invocation or semantic compatibility claim."""
    try:
        job = Path(job).absolute()
        if (
            job.is_symlink()
            or not job.is_dir()
            or job.resolve() != job
            or job.parent.name != "upstream-overlays"
            or not re.fullmatch(r"[0-9a-f]{32}", job.name)
        ):
            raise CapsuleError("regular_overlay_job_required")
        if (job / "TERMINAL.json").exists() or not (job / "SEALED.json").exists():
            return outcome("incomplete_or_uncertain", overlay_complete=False)
        seal = json.loads(regular_bytes(job / "SEALED.json", 8192))
        raw = regular_bytes(job / "OVERLAY.json", MAX_MANIFEST)
        intent_raw = regular_bytes(job / "INTENT.json", 8192)
        if (
            seal["schema"] != "candidate-overlay-seal-v1"
            or seal["overlay_sha256"] != digest(raw)
            or seal["intent_sha256"] != digest(intent_raw)
        ):
            raise CapsuleError("corrupt_overlay")
        manifest, intent = json.loads(raw), json.loads(intent_raw)
        if (
            manifest["schema"] != "candidate-overlay-v1"
            or intent["schema"] != "candidate-overlay-intent-v1"
            or manifest["job_id"] != job.name
            or intent["job_id"] != job.name
            or manifest["release_gates"] != GATES
            or manifest["version_contract"] != VERSION
            or any(
                manifest[k] is not False
                for k in ("candidate_assembled", "update_allowed", "activation_allowed")
            )
            or manifest["application_base"] != "exact_committed_custom_tree"
            or manifest["untouched_paths"] != "inherited_by_reference_only"
            or not valid_limits(manifest["limits"])
            or len(raw) > manifest["limits"]["max_manifest_bytes"]
        ):
            raise CapsuleError("corrupt_overlay")
        for key in (
            "source_capsule_id",
            "source_manifest_sha256",
            "limits",
            "request_sha256",
        ):
            if manifest[key] != intent[key]:
                raise CapsuleError("corrupt_overlay")
        expected_request = dict(
            contract_version=1,
            purpose="prepare_candidate_overlay",
            source_capsule_id=manifest["source_capsule_id"],
            expected_manifest_sha256=manifest["source_manifest_sha256"],
            limits=manifest["limits"],
        )
        if manifest["request_sha256"] != digest(canonical(expected_request)):
            raise CapsuleError("corrupt_overlay")
        _, source = source_input(
            job.parent.parent,
            manifest["source_capsule_id"],
            manifest["source_manifest_sha256"],
        )
        if (
            manifest["commits"] != source["commits"]
            or manifest["trees"] != source["trees"]
            or manifest["source_review_sha256"] != digest(canonical(source["review"]))
            or len(manifest["paths"]) != len(source["paths"])
            or len(manifest["paths"]) > manifest["limits"]["max_changed_paths"]
            or len(manifest["artifacts"]) > 2 * len(source["paths"])
        ):
            raise CapsuleError("corrupt_overlay")
        available, total = {}, 0
        for item in manifest["artifacts"]:
            name = item["sha256"]
            if (
                not isinstance(name, str)
                or not re.fullmatch(r"[0-9a-f]{64}", name)
                or name in available
                or type(item["bytes"]) is not int
                or item["bytes"] < 0
            ):
                raise CapsuleError("corrupt_overlay")
            data = regular_bytes(job / "blobs" / name, 3 * MAX_BLOB)
            if blob_identity(data) != item:
                raise CapsuleError("corrupt_overlay")
            total += len(data)
            if total > manifest["limits"]["max_output_bytes"]:
                raise CapsuleError("budget_exceeded")
            available[name] = item
        used, unresolved = set(), 0
        for row, original in zip(manifest["paths"], source["paths"], strict=True):
            if any(
                row[key] != original[key]
                for key in ("path", "base", "upstream", "custom")
            ):
                raise CapsuleError("corrupt_overlay")
            strategy, reason, side = classify(original)
            output = row["output"]
            if output is None:
                unresolved += 1
                if (
                    row["strategy"] != "review_required"
                    or not isinstance(row["diagnostic_code"], str)
                    or not 1 <= len(row["diagnostic_code"]) <= 128
                ):
                    raise CapsuleError("corrupt_overlay")
                if (
                    strategy == "review_required"
                    and row["diagnostic_code"] != reason
                    or strategy not in ("review_required", "three_way_merge")
                ):
                    raise CapsuleError("corrupt_overlay")
                if strategy == "three_way_merge":
                    if row["diagnostic_code"] == "merge_not_clean":
                        if (
                            type(row["merge_returncode"]) is not int
                            or row["merge_returncode"] == 0
                            or row["diagnostic_artifact"] is None
                        ):
                            raise CapsuleError("corrupt_overlay")
                    elif (
                        row["diagnostic_code"]
                        not in (
                            "binary_merge_requires_review",
                            "merge_input_limit_requires_review",
                        )
                        or "merge_returncode" in row
                    ):
                        raise CapsuleError("corrupt_overlay")
            elif (
                strategy == "review_required"
                or row["strategy"] != strategy
                or row["diagnostic_code"] is not None
            ):
                raise CapsuleError("corrupt_overlay")
            if output is not None:
                record = {k: output[k] for k in ("sha256", "bytes", "git_oid")}
                if (
                    available.get(output["sha256"]) != record
                    or output["mode"] != original["upstream"]["mode"]
                    or (
                        side is not None and output["git_oid"] != original[side]["blob"]
                    )
                    or (
                        strategy == "three_way_merge"
                        and (
                            type(row["merge_returncode"]) is not int
                            or row["merge_returncode"] != 0
                        )
                    )
                ):
                    raise CapsuleError("corrupt_overlay")
                used.add(output["sha256"])
            diagnostic = row["diagnostic_artifact"]
            if row["diagnostic_code"] != "merge_not_clean" and diagnostic is not None:
                raise CapsuleError("corrupt_overlay")
            if diagnostic is not None:
                if (
                    output is not None
                    or available.get(diagnostic["sha256"]) != diagnostic
                ):
                    raise CapsuleError("corrupt_overlay")
                used.add(diagnostic["sha256"])
        if used != set(available) or manifest["status"] != (
            "review_required" if unresolved else "prepared"
        ):
            raise CapsuleError("corrupt_overlay")
        return outcome(
            manifest["status"],
            overlay_complete=not unresolved,
            source_transformed=bool(available),
            job_id=job.name,
            overlay_sha256=digest(raw),
            source_manifest_sha256=manifest["source_manifest_sha256"],
            unresolved_paths=unresolved,
            changed_paths=len(manifest["paths"]),
            integrity="verified",
            durability="not_attested_by_inspection",
            release_gates=GATES,
        )
    except (OSError, ValueError, TypeError, KeyError, AttributeError, RecursionError):
        return outcome("corrupt_or_unsupported", overlay_complete=False)


def prepare_overlay(request, state_dir, stop, emit=lambda event: None):
    job, sealed, deadline = None, False, time.monotonic() + 90

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
                "source_capsule_id",
                "expected_manifest_sha256",
                "limits",
            }
            or type(request["contract_version"]) is not int
            or request["contract_version"] != 1
            or request["purpose"] != "prepare_candidate_overlay"
            or not valid_limits(request["limits"])
        ):
            raise CapsuleError("invalid_request_or_contract")
        check()
        state = Path(state_dir).absolute()
        if state.is_symlink() or not state.is_dir() or state.resolve() != state:
            raise CapsuleError("regular_state_directory_required")
        source_job, source = source_input(
            state, request["source_capsule_id"], request["expected_manifest_sha256"]
        )
        if len(source["paths"]) > request["limits"]["max_changed_paths"]:
            raise CapsuleError("budget_exceeded")
        blobs = {item["git_oid"]: item for item in source["blobs"]}

        def read_blob(entry):
            item = blobs[entry["blob"]]
            raw = regular_bytes(source_job / "blobs" / item["sha256"], MAX_BLOB)
            if blob_identity(raw) != item:
                raise CapsuleError("source_blob_changed")
            return raw

        root = state / "upstream-overlays"
        root.mkdir(mode=0o700, exist_ok=True)
        if root.is_symlink() or not root.is_dir() or root.stat().st_mode & 0o077:
            raise CapsuleError("private_state_directory_required")
        candidate = root / uuid.uuid4().hex
        candidate.mkdir(mode=0o700)
        job = candidate
        sync_directory(state)
        sync_directory(root)
        binding = dict(
            job_id=job.name,
            source_capsule_id=request["source_capsule_id"],
            source_manifest_sha256=request["expected_manifest_sha256"],
            limits=request["limits"],
            request_sha256=digest(canonical(request)),
        )
        intent_raw = canonical(dict(schema="candidate-overlay-intent-v1", **binding))
        atomic_file(job, "INTENT.json", intent_raw, check)
        (job / "blobs").mkdir(mode=0o700)
        (job / "merge-inputs").mkdir(mode=0o700)
        emit(
            dict(
                stage="source_overlay_admitted",
                job_id=job.name,
                changed_paths=len(source["paths"]),
            )
        )
        artifacts, rows, output_bytes = {}, [], 0

        def store_blob(data):
            nonlocal output_bytes
            item = blob_identity(data)
            if len(data) > 3 * MAX_BLOB:
                raise CapsuleError("budget_exceeded")
            if item["sha256"] not in artifacts:
                output_bytes += len(data)
                if output_bytes > request["limits"]["max_output_bytes"]:
                    raise CapsuleError("budget_exceeded")
                atomic_file(job / "blobs", item["sha256"], data, check)
                artifacts[item["sha256"]] = item
            return item

        for original in source["paths"]:
            check()
            strategy, reason, side = classify(original)
            row = dict(
                original,
                strategy=strategy,
                diagnostic_code=reason,
                output=None,
                diagnostic_artifact=None,
            )
            if side is not None:
                row["output"] = dict(
                    mode=original[side]["mode"], **store_blob(read_blob(original[side]))
                )
            elif strategy == "three_way_merge":
                contents = {
                    key: read_blob(original[key])
                    for key in ("base", "upstream", "custom")
                }
                if any(b"\0" in data for data in contents.values()):
                    row.update(
                        strategy="review_required",
                        diagnostic_code="binary_merge_requires_review",
                    )
                elif any(len(data) > MAX_MERGE_INPUT for data in contents.values()):
                    row.update(
                        strategy="review_required",
                        diagnostic_code="merge_input_limit_requires_review",
                    )
                else:
                    # Reuse only this job's three scratch inputs. A cancelled
                    # atomic replacement adds at most one MAX_BLOB temporary.
                    scratch = job / "merge-inputs"
                    for key, data in contents.items():
                        atomic_file(scratch, key, data, check)
                    emit(
                        dict(
                            stage="source_overlay_merge_starting",
                            job_id=job.name,
                            path=original["path"],
                        )
                    )
                    merged = owned_git.capture(
                        [
                            "merge-file",
                            "--stdout",
                            "--diff3",
                            "-L",
                            "custom",
                            "-L",
                            "base",
                            "-L",
                            "upstream",
                            str(scratch / "custom"),
                            str(scratch / "base"),
                            str(scratch / "upstream"),
                        ],
                        env=dict(
                            os.environ,
                            GIT_OPTIONAL_LOCKS="0",
                            GIT_NO_LAZY_FETCH="1",
                            GIT_NO_REPLACE_OBJECTS="1",
                        ),
                        stop=stop,
                        deadline=deadline,
                    )
                    check()
                    if any(
                        regular_bytes(scratch / key, MAX_MERGE_INPUT) != data
                        for key, data in contents.items()
                    ):
                        raise CapsuleError("merge_input_changed")
                    row["merge_returncode"] = merged.returncode
                    if merged.returncode == 0:
                        row["output"] = dict(
                            mode=original["custom"]["mode"], **store_blob(merged.stdout)
                        )
                    else:
                        row.update(
                            strategy="review_required",
                            diagnostic_code="merge_not_clean",
                            diagnostic_artifact=store_blob(merged.stdout),
                        )
            rows.append(row)
        check()
        # A changed or terminal-marked source capsule invalidates this transformation.
        source_input(
            state, request["source_capsule_id"], request["expected_manifest_sha256"]
        )
        status = (
            "review_required"
            if any(row["output"] is None for row in rows)
            else "prepared"
        )
        manifest = dict(
            schema="candidate-overlay-v1",
            **binding,
            status=status,
            version_contract=VERSION,
            release_gates=GATES,
            commits=source["commits"],
            trees=source["trees"],
            source_review_sha256=digest(canonical(source["review"])),
            application_base="exact_committed_custom_tree",
            untouched_paths="inherited_by_reference_only",
            candidate_assembled=False,
            activation_allowed=False,
            update_allowed=False,
            paths=rows,
            artifacts=list(artifacts.values()),
        )
        raw = canonical(manifest)
        if len(raw) > request["limits"]["max_manifest_bytes"]:
            raise CapsuleError("budget_exceeded")
        atomic_file(job, "OVERLAY.json", raw, check)
        atomic_file(
            job,
            "SEALED.json",
            canonical(
                dict(
                    schema="candidate-overlay-seal-v1",
                    overlay_sha256=digest(raw),
                    intent_sha256=digest(intent_raw),
                )
            ),
            check,
        )
        sealed = True
        result = inspect_overlay(job)
        if result["status"] not in ("prepared", "review_required"):
            raise CapsuleError("seal_validation_failed")
        result["durability"] = "directory_fsync_completed"
        emit(
            dict(
                stage="source_overlay_sealed", job_id=job.name, status=result["status"]
            )
        )
        return result
    except (
        OSError,
        ValueError,
        TypeError,
        KeyError,
        RecursionError,
        owned_git.OwnedGitError,
    ) as error:
        if sealed:
            return inspect_overlay(job)
        code = (
            str(error)
            if isinstance(error, (CapsuleError, owned_git.OwnedGitError))
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
        return outcome(
            "incomplete_or_unavailable",
            overlay_complete=False,
            diagnostic_code=code,
            job_id=job.name if job is not None else None,
        )
