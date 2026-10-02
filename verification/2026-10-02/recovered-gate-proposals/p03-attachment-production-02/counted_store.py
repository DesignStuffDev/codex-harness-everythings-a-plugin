"""Trusted synthetic upload fixture, distinct from the extracted native package."""
import hashlib
import json
from pathlib import Path
from codex_component_sdk import Plugin

app = Plugin()


@app.handle("attachment_store", "default", "upload")
def upload(params, context):
    path = Path(params["blob"]["path"])
    size = params["blob"]["size_bytes"]
    if not path.is_absolute() or type(size) is not int or not 0 < size <= 4 * 1024 * 1024:
        return {"error": "invalid_attachment"}
    with path.open("rb") as stream:
        data = stream.read(4 * 1024 * 1024 + 1)
    if len(data) != size:
        return {"error": "invalid_attachment"}
    state = context.initialization.state_dir
    index = len(list(state.glob("upload-*.json")))
    if index >= 8:
        raise ValueError("upload count limit")
    fail = context.initialization.config.get("fail") is True
    with (state / f"upload-{index}.json").open("x") as output:
        json.dump({"schema": "attachment-counted-upload-v1", "prepared_sha256": hashlib.sha256(data).hexdigest(),
                   "bytes": len(data), "typed_backend_failure": fail}, output)
    return {"error": "backend"} if fail else {"ok": {"kind": "file", "file_id": "file_attachment_production_fixture"}}
