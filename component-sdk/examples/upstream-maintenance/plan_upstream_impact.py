#!/usr/bin/env python3
"""Offline candidate impact inventory; maintenance support, not an updater.

Reuse existing lineage/object verification. No fetch, checkout, ref/index/object
write, patch application, build, source execution or activation. Exit 1 is invalid;
exit 2 is unresolved planning, including every successful report in this version.
"""

import sys

sys.dont_write_bytecode = True

import argparse
from collections import Counter
import json
from pathlib import Path
import subprocess

import validate_checkpoint_lineage as lineage

MAX_CHANGED_DETAILS = 2048
MAX_EDGE_REFERENCES = 4096
MAX_EDGES_PER_PATH = 64
MAX_OWNERS_PER_PATH = 64
MAX_REPORT_DETAILS = 4 * 1024 * 1024
REQUIRED_GATES = [
    "candidate_authenticity_and_later_revision_ancestry",
    "current_composition_source_and_ownership_closure",
    "symbol_and_transitive_dependency_impact",
    "contract_and_coordinated_package_sdk_versions",
    "permissions_credentials_sandbox_security_review",
    "persistent_schema_migration_and_downgrade_review",
    "license_notice_and_platform_review",
    "isolated_customization_preserving_candidate",
    "real_host_independent_plugin_headless_and_gui_regression",
    "real_later_upstream_revision_integration",
    "incompatible_update_refusal_and_failed_activation_state_recovery",
    "separately_installed_maintenance_and_external_recovery_entrypoint",
]


class InvalidInput(ValueError):
    pass


def signal_hints(path):
    """Review routing only. Absence of a hint never means absence of risk."""
    parts = set(path.lower().split("/"))
    name = path.rsplit("/", 1)[-1].lower()
    hints = []
    if name.startswith(("license", "notice", "copying")):
        hints.append("license_notice")
    if name in {
        "cargo.toml",
        "cargo.lock",
        "package.json",
        "pnpm-lock.yaml",
        "module.bazel",
        "module.bazel.lock",
        "rust-toolchain.toml",
    }:
        hints.append("dependency_build_toolchain")
    if parts & {
        "protocol",
        "app-server-protocol",
        "component-api",
        "component-sdk",
        "schemas",
    }:
        hints.append("contract_schema_sdk")
    if parts & {
        "login",
        "auth",
        "permissions",
        "execpolicy",
        "sandboxing",
        "network-proxy",
    }:
        hints.append("security_auth_policy")
    if parts & {"state", "thread-store", "attachment-store", "rollout"}:
        hints.append("persistent_state")
    return hints


def entry_status(before, after):
    if before is None:
        return "added"
    if after is None:
        return "deleted"
    if before["type"] != after["type"] or before["mode"] != after["mode"]:
        return "mode_or_type_changed"
    return "modified"


