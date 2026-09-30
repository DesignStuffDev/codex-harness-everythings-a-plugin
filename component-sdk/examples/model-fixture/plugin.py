"""Deterministic streaming transport for offline integration checks, not an LLM."""

from codex_component_sdk import Plugin
from uuid import uuid4

app = Plugin()


@app.handle("model_transport", "default", "model.stream")
def stream(params, context):
    text = context.initialization.config.get(
        "text", "Hello from an independently installed model transport."
    )
    invocation_id = uuid4().hex
    response_id = f"response_component_{invocation_id}"
    item = {
        "type": "message",
        "id": f"msg_component_{invocation_id}",
        "role": "assistant",
        "content": [],
        "phase": "final_answer",
    }
    context.emit({"type": "created", "response_id": response_id})
    context.emit({"type": "output_item_added", "item": item})
    for offset in range(0, len(text), 16):
        context.emit({"type": "output_text_delta", "delta": text[offset : offset + 16]})
    item = dict(item, content=[{"type": "output_text", "text": text}])
    context.emit({"type": "output_item_done", "item": item})
    context.emit({"type": "completed", "response_id": response_id, "end_turn": True})
    return {}
