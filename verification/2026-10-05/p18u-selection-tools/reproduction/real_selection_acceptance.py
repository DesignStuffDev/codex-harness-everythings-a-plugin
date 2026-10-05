"""Opt-in bounded installed selection planning on the unchanged component manager.

Run only beneath the root's existing source-binding/strict subreaper wrapper.
Synthetic policy cases are not resolved real Codex ownership or update approval.
"""

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import zipfile

HOST_SHA = "f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954"
OLD_PACKAGE_SHA = "4a860c3c55d69bbfe3554cb3a1812d3bb797dfde3fd02dd9e05d074c98c681bf"
CAPSULE_SHA = "b44b3d6ae38f0af85b23bdc112aed7fff94180bf47c527aa1eaf07b8769f2e8c"
OVERLAY_SHA = "81fb203ded8a7dc3dc87b3a8938e8f946eccf147d9306c4f24b63874bae4e1c2"
PLUGIN = "codex.maintenance.upstream-review"
REVIEW, CAPSULE, OVERLAY = "upstream_impact_review", "upstream_source_capsule", "upstream_candidate_overlay"
PLAN, RESTORE = "upstream_selection_plan", "upstream_selection_restore"


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def hashed(value):
    return digest(canonical(value))


def fingerprint(path):
    before = path.stat()
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    after = path.stat()
    keys = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    assert all(getattr(before, key) == getattr(after, key) for key in keys)
    return dict(sha256=value.hexdigest(), bytes=after.st_size,
                identity=[getattr(after, key) for key in keys])


def files(root):
    result = {}
    for path in sorted(root.rglob("*")):
        assert not path.is_symlink(), "unexpected source/artifact symlink"
        if path.is_file() and "__pycache__" not in path.parts and path.suffix != ".pyc":
            result[str(path.relative_to(root))] = fingerprint(path)
    return result


def compact(values):
    return {name: {key: value[key] for key in ("sha256", "bytes")} for name, value in values.items()}


def metadata(git_dir):
    names = ["HEAD", "config", "index", "shallow", "FETCH_HEAD", "packed-refs"]
    names += [str(path.relative_to(git_dir)) for path in (git_dir / "refs").rglob("*") if path.is_file()]
    return {name: fingerprint(git_dir / name) if (git_dir / name).exists() else None for name in sorted(names)}