def plan(index, objects, candidate_revision, composition_revision, publication=None):
    report = {
        "schema": "codex-upstream-offline-impact-v1",
        "status": "unresolved",
        "invalid": 0,
        "planning_only": True,
        "automatic_update_eligible": False,
        "runtime_or_updater_acceptance": False,
        "fetch_or_mutation_performed": False,
        "inputs": {},
        "changed_paths": [],
        "unresolved": [],
        "required_gates": REQUIRED_GATES,
        "limits": [
            "Recorded ownership/edges remain historical claims; path matches do not prove semantics.",
            "No rename inference, source parsing, transitive graph or compatibility decision.",
            "Unavailable objects are not fetched and stale indexes are not silently advanced.",
            "Same-revision, same-tree or zero-overlap reports cannot pass update acceptance.",
            "Existing GitObjects capture limits are post-capture, not streaming memory bounds.",
            "Edge details use index pointers; detail count/reference/byte caps remain explicit unresolved gaps.",
        ],
    }
    try:
        for revision in (candidate_revision, composition_revision):
            if not isinstance(revision, str) or not lineage.OID.fullmatch(revision):
                raise InvalidInput("exact_revision_required")
        checked = lineage.validate(index, objects, publication)
        report["lineage_validation"] = checked
        if checked["invalid"]:
            report.update(status="invalid", invalid=checked["invalid"])
            return report
        source = index["source"]
        for revision in (
            source["upstream_revision"],
            source["checkpoint_commit"],
            candidate_revision,
            composition_revision,
        ):
            if not isinstance(revision, str) or not lineage.OID.fullmatch(revision):
                raise InvalidInput("exact_revision_required")
        base_revision = source["upstream_revision"]
        report["inputs"] = {
            "upstream_repository_claim": source["upstream_repository"],
            "base_revision": base_revision,
            "candidate_revision": candidate_revision,
            "lineage_checkpoint_revision": source["checkpoint_commit"],
            "lineage_checkpoint_tree": source["checkpoint_tree"],
            "requested_composition_revision": composition_revision,
        }
        base_tree = objects.commit_tree(base_revision)
        if base_tree != source["import_tree"]:
            raise InvalidInput("upstream_base_tree_mismatch")
        candidate_tree = objects.commit_tree(candidate_revision)
        for tree in (base_tree, candidate_tree):
            if not isinstance(tree, str) or not lineage.OID.fullmatch(tree):
                raise InvalidInput("invalid_tree_identity")
        report["inputs"] = {
            "upstream_repository_claim": source["upstream_repository"],
            "base_revision": base_revision,
            "base_tree": base_tree,
            "candidate_revision": candidate_revision,
            "candidate_tree": candidate_tree,
            "lineage_checkpoint_revision": source["checkpoint_commit"],
            "lineage_checkpoint_tree": source["checkpoint_tree"],
            "requested_composition_revision": composition_revision,
            "candidate_publisher_authenticated": False,
            "candidate_is_later_descendant_verified": False,
        }
        if checked["unresolved"]:
            report["unresolved"].append("existing_lineage_findings_remain_unresolved")
        if composition_revision != source["checkpoint_commit"]:
            report["unresolved"].append("lineage_does_not_cover_requested_composition")
        try:
            report["inputs"]["requested_composition_tree"] = objects.commit_tree(
                composition_revision
            )
        except (ValueError, KeyError, OSError, subprocess.TimeoutExpired):
            report["unresolved"].append("requested_composition_commit_unavailable")
        if candidate_revision == base_revision:
            report["unresolved"].append("no_upstream_revision_advance")
        old, new = objects.tree(base_tree), objects.tree(candidate_tree)
        changes = sorted(
            path for path in old.keys() | new.keys() if old.get(path) != new.get(path)
        )
        if base_tree == candidate_tree:
            report["unresolved"].append("no_upstream_tree_change")
        rows = {row["path"]: row for row in index.get("paths", [])}
        origin_edges = {}
        for edge in index.get("boundary_edges", []):
            for origin in edge.get("origins", []):
                if (
                    origin.get("repository") == source["upstream_repository"]
                    and origin.get("revision") == base_revision
                    and origin.get("tree") == base_tree
                ):
                    origin_edges.setdefault(origin["path"], []).append(edge)
        counts = Counter()
        detail_bytes, remaining_edges, details_limited = 0, MAX_EDGE_REFERENCES, False
        edge_pointers = {
            edge["id"]: "/boundary_edges/" + str(i)
            for i, edge in enumerate(index.get("boundary_edges", []))
        }
        for path in changes[:MAX_CHANGED_DETAILS]:
            if not lineage.valid_path(path):
                raise InvalidInput("invalid_upstream_tree_path")
            before, after = old.get(path), new.get(path)
            matched = {edge["id"]: edge for edge in origin_edges.get(path, [])}
            inventory = rows.get(path)
            declared_owners = (
                set(inventory.get("inventory_owners", [])) if inventory else set()
            )
            declared_owners.update(edge["component"] for edge in matched.values())
            findings = [
                "semantic_impact_not_reviewed",
                "transitive_dependency_impact_unknown",
            ]
            if not matched:
                findings.append("original_source_boundary_mapping_missing")
            if not declared_owners:
                findings.append("ownership_unknown")
            elif len(declared_owners) > 1:
                findings.append("multiple_recorded_owners_not_resolved")
            if any(
                edge.get("current_semantic_closure") is not True
                for edge in matched.values()
            ):
                findings.append("matched_boundary_semantic_closure_incomplete")
            if composition_revision != source["checkpoint_commit"]:
                findings.append("historical_index_not_current_composition")
            if after is None:
                findings.append("deleted_origin_requires_customization_review")
            if any(
                entry
                and (
                    entry["type"] != "blob" or entry["mode"] not in {"100644", "100755"}
                )
                for entry in (before, after)
            ):
                findings.append("non_regular_source_entry_requires_review")
            selected_edges = sorted(matched.values(), key=lambda value: value["id"])[
                : min(MAX_EDGES_PER_PATH, remaining_edges)
            ]
            owners = sorted(
                owner
                for owner in declared_owners
                if isinstance(owner, str) and len(owner) <= 256
            )[:MAX_OWNERS_PER_PATH]
            if len(selected_edges) != len(matched) or len(owners) != len(
                declared_owners
            ):
                findings.append("mapping_detail_limit_reached")
            edge_refs = []
            for edge in selected_edges:
                labels = {key: edge.get(key) for key in ("id", "component", "relation")}
                if any(
                    not isinstance(value, str) or len(value) > 256
                    for value in labels.values()
                ):
                    findings.append("unsupported_edge_label_omitted")
                    continue
                edge_refs.append(
                    dict(
                        labels,
                        index_pointer=edge_pointers[edge["id"]],
                        recorded_destination_count=len(edge.get("destinations", [])),
                        semantic_closure_recorded=edge.get("current_semantic_closure")
                        is True,
                    )
                )
            row = {
                "path": path,
                "change": entry_status(before, after),
                "base_entry": before,
                "candidate_entry": after,
                "recorded_owners_not_validated": owners,
                "recorded_owner_count": len(declared_owners),
                "historical_inventory_present": inventory is not None,
                "matched_source_edge_count": len(matched),
                "matched_source_edges": edge_refs,
                "review_routing_hints_not_risk_clearance": signal_hints(path),
                "unresolved": findings,
            }
            encoded_bytes = len(json.dumps(row, sort_keys=True).encode())
            if detail_bytes + encoded_bytes > MAX_REPORT_DETAILS:
                details_limited = True
                report["unresolved"].append("report_detail_byte_budget_reached")
                break
            detail_bytes += encoded_bytes
            remaining_edges -= len(edge_refs)
            counts.update(findings)
            report["changed_paths"].append(row)
        report["summary"] = {
            "changed_paths_total": len(changes),
            "changed_paths_reported": len(report["changed_paths"]),
            "path_inventory_complete": len(changes) <= MAX_CHANGED_DETAILS
            and not details_limited,
            "retained_detail_bytes_compact_json": detail_bytes,
            "unresolved_counts": dict(sorted(counts.items())),
            "matched_paths_do_not_establish_compatibility": True,
        }
        if len(changes) > MAX_CHANGED_DETAILS:
            report["unresolved"].append("changed_path_detail_limit_reached")
        report["unresolved"].extend(REQUIRED_GATES)
    except InvalidInput:
        report.update(
            status="invalid",
            invalid=1,
            diagnostic_kind="invalid_revision_or_tree_binding",
        )
    except (
        KeyError,
        TypeError,
        AttributeError,
        ValueError,
        OSError,
        RecursionError,
        subprocess.TimeoutExpired,
    ) as error:
        # No raw stderr, URLs, config or arbitrary exception strings in reports.
        report["unresolved"].append("input_or_local_object_unavailable_or_unsupported")
        report["diagnostic_kind"] = type(error).__name__
    return report


