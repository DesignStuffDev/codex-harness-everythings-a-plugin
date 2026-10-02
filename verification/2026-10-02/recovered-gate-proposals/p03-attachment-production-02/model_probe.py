"""Deterministic image-only provider probe; retain no raw request/image/session ID."""
import base64
import hashlib
import json
from codex_component_sdk import Plugin

app = Plugin()
FILE_ID = "file_attachment_production_fixture"


@app.handle("model_transport", "default", "model.stream")
def stream(params, context):
    state = context.initialization.state_dir
    index = len(list(state.glob("request-*.json")))
    if index >= 16:
        raise ValueError("request limit")
    images = []
    for item in params["request"]["input"]:
        for part in item.get("content", []) if isinstance(item.get("content"), list) else []:
            if part.get("type") != "input_image":
                continue
            if len(images) >= 16:
                raise ValueError("image count limit")
            if "file_id" in part:
                images.append({"kind": "file", "fixture_file_id": part["file_id"] == FILE_ID})
            else:
                url = part.get("image_url", "")
                if not isinstance(url, str) or len(url) > 4 * 1024 * 1024 or not url.startswith("data:image/"):
                    raise ValueError("inline image contract")
                header, encoded = url.split(",", 1)
                if not header.endswith(";base64"):
                    raise ValueError("inline encoding")
                data = base64.b64decode(encoded, validate=True)
                images.append({"kind": "inline", "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)})
    with (state / f"request-{index}.json").open("x") as output:
        json.dump({"schema": "attachment-model-probe-v1", "images": images}, output)
    response_id = f"attachment-proof-{index}"
    item = {"type": "message", "id": f"attachment-message-{index}", "role": "assistant", "content": [], "phase": "final_answer"}
    context.emit({"type": "created", "response_id": response_id})
    context.emit({"type": "output_item_added", "item": item})
    for delta in ("attachment-proof ", "complete"):
        context.emit({"type": "output_text_delta", "delta": delta})
    context.emit({"type": "output_item_done", "item": dict(item, content=[{"type": "output_text", "text": "attachment-proof complete"}])})
    context.emit({"type": "completed", "response_id": response_id, "end_turn": True})
    return {}