def synthetic():
    review = dict(schema="codex-upstream-offline-impact-v1", status="unresolved", unresolved=[],
                  changed_paths=[dict(path=name + ".rs", unresolved=[]) for name in ("a", "b")],
                  summary=dict(changed_paths_total=2, changed_paths_reported=2, path_inventory_complete=True))
    graph = dict(contract_version=1, unresolved=[], units=[], changes=[], bindings=dict(
        base_revision="a" * 40, upstream_revision="b" * 40, custom_revision="c" * 40,
        composition_sha256="d" * 64, before_inventory_sha256="e" * 64,
        lineage_sha256="f" * 64, review_sha256=hashed(review), capsule_sha256="0" * 64))
    scopes = dict(contract_version=1, id="fixture-scopes", revision=1, rules=[])
    for name in ("a", "b"):
        graph["units"].append(dict(id="component:" + name, members=[], requires=[], conflicts=[], unresolved=[]))
        row = dict(id=name, unit="component:" + name, plugins=["plugin." + name], path=name + ".rs",
                   category="implementation", before=dict(blob=name * 40, mode="100644"),
                   origin=dict(revision="a" * 40, path=name + ".rs", blob=name * 40, symbol="fn fixture"), unresolved=[])
        graph["changes"].append(row)
        scopes["rules"].append(dict(plugin="plugin." + name, change=name, path=row["path"],
            category=row["category"], before=copy.deepcopy(row["before"]), origin_sha256=hashed(row["origin"]),
            operations=["modify"], granularity="file"))
    profile = dict(contract_version=1, id="fixture-remembered-choice", revision=1,
                   catalog_sha256=hashed(graph), include=["component:a"], exclude=["component:b"])
    return dict(contract_version=1, purpose="plan_selection", graph=graph, profile=profile, policy=scopes, review=review)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ("host", "sdk-python", "request", "work-dir", "project", "source-root", "old-package", "source-capsule", "source-overlay"):
        parser.add_argument("--" + key, required=True, type=Path)
    args = parser.parse_args()
    host, project, source_root, old, capsule, overlay = (path.resolve(strict=True) for path in (
        args.host, args.project, args.source_root, args.old_package, args.source_capsule, args.source_overlay))
    work, python = args.work_dir.resolve(), args.sdk_python.absolute()
    assert Path("/tmp") in work.parents and not work.exists()
    for path in (host, project, source_root, old, capsule, overlay):
        assert path != work and path not in work.parents and work not in path.parents
    source_before, host_before = files(project), fingerprint(host)
    old_before, capsule_before, overlay_before = files(old), files(capsule), files(overlay)
    assert host_before["sha256"] == HOST_SHA and old_before["plugin.pyz"]["sha256"] == OLD_PACKAGE_SHA
    assert capsule_before["MANIFEST.json"]["sha256"] == CAPSULE_SHA
    assert overlay_before["OVERLAY.json"]["sha256"] == OVERLAY_SHA
    assert sum(value["bytes"] for tree in (source_before, old_before, capsule_before, overlay_before) for value in tree.values()) < 4 * 1024 * 1024
    request = json.loads(args.request.read_bytes())
    inputs = {str(path): fingerprint(path) for path in (args.request, Path(request["lineage_index"]), Path(request["publication_receipt"]))}
    assert inputs[str(Path(request["lineage_index"]))]["sha256"] == request["lineage_sha256"]
    assert inputs[str(Path(request["publication_receipt"]))]["sha256"] == request["publication_sha256"]
    repository = Path(request["repository"])
    assert (repository / "HEAD").is_file(), "acceptance requires the preserved bare test object cache"
    git_before = {str(path): metadata(path) for path in (repository, source_root / ".git")}
    source = json.loads((capsule / "MANIFEST.json").read_bytes())
    previous_overlay = json.loads((overlay / "OVERLAY.json").read_bytes())
    assert len(source["paths"]) == 13 and previous_overlay["source_capsule_id"] == capsule.name
    assert previous_overlay["source_manifest_sha256"] == CAPSULE_SHA
    work.mkdir(mode=0o700)
    environment = dict(os.environ, PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1", GIT_OPTIONAL_LOCKS="0", GIT_NO_LAZY_FETCH="1", GIT_NO_REPLACE_OBJECTS="1")
    commands, tool_calls = [], []
    home, package = work / "component-home", work / "package"
    manager = [host, "--codex-home", home]

    def run(label, argv, expected=0, contains=None, env=environment):
        arguments = [str(value) for value in argv]
        assert all(len(os.fsencode(argument)) < 120 * 1024 for argument in arguments), "helper argument exceeds safe CLI transport bound"
        result = subprocess.run(arguments, cwd=work, env=env, capture_output=True, timeout=130, check=False)
        commands.append(dict(label=label, exit=result.returncode, expected_exit=expected, expected_nonzero=expected is None,
            stdout_sha256=digest(result.stdout), stderr_sha256=digest(result.stderr), stdout_bytes=len(result.stdout), stderr_bytes=len(result.stderr)))
        assert max(len(result.stdout), len(result.stderr)) <= 8 * 1024 * 1024, label + ": output bound"
        assert result.returncode != 0 if expected is None else result.returncode == expected, label
        if contains is not None:
            assert contains in result.stderr, label + ": missing diagnostic"
        return result.stdout

    def call(label, name, value, success=True):
        envelope = dict(call_id=label, name=name, arguments=value)
        raw = run(label, manager + ["call", "tool", name, "invoke", canonical(envelope).decode()])
        tool = json.loads(raw)
        assert tool["success"] is success, label
        if name in (PLAN, RESTORE):
            assert len(tool["text"].encode()) <= 8 * 1024
            assert len(canonical(tool)) <= 32 * 1024
            framed = canonical(dict(type="result", id=2**64 - 1, result=tool)) + b"\n"
            assert len(framed) <= 64 * 1024 and json.loads(framed)["result"] == tool
        report = json.loads(tool["text"])
        assert report["update_allowed"] is False
        tool_calls.append(dict(label=label, name=name, success=tool["success"], arguments_sha256=hashed(value), result=report,
            reconstructed_max_id_frame_bytes=len(framed) if name in (PLAN, RESTORE) else None))
        return report

    try:
        sdk = json.loads(run("installed SDK identity", [python, "-I", "-B", "-c", "import codex_component_sdk,importlib.metadata,json;print(json.dumps({'path':codex_component_sdk.__file__,'version':importlib.metadata.version('codex-component-sdk')}))"]))
        assert sdk["version"] == "0.1.0" and source_root not in Path(sdk["path"]).resolve().parents
        external = work / "external-project"
        shutil.copytree(project, external, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        run("external 0.4 package build", [python, "-I", "-B", external / "build_package.py", "--output", package])
        shutil.rmtree(external)
        assert not external.exists()
        package_before = files(package)
        declaration = json.loads((package / "codex-component.json").read_bytes())
        old_declaration = json.loads((old / "codex-component.json").read_bytes())
        assert declaration["id"] == old_declaration["id"] == PLUGIN
        assert declaration["api_version"] == old_declaration["api_version"] == 1
        assert declaration["version"] == "0.4.0" and old_declaration["version"] == "0.3.0"
        assert {row["name"]: row["contract_version"] for row in declaration["components"]} == {name: 1 for name in (REVIEW, CAPSULE, OVERLAY, PLAN, RESTORE)}
        assert [row for row in declaration["components"] if row["name"] in (REVIEW, CAPSULE, OVERLAY)] == old_declaration["components"]
        assert declaration == json.loads((project / "codex-component.json").read_bytes())
        with zipfile.ZipFile(package / "plugin.pyz") as archive:
            members = {name: dict(sha256=digest(archive.read(name)), bytes=len(archive.read(name))) for name in archive.namelist() if not name.endswith("/")}
            for name in ("plugin.py", "selection_plan.py", "selection_tools.py", "maintenance_review.py", "source_capsule.py", "candidate_overlay.py"):
                assert members[name] == compact(source_before)[name]
            for row in json.loads(archive.read("static/TOOL_PROVENANCE.json"))["tools"]:
                assert members[row["packaged_path"]] == {key: row[key] for key in ("sha256", "bytes")}
        assert run("install existing 0.3", manager + ["install", old]).decode().strip() == PLUGIN
        old_list = json.loads(run("list installed 0.3", manager + ["list"]))
        old_object = home / "components/objects" / old_list["installed"][PLUGIN]
        old_object_before = files(old_object)
        assert compact(old_object_before) == compact(old_before)
        state = home / "components/state" / PLUGIN
        state.mkdir(mode=0o700, parents=True)
        copied_capsule = state / "upstream-capsules" / capsule.name
        copied_overlay = state / "upstream-overlays" / overlay.name
        shutil.copytree(capsule, copied_capsule)
        shutil.copytree(overlay, copied_overlay)
        assert compact(files(copied_capsule)) == compact(capsule_before)
        assert compact(files(copied_overlay)) == compact(overlay_before)
        copies_before = (files(copied_capsule), files(copied_overlay))
        old_review = call("0.3 existing review contract", REVIEW, request)
        empty = work / "empty-path"
        empty.mkdir()
        offline_env = dict(environment, PATH=str(empty))
        retained_inspection = json.loads(run("0.3 inspect retained overlay without host or Git", [python, "-I", "-B", old_object / "overlay_bootstrap.py", copied_overlay], env=offline_env))
        assert retained_inspection["status"] == "prepared" and retained_inspection["overlay_sha256"] == OVERLAY_SHA
        before_upgrade = files(home)
        run("duplicate installation rejected", manager + ["install", package], expected=None, contains=b"plugin is already installed")
        assert files(home) == before_upgrade
        run("remove 0.3 activation", manager + ["remove", PLUGIN])
        inactive = json.loads(run("observe nonatomic inactive upgrade intermediate", manager + ["list"]))
        assert inactive["installed"] == {} and inactive["enabled"] == []
        assert run("install 0.4 activation", manager + ["install", package]).decode().strip() == PLUGIN
        new_list = json.loads(run("list installed 0.4", manager + ["list"]))
        new_object = home / "components/objects" / new_list["installed"][PLUGIN]
        new_object_before = files(new_object)
        assert new_object != old_object and new_list["enabled"] == [PLUGIN]
        assert compact(new_object_before) == compact(package_before)
        assert call("0.4 retained review contract", REVIEW, request) == old_review
        full = json.loads(run("standalone real 13-path impact report", [python, "-I", "-B", package / "bootstrap.py", args.request, "--full-report"], expected=2))
        assert full["report_sha256"] == source["review"]["report_sha256"] == hashed(full["report"])
        assert full["plan_id"] == old_review["plan_id"]
        source_request = dict(contract_version=1, purpose="prepare_source_inputs", review_request=request, expected_plan_id=old_review["plan_id"],
            limits=dict(max_changed_paths=13, max_unique_blob_bytes=64 * 1024 * 1024, max_manifest_bytes=8 * 1024 * 1024))
        fresh = call("0.4 retained capsule contract", CAPSULE, source_request)
        fresh_source = json.loads((state / "upstream-capsules" / fresh["job_id"] / "MANIFEST.json").read_bytes())
        assert fresh["status"] == "sealed" and fresh["durability"] == "directory_fsync_completed"
        assert all(fresh_source[key] == source[key] for key in ("paths", "blobs", "trees", "commits", "review"))
        overlay_request = dict(contract_version=1, purpose="prepare_candidate_overlay", source_capsule_id=copied_capsule.name, expected_manifest_sha256=CAPSULE_SHA,
            limits=dict(max_changed_paths=13, max_output_bytes=64 * 1024 * 1024, max_manifest_bytes=8 * 1024 * 1024))
        fresh_overlay = call("0.4 retained overlay contract", OVERLAY, overlay_request)
        generated = state / "upstream-overlays" / fresh_overlay["job_id"]
        manifest = json.loads((generated / "OVERLAY.json").read_bytes())
        assert fresh_overlay["status"] == "prepared" and fresh_overlay["durability"] == "directory_fsync_completed"
        assert manifest["paths"] == previous_overlay["paths"] and fresh_overlay["release_gates"] == retained_inspection["release_gates"]
        assert compact(files(generated / "blobs")) == compact(files(copied_overlay / "blobs"))
        state_before_policy = files(state)
        requests, plans = {}, {}

        def planning_case(label, value, status, findings=(), success=True):
            requests[label] = copy.deepcopy(value)
            result = call(label, PLAN, value, success)
            assert result["status"] == status and set(findings) <= set(result.get("findings", []))
            assert result["state_written"] is result["activation_allowed"] is result["enforcement_authority"] is False
            if success:
                assert 0 < result["plan_bytes"] <= 96 * 1024
                if result["plan_json"] is not None:
                    assert result["plan_json_omitted"] is False
                    assert len(result["plan_json"].encode()) == result["plan_bytes"]
                    assert digest(result["plan_json"].encode()) == result["plan_id"]
                    assert json.loads(result["plan_json"])["inputs"] == {key: value[key] for key in ("graph", "profile", "policy", "review")}
                else:
                    assert result["plan_json_omitted"] is True
                assert set(result["required_later_gates"].values()) == {"pending"}
            assert files(state) == state_before_policy
            plans[label] = result
            return result

        value = synthetic()
        accepted = planning_case("synthetic explicit exclusion", value, "scoped_plan")
        accepted_result = json.loads(accepted["plan_json"])["result"]
        assert accepted_result["selected_changes"] == ["a"] and accepted_result["excluded"] == ["component:b"]
        needed = copy.deepcopy(value)
        needed["graph"]["units"][0]["requires"] = ["component:b"]
        needed["profile"]["catalog_sha256"] = hashed(needed["graph"])
        planning_case("synthetic missing excluded dependency", needed, "blocked", ["missing_dependency:component:b"])
        needed["profile"].update(include=["component:a", "component:b"], exclude=[])
        satisfied = planning_case("synthetic explicit dependency satisfied", needed, "scoped_plan")
        assert json.loads(satisfied["plan_json"])["result"]["selected_changes"] == ["a", "b"]
        owners = copy.deepcopy(value)
        owners["graph"]["changes"][0]["plugins"].append("plugin.shared")
        owners["profile"]["catalog_sha256"] = hashed(owners["graph"])
        planning_case("synthetic second owner scope missing", owners, "blocked", ["scope_binding_missing:a:plugin.shared"])
        owners["policy"]["rules"].append(dict(owners["policy"]["rules"][0], plugin="plugin.shared"))
        planning_case("synthetic both owner scopes explicit", owners, "scoped_plan")
        stale = copy.deepcopy(value)
        stale["profile"]["catalog_sha256"] = "0" * 64
        planning_case("synthetic stale catalog profile", stale, "blocked", ["stale_profile"])
        invalid = copy.deepcopy(value)
        invalid["graph"]["units"] = invalid["graph"]["units"][:1] * 65
        assert planning_case("synthetic bounded invalid graph", invalid, "invalid", success=False)["diagnostic_code"] == "tool_row_limit_or_shape"
        changed = copy.deepcopy(value)
        changed["profile"]["revision"] += 1
        newer = planning_case("synthetic changed profile identity", changed, "scoped_plan")
        assert newer["plan_id"] != accepted["plan_id"]
        projection = {key: full["report"][key] for key in ("schema", "status", "unresolved", "summary")}
        projection["changed_paths"] = [{key: row[key] for key in ("path", "unresolved")} for row in full["report"]["changed_paths"]]
        assert sorted(row["path"] for row in projection["changed_paths"]) == sorted(row["path"] for row in source["paths"])
        actual = synthetic()
        actual.update(review=projection)
        actual["graph"].update(changes=[], unresolved=["current_component_mapping_not_attested"])
        actual["graph"]["bindings"].update(base_revision=source["commits"]["base"], upstream_revision=source["commits"]["upstream"],
            custom_revision=source["commits"]["custom"], review_sha256=hashed(projection), capsule_sha256=CAPSULE_SHA,
            lineage_sha256=request["lineage_sha256"])
        actual["profile"]["catalog_sha256"] = hashed(actual["graph"])
        actual["policy"]["rules"] = []
        blocked = planning_case("real 13-path gaps remain blocked", actual, "blocked", ["impact:historical_index_not_current_composition"])
        assert blocked["selected_change_count"] == 0 and blocked["finding_count"] > 16 and blocked["findings_omitted"] is True
        assert blocked["plan_json"] is None and blocked["plan_json_omitted"] is True
        saved = dict(contract_version=1, purpose="restore_selection", plan_id=accepted["plan_id"], plan_json=accepted["plan_json"])
        assert call("installed caller plan revalidation", RESTORE, saved) == dict(accepted, plan_revalidated=True)
        wrong = dict(saved, plan_json=newer["plan_json"])
        assert call("stale caller plan rejected", RESTORE, wrong, False)["diagnostic_code"] == "stale_plan_evidence"
        altered = json.loads(accepted["plan_json"])
        altered["result"]["update_allowed"] = True
        assert call("altered caller result rejected", RESTORE, dict(saved, plan_json=canonical(altered).decode()), False)["diagnostic_code"] == "altered_plan"
        assert call("incompatible selection contract rejected", PLAN, dict(value, contract_version=2), False)["diagnostic_code"] == "unsupported_policy_contract"
        assert files(state) == state_before_policy
        (work / "caller-plan.json").write_bytes(accepted["plan_json"].encode())
        caller_before = fingerprint(work / "caller-plan.json")
        run("remove 0.4 activation", manager + ["remove", PLUGIN])
        removed = json.loads(run("list after removal", manager + ["list"]))
        assert removed["installed"] == {} and removed["enabled"] == []
        for name, arguments in ((REVIEW, request), (CAPSULE, source_request), (OVERLAY, overlay_request), (PLAN, value), (RESTORE, saved)):
            envelope = dict(call_id="removed", name=name, arguments=arguments)
            run("removed " + name, manager + ["call", "tool", name, "invoke", canonical(envelope).decode()], expected=None,
                contains=b"requested component is not installed and enabled")
        assert json.loads(run("retained overlay inspection after removal", [python, "-I", "-B", package / "overlay_bootstrap.py", copied_overlay], env=offline_env)) == retained_inspection
        assert fingerprint(work / "caller-plan.json") == caller_before
        assert files(state) == state_before_policy
        assert (files(copied_capsule), files(copied_overlay)) == copies_before
        assert files(old_object) == old_object_before and files(new_object) == new_object_before
        assert files(old) == old_before and files(capsule) == capsule_before and files(overlay) == overlay_before
        assert files(project) == source_before and files(package) == package_before and fingerprint(host) == host_before
        assert {str(path): metadata(path) for path in (repository, source_root / ".git")} == git_before
        assert {name: fingerprint(Path(name)) for name in inputs} == inputs
        retained_bytes = sum(row["bytes"] for row in files(work).values())
        assert retained_bytes < 12 * 1024 * 1024
        assert len(commands) == 36 and len(tool_calls) == 17
        assert sum(row["exit"] != 0 for row in commands) == 7
        assert sum(not row["success"] for row in tool_calls) == 4
        assert sum(row["result"].get("status") == "blocked" for row in tool_calls) == 4
        evidence = dict(passed=True, host_before=host_before, source_project=source_before, new_host_build_performed=False,
            source_package_files=package_before, package_members=members, sdk=sdk, inputs=inputs,
            original_package_files=old_before, original_capsule_files=capsule_before, original_overlay_files=overlay_before,
            package_upgrade=dict(from_version="0.3.0", to_version="0.4.0", method="remove_then_install", atomic=False,
                inactive_intermediate_observed=True, old_package_retained=True, copied_state_retained=True,
                old_activation=old_list, new_activation=new_list), legacy_contracts=dict(review=old_review, capsule=fresh, overlay=fresh_overlay),
            actual_review_report_sha256=full["report_sha256"], actual_projection=projection,
            synthetic_scope="Invented graph/object identities supplied to actual installed planning tools; no Codex ownership or filesystem attestation",
            real_mapping_scope="Empty unresolved catalog retains fixture-only composition/inventory placeholders and preserves all actual impact gaps; source labels never promoted to owner mappings",
            requests=requests, plans=plans, caller_plan=caller_before, state_files=state_before_policy,
            state_not_written_by_planning_tools=True, original_artifacts_unchanged=True, git_metadata_unchanged=True,
            caller_plan_retained_after_removal=True, offline_selection_restore="not_implemented_or_claimed",
            independent_old_overlay_inspection_after_removal=True, retained_work_bytes=retained_bytes,
            commands=commands, command_count=len(commands), tool_calls=tool_calls,
            expected_nonzero_commands=sum(row["exit"] != 0 for row in commands),
            expected_rejected_tool_calls=sum(not row["success"] for row in tool_calls),
            computed_blocked_plans=sum(row["result"].get("status") == "blocked" for row in tool_calls),
            limits="Planning/revalidation only; no final filesystem diff enforcement, real resolved component map, installation rollback, activation, A/B routing, browser flow, or native extraction acceptance.")
        report = json.dumps(evidence, indent=2).encode() + b"\n"
        assert len(report) < 2 * 1024 * 1024
        (work / "acceptance.json").write_bytes(report)
        print(work / "acceptance.json")
    except Exception as error:
        (work / "acceptance-failure.json").write_text(json.dumps(dict(passed=False, failure_type=type(error).__name__, commands=commands, tool_calls=tool_calls), indent=2) + "\n")
        raise


if __name__ == "__main__":
    main()