def render_report(report):
    output = json.dumps(report, indent=2, sort_keys=True)
    if len(output.encode()) <= lineage.MAX_INPUT:
        return output
    # No large inherited scalar survives this bounded fallback. Keep input
    # digests when available, never relabel truncated evidence as complete.
    compact = {
        "schema": "codex-upstream-offline-impact-v1",
        "status": "invalid" if report["invalid"] else "unresolved",
        "invalid": report["invalid"],
        "automatic_update_eligible": False,
        "runtime_or_updater_acceptance": False,
        "detailed_report_omitted": True,
        "unresolved": ["report_detail_output_limit_reached"],
    }
    for name in (
        "index_sha256",
        "planner_sha256",
        "validator_sha256",
        "publication_receipt_sha256",
    ):
        value = report.get(name)
        if (
            isinstance(value, str)
            and len(value) == 64
            and all(c in "0123456789abcdef" for c in value)
        ):
            compact[name] = value
    return json.dumps(compact, indent=2, sort_keys=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("index", type=Path)
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--candidate-revision", required=True)
    parser.add_argument("--composition-revision", required=True)
    parser.add_argument("--publication-receipt", type=Path)
    args = parser.parse_args()
    try:
        raw = lineage.read_limited(args.index)
        publication = (
            lineage.read_limited(args.publication_receipt)
            if args.publication_receipt
            else None
        )
        report = plan(
            lineage.json_bytes(raw),
            lineage.GitObjects(args.repository),
            args.candidate_revision,
            args.composition_revision,
            publication,
        )
        report["index_sha256"] = lineage.digest(raw)
        report["planner_sha256"] = lineage.digest(Path(__file__).read_bytes())
        report["validator_sha256"] = lineage.digest(Path(lineage.__file__).read_bytes())
        if publication is not None:
            report["publication_receipt_sha256"] = lineage.digest(publication)
    except (ValueError, TypeError, KeyError, OSError, RecursionError):
        report = {
            "schema": "codex-upstream-offline-impact-v1",
            "status": "invalid",
            "invalid": 1,
            "diagnostic_kind": "invalid_or_unavailable_input",
            "automatic_update_eligible": False,
            "runtime_or_updater_acceptance": False,
        }
    output = render_report(report)
    print(output)
    return 1 if report["invalid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
