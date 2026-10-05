"""Build outside the harness source; package the SDK and exact local planner copies."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil

from codex_component_sdk.__main__ import build


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = Path(__file__).resolve().parent
    provenance = json.loads((source / "static/TOOL_PROVENANCE.json").read_text())
    for row in provenance["tools"]:
        path = source / row["packaged_path"]
        if (
            path.is_symlink()
            or hashlib.sha256(path.read_bytes()).hexdigest() != row["sha256"]
        ):
            raise SystemExit(
                "packaged planner differs from recorded source; update provenance explicitly"
            )
    build(source, args.output)
    bootstrap = args.output / "bootstrap.py"
    shutil.copyfile(source / "bootstrap_entry.py", bootstrap)
    bootstrap.chmod(0o700)
    shutil.copyfile(
        source / "capsule_bootstrap.py", args.output / "capsule_bootstrap.py"
    )
    shutil.copyfile(
        source / "overlay_bootstrap.py", args.output / "overlay_bootstrap.py"
    )
    print(args.output.absolute())


if __name__ == "__main__":
    main()
