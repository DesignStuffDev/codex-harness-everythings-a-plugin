"""Install an independently built native search worker into a fresh GUI fixture.

This helper never starts a substitute search service. Actual manager commands
install/select the immutable package; the normal App Server owns its worker.
Private GUI readiness URLs and credentials are never read or emitted here.
"""

import json
import os
from pathlib import Path
import shutil

from file_search.acceptance_support import fingerprint, inventory, require, same_content

PLUGIN = "native.file-search-local"


def validate_build(proof_path, host, *, reuse_proof_sha256=None):
    proof_path = proof_path.resolve(strict=True)
    proof_before = fingerprint(proof_path)
    if reuse_proof_sha256 is not None:
        require(
            len(reuse_proof_sha256) == 64
            and all(char in "0123456789abcdef" for char in reuse_proof_sha256)
            and proof_before["sha256"] == reuse_proof_sha256,
            "reused independent build report differs from its reviewed SHA-256",
        )
    proof = json.loads(proof_path.read_text())
    require(
        proof.get("passed") is True
        and proof.get("evidence_kind") == "independent_source_build",
        "search report must prove a successful independent source build",
    )
    require(
        proof["build_artifact"]["separate_initially_empty_target"] is True
        and proof["build_artifact"]["fresh"] is False
        and proof["source_resolution"]["all_local_paths_inside_export"] is True
        and proof["source_resolution"]["lock_new_identities"] == 0
        and proof["final_locked_build_preserved_lock_bytes"] is True,
        "independent worker compilation/provenance was not verified",
    )
    artifacts = Path(proof["artifact_directory"]).resolve(strict=True)
    require(
        proof["source_parked_before_runtime"] is True
        and not (artifacts / "source").exists()
        and fingerprint(artifacts / "source-inventory.json")
        == proof["exported_source_inventory"],
        "exported worker source paths or inventory changed",
    )
    require(
        proof["frozen_binaries_before"] == proof["frozen_binaries_after"],
        "original independent-build binaries changed during the build",
    )
    if reuse_proof_sha256 is None:
        require(
            fingerprint(host) == proof["frozen_binaries_before"]["manager"],
            "manager differs from the frozen independent-build manager",
        )
    # Explicit reuse preserves the old build record and its package; the current
    # manager/CLI are bound by the fresh migration and full GUI runtime reports.
    package = Path(proof["package_path"]).resolve(strict=True)
    require(package == artifacts / "package", "unexpected independent package path")
    require(inventory(package) == proof["package_files"], "search package changed")
    manifest = json.loads((package / "codex-component.json").read_text())
    require(
        manifest["id"] == PLUGIN
        and manifest["api_version"] == 1
        and any(
            component["kind"] == "file_search"
            and component["name"] == "default"
            and component["contract_version"] == 1
            for component in manifest["components"]
        ),
        "independent package does not provide the compatible native search component",
    )
    worker = (package / manifest["entrypoint"]).resolve(strict=True)
    require(package in worker.parents, "worker entrypoint escapes package")
    require(
        same_content(
            {"worker": fingerprint(worker)},
            {"worker": proof["build_artifact"]["fingerprint"]},
        ),
        "packaged worker differs from the independently compiled binary",
    )
    require(
        fingerprint(proof_path) == proof_before,
        "independent build report changed during validation",
    )
    return proof_path, proof, package, manifest


def verify_inputs(host, evidence):
    """Recheck original proof/package provenance after installation and UI use."""
    require(
        fingerprint(host) == evidence["frozen_manager"],
        "runtime manager changed after search installation",
    )
    require(
        fingerprint(Path(evidence["build_report"]))
        == evidence["build_report_fingerprint"],
        "independent build report changed during GUI acceptance",
    )
    validate_build(
        Path(evidence["build_report"]),
        host,
        reuse_proof_sha256=evidence["reuse_proof_sha256"],
    )


