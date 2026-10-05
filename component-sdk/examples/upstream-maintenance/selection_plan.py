"""Pure supplied-evidence plans; no filesystem, sandbox or activation authority."""

from dataclasses import dataclass
import hashlib
import json
import re

MAX_BYTES, MAX_ROWS = 4 * 1024 * 1024, 4096
CATEGORIES = {"implementation", "config", "contracts", "dependencies", "schemas"}
OPERATIONS = {"add", "modify", "delete", "mode"}
UNIT = r"(?:component|feature|coherent_group):[a-z0-9][a-z0-9._-]{0,95}"
LABEL = r"[a-z0-9][a-z0-9._-]{0,127}"
LATER_GATES = (
    "trusted_inventory_and_scope_enforcement",
    "isolated_build_and_regressions",
    "consistent_recovery_restore",
    "ab_staging_resource_reservation",
    "candidate_external_effects_fenced",
    "coexistence_or_lossless_handoff",
    "health_and_atomic_route_switch",
    "session_generation_pinning_and_drain",
    "post_update_write_preserving_rollback",
    "external_recovery_without_host",
)


class PolicyError(ValueError):
    """Malformed/unsupported input; messages are fixed diagnostic codes."""


def require(condition, code="invalid_policy_input"):
    if not condition:
        raise PolicyError(code)


def canonical(value):
    try:
        raw = json.dumps(
            value, sort_keys=True, separators=(",", ":"), allow_nan=False
        ).encode()
    except (ValueError, TypeError, RecursionError, UnicodeError):
        raise PolicyError("non_json_input") from None
    require(len(raw) <= MAX_BYTES, "input_byte_limit")
    return raw


def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()


def exact(value, names):
    require(type(value) is dict and set(value) == set(names.split()))


def text(value, pattern=LABEL):
    require(type(value) is str and re.fullmatch(pattern, value) is not None)


def sequence(value, limit=MAX_ROWS):
    require(type(value) is list and len(value) <= limit)


def labels(value, pattern=LABEL):
    sequence(value)
    for item in value:
        text(item, pattern)
    require(len(set(value)) == len(value), "duplicate_identity")


def path(value):
    require(type(value) is str and 0 < len(value) <= 1024)
    require(
        not any(char in value for char in "\\:\x00")
        and all(ord(c) >= 32 for c in value)
    )
    require(
        all(part not in ("", ".", "..") for part in value.split("/")),
        "noncanonical_path",
    )


def entry(value):
    if value is not None:
        exact(value, "blob mode")
        text(value["blob"], r"[a-f0-9]{40}")
        require(value["mode"] in ("100644", "100755"), "unsupported_entry_mode")


def revisioned(value, names):
    exact(value, "contract_version " + names)
    require(
        type(value["contract_version"]) is int and value["contract_version"] == 1,
        "unsupported_policy_contract",
    )


