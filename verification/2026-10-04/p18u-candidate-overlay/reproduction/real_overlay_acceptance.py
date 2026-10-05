"""Opt-in installed 0.2 -> 0.3 upgrade and sparse source transformation acceptance.

No fetch, checkout, host build, live activation, or complete-candidate claim.
Run only under the existing bounded ownership/subreaper evidence wrapper.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import zipfile

BASE = "d42056091aded7feb1d88ac7e83972108b2aa478"
UPSTREAM = "2e5fea64eefcaa19f48458b2386011b619f69c70"
CUSTOM = "914cc59374c1149463e78bc33851d83e3f14d0a4"
HOST_SHA = "f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954"
OLD_PACKAGE_SHA = "2e5adafa9a231ace5b7687c685447afedc7c6f81b230373ab6b05581259b018f"
OLD_CAPSULE_SHA = "b44b3d6ae38f0af85b23bdc112aed7fff94180bf47c527aa1eaf07b8769f2e8c"
PLUGIN = "codex.maintenance.upstream-review"
REVIEW, CAPSULE, OVERLAY = "upstream_impact_review", "upstream_source_capsule", "upstream_candidate_overlay"
SCENARIOS = "codex-rs/core/tests/suite/scenarios.rs"
CUSTOM_REMOVAL = b"use codex_protocol::openai_models::ReasoningEffort;\n"
EXPECTED_PATHS = sorted(["codex-rs/core/" + name for name in (
    "config.schema.json", "src/context/world_state/mod.rs", "src/context/world_state/model_catalog.rs",
    "src/session/world_state.rs", "src/tools/handlers/multi_agents_spec.rs",
    "src/tools/handlers/multi_agents_spec_tests.rs", "src/tools/router.rs", "src/tools/spec_plan.rs",
    "tests/common/context_snapshot/normalize.rs", "tests/suite/scenarios.rs",
    "tests/suite/spawn_agent_description.rs",
    "tests/suite/snapshots/all__suite__scenarios__model_catalog_refresh_preserves_tools_and_history.snap",
)] + ["codex-rs/features/src/lib.rs"])


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


def fingerprint(path):
    before = path.stat()
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    after = path.stat()
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    assert all(getattr(before, key) == getattr(after, key) for key in fields)
    return dict(sha256=digest.hexdigest(), bytes=after.st_size,
                identity=[getattr(after, key) for key in fields])


def files(root):
    result = {}
    for path in sorted(root.rglob("*")):
        assert not path.is_symlink(), "unexpected artifact symlink"
        if path.is_file() and "__pycache__" not in path.parts and path.suffix != ".pyc":
            result[str(path.relative_to(root))] = fingerprint(path)
    return result


def compact(tree):
    return {name: {key: value[key] for key in ("sha256", "bytes")} for name, value in tree.items()}


def directories(root):
    return sorted(str(path.relative_to(root)) for path in root.rglob("*") if path.is_dir())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ("host", "sdk-python", "request", "work-dir", "project", "source-root", "old-package", "source-capsule"):
        parser.add_argument("--" + key, required=True, type=Path)
    args = parser.parse_args()
    host, project, source_root = (p.resolve(strict=True) for p in (args.host, args.project, args.source_root))
    original_package, original_capsule = (p.resolve(strict=True) for p in (args.old_package, args.source_capsule))
    work, python = args.work_dir.resolve(), args.sdk_python.absolute()
    for source in (project, source_root, original_package, original_capsule):
        assert work != source and source not in work.parents and work not in source.parents
    work.mkdir(mode=0o700, parents=True, exist_ok=False)
    request_raw = args.request.read_bytes()
    assert len(request_raw) <= 32768
    request = json.loads(request_raw)
    assert request["candidate_revision"] == UPSTREAM and request["composition_revision"] == CUSTOM
    inputs = {key: fingerprint(Path(request[key])) for key in ("lineage_index", "publication_receipt")}
    assert inputs["lineage_index"]["sha256"] == request["lineage_sha256"]
    assert inputs["publication_receipt"]["sha256"] == request["publication_sha256"]
    source_before, host_before = files(project), fingerprint(host)
    original_package_before, original_capsule_before = files(original_package), files(original_capsule)
    assert host_before["sha256"] == HOST_SHA
    assert original_package_before["plugin.pyz"]["sha256"] == OLD_PACKAGE_SHA
    assert original_capsule_before["MANIFEST.json"]["sha256"] == OLD_CAPSULE_SHA
    assert sum(value["bytes"] for value in original_capsule_before.values()) < 2 * 1024 * 1024
    environment = dict(os.environ, PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1",
                       GIT_OPTIONAL_LOCKS="0", GIT_NO_LAZY_FETCH="1", GIT_NO_REPLACE_OBJECTS="1")
    home, commands = work / "component-home", []
    manager = [host, "--codex-home", home]
    repository = Path(request["repository"])

    def run(label, argv, expected=0, env=environment, stderr_contains=None):
        result = subprocess.run([str(value) for value in argv], cwd=work, env=env,
                                capture_output=True, timeout=130, check=False)
        commands.append(dict(label=label, exit=result.returncode, stdout_sha256=sha(result.stdout),
                             stderr_sha256=sha(result.stderr), stdout_bytes=len(result.stdout), stderr_bytes=len(result.stderr)))
        assert max(len(result.stdout), len(result.stderr)) <= 8 * 1024 * 1024, label + ": output bound"
        assert result.returncode != 0 if expected is None else result.returncode == expected, label
        if stderr_contains is not None:
            assert stderr_contains in result.stderr, label + ": expected diagnostic"
        return result.stdout

    def git(*argv):
        return run("git " + str(argv[0]), ["git", "--no-pager", "-C", repository, *argv])

    def call(name, value, success):
        envelope = dict(call_id="real-overlay-acceptance", name=name, arguments=value)
        tool = json.loads(run(name, manager + ["call", "tool", name, "invoke", json.dumps(envelope)]))
        assert tool["success"] is success
        report = json.loads(tool["text"])
        assert report["update_allowed"] is False
        return report

    try:
        git_dir = Path(git("rev-parse", "--absolute-git-dir").decode().strip())

        def metadata():
            names = ["HEAD", "config", "index", "shallow", "FETCH_HEAD", "packed-refs"]
            names += [str(p.relative_to(git_dir)) for p in (git_dir / "refs").rglob("*") if p.is_file()]
            return {name: sha((git_dir / name).read_bytes()) if (git_dir / name).exists() else None for name in sorted(names)}

        metadata_before = metadata()
        assert git("show", "-s", "--format=%P", UPSTREAM).decode().strip() == BASE
        changed = sorted(p.decode() for p in git("diff-tree", "--no-commit-id", "--no-renames", "-r", "--name-only", "-z", BASE, UPSTREAM).split(b"\0") if p)
        assert changed == EXPECTED_PATHS
        source = json.loads((original_capsule / "MANIFEST.json").read_bytes())
        assert source["commits"] == dict(base=BASE, upstream=UPSTREAM, custom=CUSTOM)
        assert sorted(row["path"] for row in source["paths"]) == changed
        assert source["review"]["request_sha256"] == sha(canonical(request))
        entries, expected_blobs = {}, {}
        for side, revision in dict(base=BASE, upstream=UPSTREAM, custom=CUSTOM).items():
            assert source["trees"][side] == git("rev-parse", revision + "^{tree}").decode().strip()
            values = {}
            for row in git("ls-tree", "-r", "-z", revision, "--", *changed).split(b"\0"):
                if row:
                    attributes, name = row.split(b"\t", 1)
                    mode, kind, oid = attributes.decode().split()
                    values[name.decode()] = dict(mode=mode, type=kind, blob=oid)
            entries[side] = values
        assert source["paths"] == [dict(path=path, **{side: rows.get(path) for side, rows in entries.items()}) for path in changed]
        for blob in source["blobs"]:
            raw = git("cat-file", "blob", blob["git_oid"])
            assert raw == (original_capsule / "blobs" / blob["sha256"]).read_bytes()
            assert sha(raw) == blob["sha256"] and len(raw) == blob["bytes"]
            expected_blobs[blob["git_oid"]] = raw
        assert set(expected_blobs) == {value["blob"] for rows in entries.values() for value in rows.values()}
        expected_outputs = {}
        for path in changed:
            upstream = expected_blobs[entries["upstream"][path]["blob"]]
            base, custom = (expected_blobs[entries[side][path]["blob"]] if path in entries[side] else None
                            for side in ("base", "custom"))
            if path == SCENARIOS:
                assert base.count(CUSTOM_REMOVAL) == upstream.count(CUSTOM_REMOVAL) == 1
                assert custom == base.replace(CUSTOM_REMOVAL, b"", 1)
                expected_outputs[path] = upstream.replace(CUSTOM_REMOVAL, b"", 1)
            else:
                assert entries["base"].get(path) == entries["custom"].get(path) and base == custom
                assert entries["upstream"][path]["type"] == "blob"
                expected_outputs[path] = upstream
        sdk = json.loads(run("installed SDK identity", [python, "-I", "-B", "-c",
            "import codex_component_sdk,importlib.metadata,json;print(json.dumps({'path':codex_component_sdk.__file__,'version':importlib.metadata.version('codex-component-sdk')}))"]))
        assert sdk["version"] == "0.1.0"
        assert source_root not in Path(sdk["path"]).resolve().parents and project not in Path(sdk["path"]).resolve().parents
        external, package = work / "external-project", work / "package"
        shutil.copytree(project, external, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        run("external 0.3 package build", [python, "-I", "-B", external / "build_package.py", "--output", package])
        shutil.rmtree(external)
        assert not external.exists()
        package_before = files(package)
        new_declaration = json.loads((package / "codex-component.json").read_bytes())
        old_declaration = json.loads((original_package / "codex-component.json").read_bytes())
        assert old_declaration["id"] == new_declaration["id"] == PLUGIN
        assert old_declaration["api_version"] == new_declaration["api_version"] == 1
        assert old_declaration["version"] == "0.2.0" and new_declaration["version"] == "0.3.0"
        expected_components = {REVIEW: 1, CAPSULE: 1, OVERLAY: 1}
        assert {item["name"]: item["contract_version"] for item in new_declaration["components"]} == expected_components
        with zipfile.ZipFile(package / "plugin.pyz") as archive:
            members = {name: dict(sha256=sha(archive.read(name)), bytes=len(archive.read(name))) for name in archive.namelist() if not name.endswith("/")}
            for item in json.loads(archive.read("static/TOOL_PROVENANCE.json"))["tools"]:
                assert members[item["packaged_path"]] == {key: item[key] for key in ("sha256", "bytes")}
        assert run("install existing 0.2", manager + ["install", original_package]).decode().strip() == PLUGIN
        old_list = json.loads(run("list existing 0.2", manager + ["list"]))
        old_object = home / "components" / "objects" / old_list["installed"][PLUGIN]
        old_object_before = files(old_object)
        assert compact(old_object_before) == compact(original_package_before)
        state = home / "components" / "state" / PLUGIN
        state.mkdir(mode=0o700, parents=True)
        capsule_root = state / "upstream-capsules"
        capsule_root.mkdir(mode=0o700)
        restored = capsule_root / original_capsule.name
        shutil.copytree(original_capsule, restored)
        assert compact(files(restored)) == compact(original_capsule_before)
        restored_before = files(restored)
        empty_path = work / "empty-path"
        empty_path.mkdir()
        offline_environment = dict(environment, PATH=str(empty_path))
        assert shutil.which("git", path=str(empty_path)) is None
        assert shutil.which("codex-component", path=str(empty_path)) is None
        old_inspect = json.loads(run("old package inspects restored capsule", [python, "-I", "-B", old_object / "capsule_bootstrap.py", restored], env=offline_environment))
        assert old_inspect["status"] == "sealed" and old_inspect["manifest_sha256"] == OLD_CAPSULE_SHA
        old_review = call(REVIEW, request, True)
        assert old_review["plan_id"] == source["review"]["plan_id"]
        before_upgrade = files(home)
        run("duplicate install rejected before explicit upgrade", manager + ["install", package], expected=None,
            stderr_contains=b"plugin is already installed")
        assert files(home) == before_upgrade
        run("upgrade step 1 remove old activation", manager + ["remove", PLUGIN])
        assert files(restored) == restored_before and files(old_object) == old_object_before
        inactive = json.loads(run("upgrade inactive intermediate state", manager + ["list"]))
        assert inactive["enabled"] == [] and inactive["installed"] == {}
        assert run("upgrade step 2 install 0.3", manager + ["install", package]).decode().strip() == PLUGIN
        new_list = json.loads(run("list upgraded 0.3", manager + ["list"]))
        new_object = home / "components" / "objects" / new_list["installed"][PLUGIN]
        new_object_before = files(new_object)
        assert new_list["enabled"] == [PLUGIN] and new_object != old_object
        assert compact(new_object_before) == compact(package_before)
        assert json.loads((new_object / "codex-component.json").read_bytes()) == new_declaration
        assert files(restored) == restored_before and files(old_object) == old_object_before
        new_review = call(REVIEW, request, True)
        assert new_review == old_review
        source_request = dict(contract_version=1, purpose="prepare_source_inputs", review_request=request,
            expected_plan_id=new_review["plan_id"], limits=dict(max_changed_paths=13,
            max_unique_blob_bytes=64 * 1024 * 1024, max_manifest_bytes=8 * 1024 * 1024))
        fresh = call(CAPSULE, source_request, True)
        assert fresh["status"] == "sealed" and fresh["durability"] == "directory_fsync_completed"
        fresh_job = capsule_root / fresh["job_id"]
        assert fresh_job != restored
        fresh_source = json.loads((fresh_job / "MANIFEST.json").read_bytes())
        assert sha((fresh_job / "MANIFEST.json").read_bytes()) == fresh["manifest_sha256"]
        for key in ("paths", "blobs", "trees", "commits", "review"):
            assert fresh_source[key] == source[key], "old source-capsule contract changed: " + key
        value = dict(contract_version=1, purpose="prepare_candidate_overlay", source_capsule_id=restored.name,
                     expected_manifest_sha256=OLD_CAPSULE_SHA,
                     limits=dict(max_changed_paths=13, max_output_bytes=64 * 1024 * 1024, max_manifest_bytes=8 * 1024 * 1024))
        result = call(OVERLAY, value, True)
        assert result["status"] == "prepared" and result["durability"] == "directory_fsync_completed"
        assert result["changed_paths"] == 13 and result["unresolved_paths"] == 0
        assert result["candidate_assembled"] is result["activation_allowed"] is result["self_contained_candidate"] is False
        assert result["release_gates"] == {name: "pending" for name in (
            "candidate_build", "custom_plugin_runtime", "security_and_contract_review", "coordinated_versions",
            "state_migration_review", "ui_headless_regression", "external_bootstrap_rollback")}
        job = state / "upstream-overlays" / result["job_id"]
        raw = (job / "OVERLAY.json").read_bytes()
        overlay = json.loads(raw)
        assert sha(raw) == result["overlay_sha256"]
        assert overlay["source_manifest_sha256"] == OLD_CAPSULE_SHA and overlay["source_capsule_id"] == restored.name
        assert overlay["commits"] == source["commits"] and overlay["trees"] == source["trees"]
        assert overlay["request_sha256"] == sha(canonical(value))
        assert overlay["application_base"] == "exact_committed_custom_tree" and overlay["untouched_paths"] == "inherited_by_reference_only"
        assert overlay["version_contract"] == dict(package="0.3.0", component_api=1, tool_contract=1, source_capsule_schema="source-input-capsule-v1")
        assert [row["path"] for row in overlay["paths"]] == changed
        output_evidence = []
        for row, original in zip(overlay["paths"], source["paths"], strict=True):
            assert {key: row[key] for key in original} == original
            expected = expected_outputs[row["path"]]
            actual = (job / "blobs" / row["output"]["sha256"]).read_bytes()
            expected_record = dict(mode=entries["upstream"][row["path"]]["mode"], sha256=sha(expected), bytes=len(expected),
                git_oid=hashlib.sha1(b"blob " + str(len(expected)).encode() + b"\0" + expected).hexdigest())
            assert actual == expected and row["output"] == expected_record
            assert row["strategy"] == ("three_way_merge" if row["path"] == SCENARIOS else "choose_upstream")
            assert row["diagnostic_code"] is row["diagnostic_artifact"] is None
            if row["path"] == SCENARIOS:
                assert row["merge_returncode"] == 0 and CUSTOM_REMOVAL not in actual
            output_evidence.append(dict(path=row["path"], expected=expected_record, strategy=row["strategy"], exact_bytes_equal=True))
        assert files(restored) == restored_before
        retained_state = files(state)
        retained_directories = directories(state)
        negative = []
        for label, bad, diagnostic in (
            ("incompatible tool contract", dict(value, contract_version=2), "invalid_request_or_contract"),
            ("malformed capsule binding", dict(value, source_capsule_id="../outside"), "invalid_capsule_binding"),
            ("stale capsule binding", dict(value, expected_manifest_sha256="0" * 64), "source_capsule_unavailable_or_changed"),
        ):
            rejected = call(OVERLAY, bad, False)
            assert rejected["status"] == "incomplete_or_unavailable" and rejected["job_id"] is None
            assert rejected["diagnostic_code"] == diagnostic
            assert files(state) == retained_state and directories(state) == retained_directories
            negative.append(dict(case=label, result=rejected))
        incompatible = work / "incompatible-package"
        shutil.copytree(package, incompatible)
        bad_manifest = dict(new_declaration, api_version=999)
        (incompatible / "codex-component.json").write_bytes(canonical(bad_manifest))
        home_before_rejection = files(home)
        run("incompatible component API rejected", manager + ["install", incompatible], expected=None,
            stderr_contains=b"incompatible component API version")
        assert files(home) == home_before_rejection
        offline = json.loads(run("external overlay inspection", [python, "-I", "-B", package / "overlay_bootstrap.py", job], env=offline_environment))
        assert offline == dict(result, durability="not_attested_by_inspection")
        assert json.loads(run("retained old package still inspects original state", [python, "-I", "-B", old_object / "capsule_bootstrap.py", restored], env=offline_environment)) == old_inspect
        run("remove upgraded activation", manager + ["remove", PLUGIN])
        removed_list = json.loads(run("list after removal", manager + ["list"]))
        assert removed_list["enabled"] == [] and removed_list["installed"] == {}
        for name, arguments in ((REVIEW, request), (CAPSULE, source_request), (OVERLAY, value)):
            envelope = json.dumps(dict(call_id="removed", name=name, arguments=arguments))
            run("removed " + name, manager + ["call", "tool", name, "invoke", envelope], expected=None,
                stderr_contains=b"requested component is not installed and enabled")
        assert files(state) == retained_state and directories(state) == retained_directories
        assert json.loads(run("external overlay inspection after removal", [python, "-I", "-B", package / "overlay_bootstrap.py", job], env=offline_environment)) == offline
        assert json.loads(run("old capsule inspection after removal", [python, "-I", "-B", original_package / "capsule_bootstrap.py", restored], env=offline_environment)) == old_inspect
        assert files(original_package) == original_package_before and files(original_capsule) == original_capsule_before
        assert files(old_object) == old_object_before and files(new_object) == new_object_before
        assert files(package) == package_before and files(project) == source_before
        assert metadata() == metadata_before and fingerprint(host) == host_before
        assert {key: fingerprint(Path(request[key])) for key in inputs} == inputs
        retained_bytes = sum(item["bytes"] for item in files(work).values())
        assert retained_bytes < 16 * 1024 * 1024
        evidence = dict(passed=True, host_before=host_before, host_after=fingerprint(host), new_host_build_performed=False,
            source_project=source_before, package_files=package_before, package_members=members, sdk=sdk, inputs=inputs,
            request_sha256=sha(request_raw), commits=source["commits"], trees=source["trees"], exact_changed_paths=changed,
            restored_state_origin=dict(path=str(original_capsule), files=original_capsule_before, manifest_sha256=OLD_CAPSULE_SHA),
            original_artifacts_unchanged=True, temporary_build_copy_removed_before_install=True,
            upgrade=dict(from_version="0.2.0", to_version="0.3.0", method="remove_activation_then_install_same_id",
                atomic=False, old_object=str(old_object), new_object=str(new_object), old_object_retained=True,
                restored_state_retained=True, restored_state_is_copy=True, old_package_files=original_package_before,
                old_activation=old_list, new_activation=new_list, capabilities=expected_components),
            old_contract_review=new_review, old_contract_fresh_capsule=fresh, overlay_result=result, overlay_path=str(job),
            oracle="12 exact chosen-upstream blobs; scenarios equals chosen upstream minus the single exact ReasoningEffort import, independently attested against base/custom bytes",
            outputs=output_evidence, overlay_manifest=overlay, state_files=retained_state, state_directories=retained_directories,
            negative_real_input_cases=negative, incompatible_component_api_rejected=True,
            offline_result=offline, retained_after_removal=True, repository_metadata_unchanged=True,
            retained_work_bytes=retained_bytes, commands=commands,
            limits="Sparse transformation and restored-state package upgrade only; no full candidate, integrated host build, active merge cancellation, state migration, UI regression, deployment or failed-update rollback proof.")
        report = json.dumps(evidence, indent=2).encode() + b"\n"
        assert len(report) < 1024 * 1024
        (work / "acceptance.json").write_bytes(report)
        print(work / "acceptance.json")
    except Exception as error:
        (work / "acceptance-failure.json").write_text(json.dumps(dict(passed=False, failure_type=type(error).__name__, commands=commands), indent=2) + "\n")
        raise


if __name__ == "__main__":
    main()