def claim_fixture(manual_path, manual, fixture, work, build_fingerprint):
    """Refuse arbitrary homes, reused selected fixtures, and observed live users."""
    gui = manual_path.parent / "gui"
    home, project = gui / "codex-home", gui / "project"
    require(
        Path(manual["gui_fixture"]) == gui / "fixture-ready.json"
        and Path(fixture["codex_home"]) == home
        and Path(fixture["cwd"]) == project,
        "input is not the manual migration script's own isolated GUI fixture",
    )
    # The fixture generator records artifact paths as well as fingerprints;
    # the migration report records the fingerprints only. Validate both schemas
    # and recheck the actual named files instead of comparing unlike records.
    require(
        set(fixture["binaries_before"]) == set(manual["binaries_before"]),
        "fixture and migration report name different binary roles",
    )
    for name, expected in manual["binaries_before"].items():
        recorded = fixture["binaries_before"][name]
        require(
            set(recorded) == {"path", *expected},
            "unexpected fixture binary fingerprint fields",
        )
        binary = Path(recorded["path"])
        require(
            binary.is_absolute() and binary.is_file() and not binary.is_symlink(),
            "fixture binary is missing, relative or linked",
        )
        actual = fingerprint(binary)
        require(
            {key: recorded[key] for key in expected} == expected
            and {key: actual[key] for key in expected} == expected,
            "fixture binary differs from the migration fingerprint",
        )
    for directory in (gui, home, project):
        require(directory.resolve(strict=True) == directory, "fixture uses symlinks")
        require(
            not any((parent / ".git").exists() for parent in directory.parents),
            "GUI fixture must be outside source checkouts",
        )
    observed, inaccessible = [], []
    for process in Path("/proc").glob("[0-9]*"):
        for name in ("exe", "cwd"):
            try:
                target = Path(os.readlink(process / name))
            except FileNotFoundError:
                continue
            except PermissionError:
                inaccessible.append({"pid": int(process.name), "area": name})
                continue
            if target == gui or gui in target.parents:
                observed.append({"pid": int(process.name), "area": name})
    require(not observed, "GUI fixture still has observed live users")
    settings = json.loads((home / "components/config.json").read_text())
    require(
        PLUGIN not in settings["installed"]
        and "file_search:default" not in settings["selections"],
        "fixture already has an installed or selected search component",
    )
    # An exclusive claim survives failure. Never silently reuse a prior test home.
    with (gui / ".selected-search-acceptance-claim.json").open("x") as stream:
        json.dump({"work": str(work), "search_build_report": build_fingerprint}, stream)
        stream.write("\n")
    return {"isolated_fixture": str(gui), "unreadable_process_links": inaccessible}


