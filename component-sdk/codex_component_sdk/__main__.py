"""Create and package independent Python component projects using only stdlib."""

import argparse
import json
from pathlib import Path
import re
import shutil
import tempfile
import zipapp


TEMPLATE = """from codex_component_sdk import Plugin

app = Plugin()


@app.handle("tool", "greeting", "invoke")
def greet(params, context):
    name = params["arguments"].get("name", "world")
    return {"text": f"Hello, {name}!", "success": True}
"""


def validate_manifest(manifest):
    """Catch packaging mistakes; the host independently enforces its contracts."""
    if type(manifest.get("api_version")) is not int or manifest["api_version"] != 1:
        raise ValueError("api_version must be 1")
    if not isinstance(manifest.get("id"), str) or not re.fullmatch(
        r"[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}", manifest["id"]
    ):
        raise ValueError("id must be a simple identifier of at most 128 characters")
    if not isinstance(manifest.get("version"), str) or not re.fullmatch(
        r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[\w.-]+)?(?:\+[\w.-]+)?",
        manifest["version"],
    ):
        raise ValueError("version must be semantic versioning")
    if not isinstance(manifest.get("dependencies", {}), dict):
        raise ValueError("dependencies must be an object")
    if not isinstance(manifest.get("args", []), list) or not all(
        isinstance(arg, str) for arg in manifest.get("args", [])
    ):
        raise ValueError("args must be a string array")
    components = manifest.get("components")
    if not isinstance(components, list) or not components:
        raise ValueError("at least one component is required")
    seen = set()
    for component in components:
        if (
            not isinstance(component, dict)
            or type(component.get("contract_version")) is not int
            or component["contract_version"] != 1
        ):
            raise ValueError("each component requires contract_version 1")
        key = (component.get("kind"), component.get("name"))
        if not all(isinstance(part, str) and part for part in key) or key in seen:
            raise ValueError("components need unique nonempty kind/name pairs")
        seen.add(key)


def build(source: Path, output: Path):
    """Build a package directory; never write into an existing destination."""
    source = source.resolve()
    output = output.absolute()
    if output.exists():
        raise ValueError("output already exists")
    if not (source / "plugin.py").is_file():
        raise ValueError("project must contain plugin.py exporting app = Plugin()")
    manifest = json.loads((source / "codex-component.json").read_text())
    validate_manifest(manifest)
    manifest["entrypoint"] = "plugin.pyz"
    manifest.setdefault("args", [])
    manifest.setdefault("dependencies", {})
    source_files = list(source.rglob("*"))
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix="codex-component-build-", dir=output.parent
    ) as temporary:
        temporary = Path(temporary)
        archive_source = temporary / "source"
        archive_source.mkdir()
        for path in source_files:
            relative = path.relative_to(source)
            if any(
                part.startswith(".") or part == "__pycache__" for part in relative.parts
            ):
                continue
            if path.is_symlink():
                raise ValueError("source symlinks are unsupported")
            if not path.is_file() or (
                path.suffix != ".py" and "static" not in relative.parts[:-1]
            ):
                continue
            destination = archive_source / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
        if (archive_source / "codex_component_sdk").exists():
            raise ValueError("codex_component_sdk is reserved for the SDK")
        shutil.copytree(
            Path(__file__).parent,
            archive_source / "codex_component_sdk",
            ignore=shutil.ignore_patterns("__pycache__"),
        )
        (archive_source / "__main__.py").write_text(
            "from plugin import app\napp.run()\n"
        )
        package = temporary / "package"
        package.mkdir()
        zipapp.create_archive(
            archive_source,
            package / "plugin.pyz",
            interpreter="/usr/bin/env python3",
            compressed=True,
        )
        (package / "codex-component.json").write_text(
            json.dumps(manifest, indent=2) + "\n"
        )
        for name in ("LICENSE", "NOTICE", "README.md"):
            if (source / name).is_file():
                shutil.copyfile(source / name, package / name)
        shutil.copyfile(Path(__file__).parent / "LICENSE", package / "SDK-LICENSE")
        shutil.copyfile(Path(__file__).parent / "NOTICE", package / "SDK-NOTICE")
        package.rename(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    create = commands.add_parser("init", help="create a standalone plugin project")
    create.add_argument("directory", type=Path)
    create.add_argument("--id", required=True)
    package = commands.add_parser(
        "build", help="package plugin.py and local Python modules"
    )
    package.add_argument("source", type=Path)
    package.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        if args.command == "build":
            build(args.source, args.output)
            print(args.output.absolute())
        else:
            manifest = {
                "api_version": 1,
                "id": args.id,
                "version": "0.1.0",
                "entrypoint": "plugin.pyz",
                "args": [],
                "dependencies": {},
                "components": [
                    {
                        "kind": "tool",
                        "name": "greeting",
                        "contract_version": 1,
                        "metadata": {
                            "description": "Greet a person",
                            "input_schema": {
                                "type": "object",
                                "properties": {"name": {"type": "string"}},
                                "additionalProperties": False,
                            },
                        },
                    }
                ],
            }
            validate_manifest(manifest)
            args.directory.mkdir(parents=True)
            (args.directory / "plugin.py").write_text(TEMPLATE)
            (args.directory / "codex-component.json").write_text(
                json.dumps(manifest, indent=2) + "\n"
            )
            print(args.directory.absolute())
    except (ValueError, OSError) as error:
        parser.exit(2, f"component packaging failed: {error}\n")


if __name__ == "__main__":
    main()
