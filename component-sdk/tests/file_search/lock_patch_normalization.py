"""Conservative, recorded normalization of an isolated component source export.

Cargo can reorder unused records from separate patch-source tables between
processes. Remove only remote patches proven irrelevant by the FULL resolved
lock, retaining every path patch and every potentially used or ambiguous patch.
The caller must rerun metadata and validate the complete package graph before
the final locked build. This does not alter the SDK exporter or original repo.
"""

from collections import Counter
import copy
import hashlib
import json
import re
import shutil
import tomllib

from acceptance_support import fingerprint, require


def unused_patch_plan(manifest, lock):
    document = copy.deepcopy(manifest)
    used_names = {package["name"] for package in lock["package"]}
    unused = Counter(
        (item["name"], item["version"], item["source"])
        for item in lock.get("patch", {}).get("unused", [])
    )
    removals = []
    for origin, patches in list(document.get("patch", {}).items()):
        for alias, declaration in list(patches.items()):
            if not isinstance(declaration, dict):
                continue
            # Version constraints, branches, tags and additional options need
            # semantic interpretation beyond this deliberately narrow rule.
            if not {"git", "rev"} <= declaration.keys() <= {"git", "rev", "package"}:
                continue
            name = declaration.get("package", alias)
            revision = declaration["rev"]
            url = declaration["git"]
            if name in used_names or not re.fullmatch(r"[0-9a-f]{40}", revision):
                continue
            source = f"git+{url}?rev={revision}#{revision}"
            matching = [
                key
                for key, count in unused.items()
                if count and key[0] == name and key[2] == source
            ]
            if len(matching) != 1:
                continue
            identity = matching[0]
            unused[identity] -= 1
            removals.append(
                {
                    "patch_origin": origin,
                    "alias": alias,
                    "declaration": declaration,
                    "unused_identity": dict(
                        zip(("name", "version", "source"), identity)
                    ),
                    "package_name_absent_from_full_lock": True,
                }
            )
            del patches[alias]
        if not patches:
            del document["patch"][origin]
    return document, removals


def assert_package_records_unchanged(before, after):
    # Full records include dependencies and checksums, including packages omitted
    # from platform-filtered metadata. Identity-set equality alone is insufficient.
    require(
        before["package"] == after["package"],
        "unused-patch normalization changed full resolved package records",
    )
    require(
        before.get("version") == after.get("version"),
        "unused-patch normalization changed lockfile format",
    )


def prepare_normalization(source, artifacts, dump_toml):
    require(
        source.parent == artifacts and source.name == "source",
        "normalization is limited to this run's isolated source export",
    )
    manifest_path = source / "codex-rs/Cargo.toml"
    lock_path = source / "codex-rs/Cargo.lock"
    inventory_path = source / "COMPONENT_SOURCE_EXPORT.json"
    manifest = tomllib.loads(manifest_path.read_text())
    lock = tomllib.loads(lock_path.read_text())
    document, removed = unused_patch_plan(manifest, lock)
    report = {
        "removed": removed,
        "manifest_before": fingerprint(manifest_path),
        "lock_before": fingerprint(lock_path),
        "inventory_before": fingerprint(inventory_path),
        "package_graph_before_sha256": hashlib.sha256(
            json.dumps(lock["package"], sort_keys=True).encode()
        ).hexdigest(),
    }
    if not removed:
        report["applied"] = False
        return report, lock
    inventory = json.loads(inventory_path.read_text())
    entries = [
        entry
        for entry in inventory["source_hashes"]
        if entry["path"] == "codex-rs/Cargo.toml"
    ]
    require(len(entries) == 1, "export inventory has no unique root Cargo manifest")
    require(
        entries[0]["export_sha256"] == report["manifest_before"]["sha256"],
        "export root manifest changed before patch normalization",
    )
    for source_path, name in (
        (manifest_path, "manifest-before-patch-normalization.toml"),
        (lock_path, "lock-before-patch-normalization.lock"),
        (inventory_path, "original-source-inventory.json"),
    ):
        destination = artifacts / name
        require(not destination.exists(), "normalization backup already exists")
        shutil.copy2(source_path, destination)
    rewritten = dump_toml(document)
    require(
        tomllib.loads(rewritten) == document, "manifest serialization changed semantics"
    )
    manifest_path.write_text(rewritten)
    report.update(applied=True, manifest_after=fingerprint(manifest_path))
    entries[0]["export_sha256"] = report["manifest_after"]["sha256"]
    inventory["isolated_build_normalization"] = {
        "kind": "remove_full_lock_proven_unused_pinned_git_patches",
        "original_inventory_sha256": report["inventory_before"]["sha256"],
        "manifest_before_sha256": report["manifest_before"]["sha256"],
        "manifest_after_sha256": report["manifest_after"]["sha256"],
        "removed": removed,
        "package_graph_verification": "required after metadata; see independent-build.json",
    }
    inventory_path.write_text(json.dumps(inventory, indent=2) + "\n")
    report["inventory_after"] = fingerprint(inventory_path)
    return report, lock