def install(
    proof_path,
    host,
    manual_path,
    manual,
    fixture,
    work,
    environment,
    owner_type,
    drain_subreaper,
    evidence,
    *,
    reuse_proof_sha256=None,
):
    proof_path, proof, package, manifest = validate_build(
        proof_path, host, reuse_proof_sha256=reuse_proof_sha256
    )
    evidence.update(
        evidence_kind=(
            "reused_package_new_host"
            if reuse_proof_sha256 is not None
            else "independent_package_original_host"
        ),
        new_independent_build=False,
        reuse_proof_sha256=reuse_proof_sha256,
        original_build_binaries=proof["frozen_binaries_before"],
        backend="selected external native.file-search-local through real App Server fuzzyFileSearch",
        build_report=str(proof_path),
        build_report_fingerprint=fingerprint(proof_path),
        helper_fingerprint=fingerprint(Path(__file__)),
        source_inventory=proof["exported_source_inventory"],
        independent_worker=proof["build_artifact"]["fingerprint"],
        frozen_manager=fingerprint(host),
        full_cli_binding="current manual-migration proof; not the standalone search CLI from the worker build",
        commands=[],
    )
    evidence["fixture_claim"] = claim_fixture(
        manual_path, manual, fixture, work, evidence["build_report_fingerprint"]
    )
    home, project = Path(fixture["codex_home"]), Path(fixture["cwd"])
    original_settings = json.loads((home / "components/config.json").read_text())
    objects = home / "components/objects"
    original_objects = inventory(objects)
    install_input = work / "search-install-input"
    shutil.copytree(package, install_input)
    require(
        same_content(inventory(install_input), proof["package_files"]),
        "search install copy differs from independent package",
    )
    management = [host, "--codex-home", home]
    environment = dict(environment, CODEX_HOME=str(home))
    for label, arguments in (
        ("install", ["install", install_input]),
        ("select", ["select", "file_search", "default", PLUGIN]),
        ("list", ["list"]),
    ):
        entry = {"label": label, "passed": False}
        evidence["commands"].append(entry)
        owner = owner_type(
            management + arguments,
            project,
            work / f"search-manager-{label}",
            environment,
        )
        try:
            status = owner.process.wait(timeout=90)
            entry["status"] = status
            require(status == 0, "search manager command failed; inspect private logs")
            owner.reap_and_check()
            entry.update(
                processes=owner.snapshot(),
                tracked_processes_absent=True,
                reaped_orphans=owner.reaped_orphans,
            )
            if label == "list":
                selected = json.loads(owner.stdout_path.read_text())
            entry["subreaper_drain"] = drain_subreaper()
            require(entry["subreaper_drain"]["passed"], "search setup left children")
            entry["passed"] = True
        finally:
            try:
                if not entry["passed"]:
                    entry["forced_failure_cleanup"] = True
                    owner.emergency_cleanup()
            finally:
                owner.finish()
    expected_settings = json.loads(json.dumps(original_settings))
    expected_settings["installed"][PLUGIN] = selected["installed"][PLUGIN]
    expected_settings["enabled"].append(PLUGIN)
    expected_settings["selections"]["file_search:default"] = PLUGIN
    require(
        json.loads((home / "components/config.json").read_text()) == expected_settings,
        "search selection changed unrelated saved settings",
    )
    # List intentionally omits per-plugin configuration. Check that public
    # projection separately, while retaining the full saved-config comparison.
    require(
        selected
        == {
            key: expected_settings[key]
            for key in ("enabled", "installed", "selections")
        },
        "manager list differs from saved component settings",
    )
    object_name = selected["installed"][PLUGIN]
    require(
        Path(object_name).name == object_name and object_name not in (".", ".."),
        "installed object identifier escapes object directory",
    )
    installed = objects / object_name
    require(
        installed.resolve(strict=True) == installed, "installed object is a symlink"
    )
    require(
        same_content(inventory(installed), proof["package_files"]),
        "immutable installed search object differs from independent package",
    )
    current_objects = inventory(objects)
    require(
        all(
            current_objects.get(name) == data for name, data in original_objects.items()
        ),
        "search setup changed an existing immutable component object",
    )
    require(
        set(current_objects) - set(original_objects)
        == {f"{object_name}/{name}" for name in proof["package_files"]},
        "search setup added unexpected immutable objects",
    )
    worker = (installed / manifest["entrypoint"]).resolve(strict=True)
    require(installed in worker.parents, "installed worker escapes immutable object")
    install_input.rename(work / "search-install-input.parked")
    evidence.update(
        installed_worker=str(worker),
        installed_worker_fingerprint=fingerprint(worker),
        installed_package_files=inventory(installed),
        original_components_preserved=True,
        install_input_parked=True,
    )
    require(
        fingerprint(host) == evidence["frozen_manager"],
        "manager changed during install",
    )
    verify_inputs(host, evidence)
    return worker


def observed_worker(owner, worker):
    found = [
        saved for saved in owner.snapshot() if saved.get("executable") == str(worker)
    ]
    require(found, "actual selected search worker executable was never observed")
    return found


def assert_worker_absent(records):
    require(
        all(not Path(f"/proc/{saved['pid']}").exists() for saved in records),
        "selected search worker remains alive or unreaped after launcher shutdown",
    )