def validate_graph(graph):
    revisioned(graph, "bindings units changes unresolved")
    exact(
        graph["bindings"],
        "base_revision upstream_revision custom_revision composition_sha256 before_inventory_sha256 lineage_sha256 review_sha256 capsule_sha256",
    )
    for key, value in graph["bindings"].items():
        text(value, r"[a-f0-9]{40}" if key.endswith("revision") else r"[a-f0-9]{64}")
    labels(graph["unresolved"])
    sequence(graph["units"], 1024)
    units = {}
    for unit in graph["units"]:
        exact(unit, "id members requires conflicts unresolved")
        text(unit["id"], UNIT)
        require(unit["id"] not in units, "duplicate_identity")
        for key in ("members", "requires", "conflicts"):
            labels(unit[key], UNIT)
        labels(unit["unresolved"])
        if unit["id"].startswith("feature:"):
            require(not unit["members"], "feature_must_be_leaf")
        if unit["id"].startswith("coherent_group:"):
            require(bool(unit["members"]), "empty_coherent_group")
        units[unit["id"]] = unit
    for unit in units.values():
        require(
            set(unit["members"] + unit["requires"] + unit["conflicts"]) <= units.keys(),
            "unknown_catalog_reference",
        )
    pending = {
        name: set(unit["members"] + unit["requires"]) for name, unit in units.items()
    }
    while pending:
        leaves = {name for name, deps in pending.items() if not deps}
        require(bool(leaves), "cyclic_catalog")
        pending = {
            name: deps - leaves for name, deps in pending.items() if name not in leaves
        }
    sequence(graph["changes"])
    changes, paths = {}, set()
    for change in graph["changes"]:
        exact(change, "id unit plugins path category before origin unresolved")
        text(change["id"])
        require(change["id"] not in changes, "duplicate_identity")
        text(change["unit"], UNIT)
        require(
            change["unit"] in units and not change["unit"].startswith("coherent_group:")
        )
        labels(change["plugins"])
        require(bool(change["plugins"]), "owner_required")
        path(change["path"])
        require(change["path"].casefold() not in paths, "ambiguous_change_path")
        paths.add(change["path"].casefold())
        text(change["category"])
        require(change["category"] in CATEGORIES)
        entry(change["before"])
        exact(change["origin"], "revision path blob symbol")
        text(change["origin"]["revision"], r"[a-f0-9]{40}")
        require(
            change["origin"]["revision"] == graph["bindings"]["base_revision"],
            "origin_revision_mismatch",
        )
        path(change["origin"]["path"])
        if change["origin"]["blob"] is not None:
            text(change["origin"]["blob"], r"[a-f0-9]{40}")
        symbol = change["origin"]["symbol"]
        require(
            symbol is None
            or type(symbol) is str
            and 0 < len(symbol) <= 512
            and all(ord(c) >= 32 for c in symbol)
        )
        require(
            change["origin"]["blob"] is not None or symbol is None,
            "symbol_without_original_object",
        )
        labels(change["unresolved"])
        changes[change["id"]] = change
    canonical(graph)
    return units, changes


def impact_findings(review):
    """Retain actual legacy planner gaps; never promote historical owner labels."""
    require(
        type(review) is dict
        and review.get("schema") == "codex-upstream-offline-impact-v1"
    )
    require(review.get("status") in ("unresolved", "invalid"))
    labels(review.get("unresolved"))
    sequence(review.get("changed_paths"))
    require(type(review.get("summary")) is dict)
    findings = {"impact:" + value for value in review["unresolved"]}
    if review["status"] == "invalid" or review.get("invalid"):
        findings.add("impact:invalid")
    summary = review["summary"]
    require(
        all(
            type(summary.get(key)) is int and summary[key] >= 0
            for key in ("changed_paths_total", "changed_paths_reported")
        )
    )
    if (
        summary.get("path_inventory_complete") is not True
        or summary.get("changed_paths_total") != len(review["changed_paths"])
        or summary.get("changed_paths_reported") != len(review["changed_paths"])
    ):
        findings.add("impact:incomplete_inventory")
    seen = set()
    for row in review["changed_paths"]:
        require(type(row) is dict)
        path(row.get("path"))
        require(row["path"] not in seen, "duplicate_identity")
        seen.add(row["path"])
        labels(row.get("unresolved"))
        findings.update("impact:" + value for value in row["unresolved"])
    canonical(review)
    return findings, seen


@dataclass(frozen=True)
class SelectionPlan:
    """Immutable canonical bytes; caller persistence and authenticity are separate."""

    payload: bytes

    @property
    def plan_id(self):
        return hashlib.sha256(self.payload).hexdigest()

    def document(self):
        try:
            return json.loads(self.payload)
        except (ValueError, TypeError, UnicodeError, RecursionError):
            raise PolicyError("invalid_plan_json") from None


