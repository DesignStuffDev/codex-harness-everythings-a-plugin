"""Bounded caller-owned planning responses; no files, subprocesses or activation."""

import json

import selection_plan as policy

PLAN_TOOL = "upstream_selection_plan"
RESTORE_TOOL = "upstream_selection_restore"
MAX_PLAN_REQUEST = 128 * 1024
MAX_RESTORE_REQUEST = 256 * 1024
MAX_PLAN_BYTES = 96 * 1024
MAX_TEXT_BYTES = 8 * 1024
MAX_TOOL_BYTES = 32 * 1024
MAX_REFERENCES = 1024


def bounded_rows(value, limit):
    policy.require(
        type(value) is list and len(value) <= limit, "tool_row_limit_or_shape"
    )
    policy.require(all(type(row) is dict for row in value), "tool_row_limit_or_shape")
    return value


def admit(inputs):
    """Smaller finite cardinalities, not a wall-clock/cancellation guarantee."""
    policy.exact(inputs, "graph profile policy review")
    for value in inputs.values():
        policy.require(type(value) is dict, "invalid_tool_input")
    graph, profile, scopes, review = (
        inputs[key] for key in ("graph", "profile", "policy", "review")
    )
    units = bounded_rows(graph.get("units"), 64)
    changes = bounded_rows(graph.get("changes"), 256)
    rules = bounded_rows(scopes.get("rules"), 512)
    paths = bounded_rows(review.get("changed_paths"), 256)
    references = [
        graph.get("unresolved"),
        profile.get("include"),
        profile.get("exclude"),
        review.get("unresolved"),
    ]
    references += [
        row.get(key)
        for row in units
        for key in ("members", "requires", "conflicts", "unresolved")
    ]
    references += [row.get(key) for row in changes for key in ("plugins", "unresolved")]
    references += [row.get("operations") for row in rules] + [
        row.get("unresolved") for row in paths
    ]
    policy.require(
        all(type(value) is list for value in references), "invalid_tool_input"
    )
    policy.require(
        sum(len(value) for value in references) <= MAX_REFERENCES,
        "tool_reference_limit",
    )


def response(report, success):
    raw = policy.canonical(report)
    policy.require(len(raw) <= MAX_TEXT_BYTES, "tool_output_limit")
    result = dict(text=raw.decode(), success=success)
    policy.require(len(policy.canonical(result)) <= MAX_TOOL_BYTES, "tool_output_limit")
    return result


def failure(code, status="invalid"):
    return response(
        dict(
            contract_version=1,
            status=status,
            diagnostic_code=code,
            planning_only=True,
            state_written=False,
            update_allowed=False,
            activation_allowed=False,
            complete_candidate=False,
            enforcement_authority=False,
        ),
        False,
    )


def invoke(name, params, stop):
    """Cancellation observed before/after pure work only; no durable job admission."""
    try:
        policy.require(name in (PLAN_TOOL, RESTORE_TOOL), "unknown_selection_tool")
        policy.exact(params, "call_id name arguments")
        policy.require(
            type(params["call_id"]) is str
            and 1 <= len(params["call_id"]) <= 256
            and params["name"] == name,
            "invalid_invocation_envelope",
        )
        limit = MAX_PLAN_REQUEST if name == PLAN_TOOL else MAX_RESTORE_REQUEST
        policy.require(len(policy.canonical(params)) <= limit, "tool_input_limit")
        args = params["arguments"]
        if stop.is_set():
            return failure("shutdown_requested", "cancelled")
        if name == PLAN_TOOL:
            policy.revisioned(args, "purpose graph profile policy review")
            policy.require(
                args["purpose"] == "plan_selection", "invalid_selection_purpose"
            )
            inputs = {
                key: args[key] for key in ("graph", "profile", "policy", "review")
            }
            admit(inputs)
            plan = policy.plan_selection(**inputs)
        else:
            policy.revisioned(args, "purpose plan_id plan_json")
            policy.require(
                args["purpose"] == "restore_selection", "invalid_selection_purpose"
            )
            policy.require(type(args["plan_json"]) is str, "invalid_plan_json")
            raw = args["plan_json"].encode()
            policy.require(len(raw) <= MAX_PLAN_BYTES, "tool_plan_limit")
            saved = json.loads(raw)
            policy.exact(saved, "inputs result")
            admit(saved["inputs"])
            plan = policy.restore_plan(raw, args["plan_id"])
        if stop.is_set():
            return failure("shutdown_requested", "cancelled")
        policy.require(len(plan.payload) <= MAX_PLAN_BYTES, "tool_plan_limit")
        result = plan.document()["result"]
        report = {
            key: result[key]
            for key in (
                "contract_version",
                "status",
                "update_allowed",
                "activation_allowed",
                "complete_candidate",
                "enforcement_authority",
                "required_later_gates",
            )
        }
        report.update(
            package_version="0.4.0",
            planning_only=True,
            state_written=False,
            plan_id=plan.plan_id,
            plan_bytes=len(plan.payload),
            plan_json=None,
            plan_json_omitted=True,
            plan_revalidated=name == RESTORE_TOOL,
            selected_count=len(result["selected"]),
            selected_change_count=len(result["selected_changes"]),
            excluded_count=len(result["excluded"]),
            finding_count=len(result["findings"]),
            findings=result["findings"][:16],
            findings_omitted=len(result["findings"]) > 16,
            evidence_scope="caller_owned_plan_only_no_filesystem_or_activation_authority",
        )
        complete = dict(
            report, plan_json=plan.payload.decode(), plan_json_omitted=False
        )
        if len(policy.canonical(complete)) <= MAX_TEXT_BYTES:
            report = complete
        # A blocked plan is successful computation, never permission to update.
        return response(report, True)
    except policy.PolicyError as error:
        return failure(str(error))
    except (ValueError, TypeError, UnicodeError, RecursionError):
        return failure("malformed_or_unsupported_request")
