"""Audit complete supplied inventories; no filesystem or activation authority."""

import json
import unicodedata

import selection_plan as policy

MAX_INVENTORY = 16384


def inventory(value):
    """Freeze and validate caller records, not completeness or hash authenticity."""
    policy.revisioned(value, "namespace completeness entries")
    policy.require(
        type(value["entries"]) is dict and len(value["entries"]) <= MAX_INVENTORY
    )
    policy.require(all(type(name) is str for name in value["entries"]))
    raw = policy.canonical(value)
    value = json.loads(raw)
    policy.text(value["namespace"])
    policy.require(value["completeness"] == "complete", "incomplete_inventory")
    entries = value["entries"]
    for name, item in entries.items():
        policy.path(name)
        policy.require(unicodedata.normalize("NFC", name) == name, "noncanonical_path")
        policy.exact(item, "kind mode blob sha256 bytes")
        policy.require(item["kind"] == "file", "unsupported_entry_kind")
        policy.entry({key: item[key] for key in ("blob", "mode")})
        policy.text(item["sha256"], r"[a-f0-9]{64}")
        policy.require(type(item["bytes"]) is int and 0 <= item["bytes"] < 2**63)
    return value


def audit_scope(plan, before, after, expected_plan_id):
    """Check every actual difference in supplied whole inventories against scopes.

    No worker change list is accepted. A rename requires delete plus add permission.
    This checks declared ownership/classification, not their real-world correctness.
    """
    policy.require(type(plan) is policy.SelectionPlan)
    saved = policy.restore_plan(plan.payload, expected_plan_id).document()
    before, after = inventory(before), inventory(after)
    policy.require(
        before["namespace"] == after["namespace"], "inventory_namespace_mismatch"
    )
    old_entries, new_entries = before["entries"], after["entries"]
    names = old_entries.keys() | new_entries.keys()
    folded = {name.casefold() for name in names}
    policy.require(len(folded) == len(names), "ambiguous_path_alias")
    for name in folded:
        parts = name.split("/")
        policy.require(
            not any("/".join(parts[:n]) in folded for n in range(1, len(parts))),
            "file_path_prefix_collision",
        )
    objects, content = {}, {}
    for item in list(old_entries.values()) + list(new_entries.values()):
        identity = (item["sha256"], item["bytes"])
        policy.require(
            objects.setdefault(item["blob"], identity) == identity,
            "contradictory_blob_identity",
        )
        identity = (item["blob"], item["bytes"])
        policy.require(
            content.setdefault(item["sha256"], identity) == identity,
            "contradictory_content_identity",
        )
    graph, scopes = (saved["inputs"][key] for key in ("graph", "policy"))
    findings = set(saved["result"]["findings"])
    if policy.digest(before) != graph["bindings"]["before_inventory_sha256"]:
        findings.add("before_inventory_binding_mismatch")
    changes = {row["path"]: row for row in graph["changes"]}
    rules = {(row["plugin"], row["change"]): row for row in scopes["rules"]}
    selected = set(saved["result"]["selected_changes"])
    for name, change in changes.items():
        entry = old_entries.get(name)
        projected = (
            None if entry is None else {key: entry[key] for key in ("blob", "mode")}
        )
        if change["before"] != projected:
            findings.add("before_object_mismatch:" + name)
    differences, observed = [], set()
    for name in sorted(names):
        old, new = old_entries.get(name), new_entries.get(name)
        if old == new:
            continue
        if old is None:
            operations = ["add"]
            if new["mode"] == "100755":
                operations.append("mode")
        elif new is None:
            operations = ["delete"]
        else:
            operations = ["modify"] if old["sha256"] != new["sha256"] else []
            if old["mode"] != new["mode"]:
                operations.append("mode")
        differences.append(
            dict(path=name, before=old, after=new, operations=operations)
        )
        change = changes.get(name)
        if change is None or change["id"] not in selected:
            findings.add("unselected_or_unmapped_diff:" + name)
            continue
        observed.add(change["id"])
        for plugin in change["plugins"]:
            rule = rules.get((plugin, change["id"]))
            if rule is None or not set(operations) <= set(rule["operations"]):
                findings.add("operation_denied:" + name + ":" + plugin)
    findings.update(
        "selected_change_not_observed:" + name for name in selected - observed
    )
    report = dict(
        contract_version=1,
        status="blocked" if findings else "supplied_inventory_conforms",
        plan_id=plan.plan_id,
        before_sha256=policy.digest(before),
        after_sha256=policy.digest(after),
        namespace=before["namespace"],
        differences=differences,
        findings=sorted(findings),
        update_allowed=False,
        activation_allowed=False,
        complete_candidate=False,
        enforcement_authority=False,
        required_later_gates=saved["result"]["required_later_gates"],
        scope="supplied_inventories_only_not_filesystem_or_semantic_attestation",
    )
    result = dict(report, receipt_sha256=policy.digest(report))
    policy.canonical(result)
    return result