def plan_selection(graph, profile, policy, review):
    """Validate explicit choices; expand membership, never silently add dependencies."""
    for value in (graph, profile, policy, review):
        canonical(value)
    units, changes = validate_graph(graph)
    revisioned(profile, "id revision catalog_sha256 include exclude")
    text(profile["id"])
    require(type(profile["revision"]) is int and profile["revision"] > 0)
    text(profile["catalog_sha256"], r"[a-f0-9]{64}")
    for key in ("include", "exclude"):
        labels(profile[key], UNIT)
    revisioned(policy, "id revision rules")
    text(policy["id"])
    require(type(policy["revision"]) is int and policy["revision"] > 0)
    sequence(policy["rules"])
    rules = {}
    for rule in policy["rules"]:
        exact(
            rule,
            "plugin change path category before origin_sha256 operations granularity",
        )
        for key in ("plugin", "change"):
            text(rule[key])
        path(rule["path"])
        text(rule["category"])
        require(rule["category"] in CATEGORIES)
        entry(rule["before"])
        text(rule["origin_sha256"], r"[a-f0-9]{64}")
        labels(rule["operations"])
        require(bool(rule["operations"]) and set(rule["operations"]) <= OPERATIONS)
        require(rule["granularity"] == "file", "unsupported_scope_granularity")
        key = (rule["plugin"], rule["change"])
        require(key not in rules, "duplicate_scope")
        require(
            rule["change"] in changes
            and rule["plugin"] in changes[rule["change"]]["plugins"],
            "unknown_scope_owner",
        )
        rules[key] = rule
    findings, origin_paths = impact_findings(review)
    if digest(review) != graph["bindings"]["review_sha256"]:
        findings.add("review_binding_mismatch")
    if {change["origin"]["path"] for change in changes.values()} != origin_paths:
        findings.add("mapped_change_inventory_incomplete")
    if profile["catalog_sha256"] != digest(graph):
        findings.add("stale_profile")
    findings.update("catalog:" + value for value in graph["unresolved"])
    if set(profile["include"]) & set(profile["exclude"]):
        findings.add("selection_overlap")

    def expand(names):
        result, todo = set(), list(names)
        while todo:
            name = todo.pop()
            if name not in units:
                findings.add("unknown_selection:" + name)
            elif name not in result:
                result.add(name)
                todo.extend(units[name]["members"])
        return result

    selected = expand(profile["include"]) - expand(profile["exclude"])
    # Fully selected atomic groups also carry their constraints when the user
    # selected each member individually. This adds no new leaf selections.
    while True:
        groups = {
            name
            for name, unit in units.items()
            if name.startswith("coherent_group:")
            and expand(unit["members"]) <= selected
        }
        if groups <= selected:
            break
        selected.update(groups)
    active = selected | {
        name
        for name, unit in units.items()
        if name.startswith("component:") and expand(unit["members"]) & selected
    }
    for name, unit in units.items():
        if name.startswith("coherent_group:"):
            members = expand(unit["members"])
            if (name in selected or members & selected) and not members <= selected:
                findings.add("partial_coherent_group:" + name)
        if name in active:
            findings.update("unit:" + value for value in unit["unresolved"])
            for dependency in unit["requires"]:
                if not expand([dependency]) <= selected:
                    findings.add("missing_dependency:" + dependency)
            if set(unit["conflicts"]) & active:
                findings.add("conflicting_selection:" + name)
    selected_changes = sorted(
        name for name, change in changes.items() if change["unit"] in selected
    )
    if not selected_changes:
        findings.add("no_changes_selected")
    for name in selected_changes:
        change = changes[name]
        findings.update("mapping:" + value for value in change["unresolved"])
        for plugin in change["plugins"]:
            rule = rules.get((plugin, name))
            if (
                rule is None
                or any(
                    rule[key] != change[key] for key in ("path", "category", "before")
                )
                or rule["origin_sha256"] != digest(change["origin"])
            ):
                findings.add("scope_binding_missing:" + name + ":" + plugin)
    result = dict(
        contract_version=1,
        status="blocked" if findings else "scoped_plan",
        selected=sorted(selected),
        active_constraints=sorted(active),
        selected_changes=selected_changes,
        excluded=sorted(expand(profile["exclude"])),
        findings=sorted(findings),
        update_allowed=False,
        activation_allowed=False,
        complete_candidate=False,
        enforcement_authority=False,
        required_later_gates={gate: "pending" for gate in LATER_GATES},
    )
    return SelectionPlan(
        canonical(
            dict(
                inputs=dict(graph=graph, profile=profile, policy=policy, review=review),
                result=result,
            )
        )
    )


def restore_plan(payload, expected_plan_id):
    """Revalidate persisted bytes; hash identity is not publisher authentication."""
    require(type(payload) is bytes and len(payload) <= MAX_BYTES)
    text(expected_plan_id, r"[a-f0-9]{64}")
    saved = SelectionPlan(payload).document()
    exact(saved, "inputs result")
    exact(saved["inputs"], "graph profile policy review")
    plan = plan_selection(**saved["inputs"])
    require(plan.payload == payload, "altered_plan")
    require(plan.plan_id == expected_plan_id, "stale_plan_evidence")
    return plan
