"""Opt-in real installed source-capsule acceptance; no fetch, checkout or activation."""

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
PLUGIN = "codex.maintenance.upstream-review"
REVIEW, CAPSULE = "upstream_impact_review", "upstream_source_capsule"
PREFIX = "codex-rs/core/"
EXPECTED_PATHS = sorted([PREFIX + p for p in [
    "config.schema.json", "src/context/world_state/mod.rs",
    "src/context/world_state/model_catalog.rs", "src/session/world_state.rs",
    "src/tools/handlers/multi_agents_spec.rs", "src/tools/handlers/multi_agents_spec_tests.rs",
    "src/tools/router.rs", "src/tools/spec_plan.rs", "tests/common/context_snapshot/normalize.rs",
    "tests/suite/scenarios.rs", "tests/suite/spawn_agent_description.rs",
    "tests/suite/snapshots/all__suite__scenarios__model_catalog_refresh_preserves_tools_and_history.snap",
]] + ["codex-rs/features/src/lib.rs"])


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
    assert all(getattr(before, f) == getattr(after, f) for f in fields)
    return {"sha256": digest.hexdigest(), "bytes": after.st_size,
            "identity": [getattr(after, f) for f in fields]}


def files(root):
    result = {}
    for path in sorted(root.rglob("*")):
        assert not path.is_symlink(), "unexpected artifact symlink"
        if path.is_file() and "__pycache__" not in path.parts and path.suffix != ".pyc":
            result[str(path.relative_to(root))] = fingerprint(path)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ("host", "sdk-python", "request", "work-dir", "project", "source-root"):
        parser.add_argument("--" + key, required=True, type=Path)
    args = parser.parse_args()
    host, project = args.host.resolve(strict=True), args.project.resolve(strict=True)
    work, python = args.work_dir.resolve(strict=False), args.sdk_python.absolute()
    source_root = args.source_root.resolve(strict=True)
    assert work != project and project not in work.parents
    assert work != source_root and source_root not in work.parents
    work.mkdir(mode=0o700, parents=True, exist_ok=False)
    request_raw = args.request.read_bytes()
    assert len(request_raw) <= 32768
    request = json.loads(request_raw)
    assert request["candidate_revision"] == UPSTREAM and request["composition_revision"] == CUSTOM
    assert json.loads(Path(request["lineage_index"]).read_bytes())["source"]["upstream_revision"] == BASE
    inputs = {key: fingerprint(Path(request[key])) for key in ("lineage_index", "publication_receipt") if key in request}
    assert inputs["lineage_index"]["sha256"] == request["lineage_sha256"]
    if "publication_receipt" in inputs:
        assert inputs["publication_receipt"]["sha256"] == request["publication_sha256"]
    source_before, host_before = files(project), fingerprint(host)
    environment = dict(os.environ, PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1",
                       GIT_OPTIONAL_LOCKS="0", GIT_NO_LAZY_FETCH="1", GIT_NO_REPLACE_OBJECTS="1")
    commands = []
    home = work / "component-home"
    manager = [host, "--codex-home", home]
    repository = Path(request["repository"])

    def run(label, argv, expected=0, env=environment, limit=8 * 1024 * 1024):
        result = subprocess.run([str(a) for a in argv], cwd=work, env=env,
                                capture_output=True, timeout=130, check=False)
        commands.append(dict(label=label, exit=result.returncode, stdout_sha256=sha(result.stdout),
                             stderr_sha256=sha(result.stderr), stdout_bytes=len(result.stdout)))
        assert len(result.stdout) <= limit, label + ": output bound"
        assert result.returncode != 0 if expected is None else result.returncode == expected, label
        return result.stdout

    def git(*argv):
        return run("git " + str(argv[0]), ["git", "--no-pager", "-C", repository, *argv])

    def call(name, value, success):
        envelope = {"call_id": "real-capsule-acceptance", "name": name, "arguments": value}
        tool = json.loads(run(name, manager + ["call", "tool", name, "invoke", json.dumps(envelope)], limit=32768))
        assert tool["success"] is success
        report = json.loads(tool["text"])
        assert report["update_allowed"] is False
        return report

    try:
        git_dir = Path(git("rev-parse", "--absolute-git-dir").decode().strip())
        def metadata():
            names = ["HEAD", "config", "index", "shallow", "FETCH_HEAD", "packed-refs"]
            names += [str(p.relative_to(git_dir)) for p in (git_dir / "refs").rglob("*") if p.is_file()]
            return {n: sha((git_dir / n).read_bytes()) if (git_dir / n).exists() else None for n in sorted(names)}
        metadata_before = metadata()
        assert git("show", "-s", "--format=%P", UPSTREAM).decode().strip() == BASE
        changed = sorted(p.decode() for p in git("diff-tree", "--no-commit-id", "--no-renames", "-r", "--name-only", "-z", BASE, UPSTREAM).split(b"\0") if p)
        assert changed == EXPECTED_PATHS
        trees = {side: git("rev-parse", revision + "^{tree}").decode().strip()
                 for side, revision in {"base": BASE, "upstream": UPSTREAM, "custom": CUSTOM}.items()}
        entries = {}
        for side, revision in {"base": BASE, "upstream": UPSTREAM, "custom": CUSTOM}.items():
            values = {}
            for row in git("ls-tree", "-r", "-z", revision, "--", *changed).split(b"\0"):
                if row:
                    attributes, name = row.split(b"\t", 1)
                    mode, kind, oid = attributes.decode().split()
                    values[name.decode()] = {"mode": mode, "type": kind, "blob": oid}
            entries[side] = values
        expected_rows = [dict(path=p, **{side: values.get(p) for side, values in entries.items()}) for p in changed]
        sdk = json.loads(run("installed sdk", [python, "-I", "-B", "-c",
            "import codex_component_sdk,importlib.metadata,json; print(json.dumps({'path':codex_component_sdk.__file__,'version':importlib.metadata.version('codex-component-sdk')}))"]))
        assert project not in Path(sdk["path"]).resolve().parents
        assert source_root not in Path(sdk["path"]).resolve().parents
        external, package = work / "external-project", work / "package"
        shutil.copytree(project, external, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        run("external package build", [python, "-I", "-B", external / "build_package.py", "--output", package])
        shutil.rmtree(external)
        assert not external.exists()
        package_before = files(package)
        declaration = json.loads((package / "codex-component.json").read_bytes())
        assert declaration["id"] == PLUGIN and declaration["version"] == "0.2.0"
        assert {c["name"] for c in declaration["components"]} == {REVIEW, CAPSULE}
        with zipfile.ZipFile(package / "plugin.pyz") as archive:
            members = {n: {"sha256": sha(archive.read(n)), "bytes": len(archive.read(n))} for n in archive.namelist() if not n.endswith("/")}
            provenance = json.loads(archive.read("static/TOOL_PROVENANCE.json"))
        for item in provenance["tools"]:
            assert members[item["packaged_path"]] == {"sha256": item["sha256"], "bytes": item["bytes"]}
        assert run("install", manager + ["install", package]).decode().strip() == PLUGIN
        listed = json.loads(run("list", manager + ["list"]))
        assert listed["enabled"] == [PLUGIN]
        installed = home / "components" / "objects" / listed["installed"][PLUGIN]
        installed_before = files(installed)
        compact = lambda tree: {p: {k: v[k] for k in ("sha256", "bytes")} for p, v in tree.items()}
        assert compact(installed_before) == compact(package_before)
        short = call(REVIEW, request, True)
        request_file = work / "request.json"
        request_file.write_bytes(request_raw)
        full = json.loads(run("full review bootstrap", [python, "-I", "-B", package / "bootstrap.py", request_file, "--full-report"], expected=2))
        assert full["status"] == "review_required" and not full.get("detailed_report_omitted")
        for key in ("plan_id", "request_sha256", "report_sha256", "planner_sha256", "validator_sha256", "candidate_revision", "composition_revision", "index_sha256"):
            assert short[key] == full[key], key
        assert full["request_sha256"] == sha(canonical(request))
        assert full["report_sha256"] == sha(canonical(full["report"]))
        assert full["index_sha256"] == request["lineage_sha256"]
        assert full["planner_sha256"] == members["plan_upstream_impact.py"]["sha256"]
        assert full["validator_sha256"] == members["validate_checkpoint_lineage.py"]["sha256"]
        binding = {k: full[k] for k in ("request_sha256", "report_sha256", "planner_sha256", "validator_sha256", "contract_version")}
        assert full["plan_id"] == sha(canonical(binding))
        assert "lineage_does_not_cover_requested_composition" in full["report"]["unresolved"]
        (work / "full-review.json").write_bytes(canonical(full))
        value = dict(contract_version=1, purpose="prepare_source_inputs", review_request=request,
                     expected_plan_id=full["plan_id"], limits=dict(max_changed_paths=13,
                     max_unique_blob_bytes=64 * 1024 * 1024, max_manifest_bytes=8 * 1024 * 1024))
        result = call(CAPSULE, value, True)
        assert result["status"] == "sealed" and result["durability"] == "directory_fsync_completed"
        root = home / "components" / "state" / PLUGIN / "upstream-capsules"
        job = root / result["job_id"]
        manifest_raw = (job / "MANIFEST.json").read_bytes()
        manifest = json.loads(manifest_raw)
        assert sha(manifest_raw) == result["manifest_sha256"]
        assert manifest["paths"] == expected_rows and manifest["trees"] == trees
        assert manifest["commits"] == dict(base=BASE, upstream=UPSTREAM, custom=CUSTOM)
        assert manifest["review"] == full and manifest["gate_inventory_complete"] is False
        assert manifest["candidate_assembled"] is False and manifest["activation_allowed"] is False
        assert manifest["request_sha256"] == sha(canonical(value))
        expected_oids = {entry["blob"] for values in entries.values() for entry in values.values()}
        assert {b["git_oid"] for b in manifest["blobs"]} == expected_oids
        assert len(manifest["blobs"]) == len(expected_oids)
        for blob in manifest["blobs"]:
            expected = git("cat-file", "blob", blob["git_oid"])
            actual = (job / "blobs" / blob["sha256"]).read_bytes()
            assert actual == expected and len(actual) == blob["bytes"] and sha(actual) == blob["sha256"]
            assert hashlib.sha1(b"blob " + str(len(actual)).encode() + b"\0" + actual).hexdigest() == blob["git_oid"]
        assert len(expected_rows) == result["changed_paths"] == 13
        customized = [r for r in expected_rows if r["base"] != r["custom"]]
        assert [r["path"] for r in customized] == [PREFIX + "tests/suite/scenarios.rs"]
        assert customized[0]["custom"]["blob"] == "c7877bb57993107e256a89d80046e8b8cd94975a"
        retained = files(root)
        directories = lambda: sorted(str(p.relative_to(root)) for p in root.rglob("*") if p.is_dir())
        retained_directories = directories()
        failures = []
        for label, bad in [("stale plan", dict(value, expected_plan_id="0" * 64)),
                           ("preflight budget", dict(value, limits=dict(value["limits"], max_changed_paths=12)))]:
            rejected = call(CAPSULE, bad, False)
            assert rejected["status"] == "incomplete_or_unavailable" and rejected["job_id"] is None
            assert rejected["diagnostic_code"] == ("plan_changed_or_unavailable" if label == "stale plan" else "budget_exceeded")
            assert files(root) == retained and directories() == retained_directories, label + ": capsule state changed"
            failures.append(dict(case=label, result=rejected))
        empty_path = work / "empty-path"
        empty_path.mkdir()
        offline_environment = dict(environment, PATH=str(empty_path))
        assert shutil.which("git", path=str(empty_path)) is None and shutil.which("codex-component", path=str(empty_path)) is None
        offline = json.loads(run("offline capsule inspection", [python, "-I", "-B", package / "capsule_bootstrap.py", job], env=offline_environment))
        assert offline == dict(result, durability="not_attested_by_inspection")
        run("remove", manager + ["remove", PLUGIN])
        assert json.loads(run("list after removal", manager + ["list"]))["enabled"] == []
        for name in (REVIEW, CAPSULE):
            envelope = json.dumps(dict(call_id="removed", name=name, arguments=request if name == REVIEW else value))
            run("removed " + name, manager + ["call", "tool", name, "invoke", envelope], expected=None)
        assert files(root) == retained and directories() == retained_directories
        assert json.loads(run("offline inspection after removal", [python, "-I", "-B", package / "capsule_bootstrap.py", job], env=offline_environment)) == offline
        assert files(package) == package_before and files(project) == source_before
        assert files(installed) == installed_before
        assert metadata() == metadata_before and fingerprint(host) == host_before
        assert {key: fingerprint(Path(request[key])) for key in inputs} == inputs
        evidence = dict(passed=True, host_before=host_before, host_after=fingerprint(host),
            new_host_build_performed=False, source_project=source_before, package_files=package_before,
            package_members=members, installed_files=installed_before, sdk=sdk, inputs=inputs, request_sha256=sha(request_raw),
            commits=manifest["commits"], trees=trees, exact_changed_paths=changed, unique_blobs=manifest["blobs"],
            short_review=short, full_review_sha256=sha(canonical(full)), capsule_result=result,
            job_path=str(job), capsule_files=retained, retained_directories=retained_directories,
            offline_result=offline, negative_real_input_cases=failures,
            temporary_build_copy_removed_before_install=True, package_version=declaration["version"],
            custom_difference_rows=customized,
            repository_metadata_unchanged=True, retained_after_removal=True, commands=commands,
            limits="Real chosen-revision source artifacts only; no fixture fault/cancellation tests in this helper; no candidate assembly, engine/UI regression, deployment, migration or rollback proof.")
        (work / "acceptance.json").write_text(json.dumps(evidence, indent=2) + "\n")
        print(work / "acceptance.json")
    except Exception as error:
        (work / "acceptance-failure.json").write_text(json.dumps(dict(passed=False, failure_type=type(error).__name__, commands=commands), indent=2) + "\n")
        raise


if __name__ == "__main__":
    main()
