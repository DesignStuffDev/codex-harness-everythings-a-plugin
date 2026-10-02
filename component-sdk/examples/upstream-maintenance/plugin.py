"""An additive maintenance-review tool on the installed component protocol v1."""

import json

from codex_component_sdk import Plugin, PluginError
from maintenance_review import review_request

app = Plugin()


@app.handle("tool", "upstream_impact_review", "invoke")
def plan(params, context):
    if (
        not isinstance(params, dict)
        or set(params) != {"call_id", "name", "arguments"}
        or not isinstance(params["call_id"], str)
        or not 1 <= len(params["call_id"]) <= 256
        or params["name"] != "upstream_impact_review"
        or not isinstance(params["arguments"], dict)
    ):
        raise PluginError("invalid tool-v1 invocation envelope")
    report = review_request(params["arguments"], context.watch_shutdown())
    # success describes report computation, never permission to update.
    return {
        "text": json.dumps(report, sort_keys=True, separators=(",", ":")),
        "success": report["status"] == "review_required",
    }
