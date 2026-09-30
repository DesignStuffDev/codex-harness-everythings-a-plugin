"""Tool and context components packaged independently from the Codex host."""

import math
from codex_component_sdk import Plugin, PluginError

app = Plugin()


@app.handle("tool", "calculator", "invoke")
def calculate(params, context):
    arguments = params["arguments"]
    values = arguments.get("values")
    if not isinstance(values, list) or not 1 <= len(values) <= 100:
        raise PluginError("values must contain 1 to 100 finite numbers")
    if any(
        type(value) not in (float, int) or not math.isfinite(value) for value in values
    ):
        raise PluginError("values must contain 1 to 100 finite numbers")
    operation = arguments.get("operation", "sum")
    if operation == "sum":
        result = math.fsum(values)
    elif operation == "product":
        result = math.prod(values)
    else:
        raise PluginError("operation must be sum or product")
    if not math.isfinite(result):
        raise PluginError("result is outside finite numeric range")
    context.emit(
        {"type": "calculation_completed", "operation": operation, "count": len(values)}
    )
    return {"text": str(result), "success": True}


@app.handle("context", "calculator_help", "contribute")
def contribute(params, context):
    return {
        "text": "The calculator tool computes sums and products of up to 100 finite numbers."
    }
