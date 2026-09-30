"""Export a standalone native plugin project, or assemble its built executable."""

import argparse
import json
from pathlib import Path
import shutil
import tomllib

COMPONENT = Path(__file__).resolve().parents[1]
RUST = COMPONENT.parent
REPO = RUST.parent


def copy_notices(destination):
    for name in ("LICENSE", "NOTICE"):
        shutil.copyfile(REPO / name, destination / name)
    shutil.copyfile(COMPONENT / "README.md", destination / "README.md")


def export(directory):
    """Export exact native implementation sources with no host source dependencies."""
    workspace = tomllib.loads((RUST / "Cargo.toml").read_text())["workspace"]
    dependencies = workspace["dependencies"]

    def version(name):
        value = dependencies[name]
        return value["version"] if isinstance(value, dict) else value

    def manifest(name, extra):
        return (
            f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2024"\n'
            f'license = "Apache-2.0"\n\n[dependencies]\n{extra}\n'
        )

    directory.mkdir(parents=True, exist_ok=False)
    for crate, extra in (
        (
            "attachment-store-api",
            f'serde = {{version = "{version("serde")}", features = ["derive"]}}',
        ),
        (
            "attachment-store-inline",
            f'codex-attachment-store-api = {{path = "../attachment-store-api"}}\ntracing = "{version("tracing")}"',
        ),
        (
            "component-api",
            f'serde = {{version = "{version("serde")}", features = ["derive"]}}\nserde_json = "{version("serde_json")}"',
        ),
    ):
        destination = directory / crate
        destination.mkdir(parents=True)
        (destination / "Cargo.toml").write_text(manifest("codex-" + crate, extra))
        shutil.copytree(RUST / crate / "src", destination / "src")
    extra = (
        'codex-attachment-store-api = {path = "attachment-store-api"}\n'
        'codex-attachment-store-inline = {path = "attachment-store-inline"}\n'
        'codex-component-api = {path = "component-api"}\n'
        f'anyhow = "{version("anyhow")}"\nserde_json = "{version("serde_json")}"\n'
        f'serde = {{version = "{version("serde")}", features = ["derive"]}}\n'
        f'tokio = {{version = "{version("tokio")}", features = ["fs", "io-util", "macros", "rt-multi-thread"]}}'
    )
    (directory / "Cargo.toml").write_text(
        manifest("codex-attachment-store-component", extra)
        + '\n[[bin]]\nname = "codex-attachment-inline-component"\npath = "src/bin/inline.rs"\n\n[workspace]\n'
    )
    (directory / "src" / "bin").mkdir(parents=True)
    for name in ("native.rs", "protocol.rs", "bin/inline.rs"):
        shutil.copyfile(COMPONENT / "src" / name, directory / "src" / name)
    (directory / "src" / "lib.rs").write_text(
        "mod native;\npub mod protocol;\npub use native::run_native_stdio;\n"
    )
    # Retain pinned registry versions where possible; Cargo removes unrelated entries.
    shutil.copyfile(RUST / "Cargo.lock", directory / "Cargo.lock")
    copy_notices(directory)


def assemble(binary, directory):
    directory.mkdir(parents=True, exist_ok=False)
    shutil.copy2(binary, directory / binary.name)
    manifest = {
        "api_version": 1,
        "id": "codex.attachment-inline",
        "version": "0.1.0",
        "entrypoint": binary.name,
        "args": [],
        "dependencies": {},
        "components": [
            {"kind": "attachment_store", "name": "default", "contract_version": 1}
        ],
    }
    (directory / "codex-component.json").write_text(
        json.dumps(manifest, indent=2) + "\n"
    )
    copy_notices(directory)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    project = commands.add_parser("export")
    project.add_argument("directory", type=Path)
    package = commands.add_parser("assemble")
    package.add_argument("binary", type=Path)
    package.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.command == "export":
        export(args.directory.absolute())
    else:
        assemble(args.binary.resolve(strict=True), args.output.absolute())


if __name__ == "__main__":
    main()
