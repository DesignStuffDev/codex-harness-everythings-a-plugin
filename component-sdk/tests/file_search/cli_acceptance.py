#!/usr/bin/env python3
"""Exercise one frozen real CLI before/after independently built plugin install.

No Rust build, engine protocol stub, or stand-in search probe runs here. The SDK
manager installs its immutable package and real CLI selection drives the worker.
Wrap with the repository's unchanged Linux subreaper runner.
"""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import sys

from acceptance_support import (
    Runner,
    fingerprint,
    inventory,
    require,
    same_content,
    write_report,
)

PLUGIN = "native.file-search-local"
ROOT = "./project//"


def prepare_fixture(base):
    project = base / "project"
    (project / "src").mkdir(parents=True)
    (project / ".git").mkdir()
    for path in (
        "alpha.txt",
        "alphabet.md",
        "src/alpha_child.txt",
        "src/excluded_alpha.txt",
        "ignored_alpha.txt",
        "naïve.txt",
        "beta.txt",
    ):
        (project / path).write_text("file-search external source acceptance\n")
    (project / ".gitignore").write_text("ignored_alpha.txt\n")
    (project / "workload").mkdir()
    for number in range(2048):
        (project / "workload" / f"misc_{number:05d}.log").touch()
    # The fixture mirrors actual native tests: a repository marker and an ignore
    # file, without creating commits or invoking Git in this acceptance script.


def parsed_rows(text):
    return [json.loads(line) for line in text.splitlines()]


def validate_native_case(label, rows):
    matches = [row for row in rows if "path" in row]
    for match in matches:
        require(match["root"] == ROOT, f"{label}: lexical -C root changed")
        require(
            type(match["score"]) is int and match["score"] >= 0,
            f"{label}: native score missing",
        )
        require(
            match["match_type"] in ("file", "directory"), f"{label}: match type missing"
        )
        require(
            not match["path"].startswith("/"), f"{label}: match path no longer relative"
        )
        if "indices" in match:
            indices = match["indices"]
            require(
                indices == sorted(set(indices)), f"{label}: invalid highlight ordering"
            )
            require(
                all(
                    type(index) is int and 0 <= index < len(match["path"])
                    for index in indices
                ),
                f"{label}: highlight indices are not character offsets",
            )
    paths = {row["path"] for row in matches}
    require("ignored_alpha.txt" not in paths, f"{label}: .gitignore was not honored")
    if label == "lexical-root":
        require(
            paths
            == {
                "alpha.txt",
                "alphabet.md",
                "src/alpha_child.txt",
                "src/excluded_alpha.txt",
            },
            "native fixture produced unexpected alpha matches",
        )
        require(
            all(row.get("indices") for row in matches),
            "native highlight indices missing",
        )
    elif label == "exclude":
        require(
            paths == {"alpha.txt", "alphabet.md", "src/alpha_child.txt"},
            "native exclude option changed",
        )
    elif label == "unicode":
        require(
            paths == {"naïve.txt"} and matches[0].get("indices"),
            "Unicode filename search/highlighting failed",
        )
    elif label == "truncated":
        require(
            len(matches) == 1 and rows[-1] == {"matches_truncated": True},
            "native limit/truncation contract changed",
        )
    elif label == "indices-off":
        require(
            matches and all("indices" not in row for row in matches),
            "disabled highlight indices were serialized",
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--manager", type=Path, required=True)
    parser.add_argument("--build-report", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()
    repo = args.repo.resolve(strict=True)
    cli, manager = args.cli.resolve(strict=True), args.manager.resolve(strict=True)
    proof_path = args.build_report.resolve(strict=True)
    proof = json.loads(proof_path.read_text())
    require(
        proof.get("passed") is True
        and proof.get("evidence_kind") == "independent_source_build",
        "input must be a successful independent source build proof",
    )
    require(
        proof["build_artifact"]["separate_initially_empty_target"] is True
        and proof["build_artifact"]["fresh"] is False,
        "input worker was not compiled with an independent fresh target",
    )
    require(
        proof["source_resolution"]["all_local_paths_inside_export"] is True
        and proof["source_resolution"]["lock_new_identities"] == 0,
        "worker source/dependency provenance was not verified",
    )
    require(
        proof["source_parked_before_runtime"] is True
        and not (Path(proof["artifact_directory"]) / "source").exists(),
        "original worker build-source paths remain active",
    )
    require(
        fingerprint(Path(proof["artifact_directory"]) / "source-inventory.json")
        == proof["exported_source_inventory"],
        "source inventory changed",
    )
    package = Path(proof["package_path"]).resolve(strict=True)
    package_files = inventory(package)
    require(
        package_files == proof["package_files"], "independent worker package changed"
    )
    before = {"cli": fingerprint(cli), "manager": fingerprint(manager)}
    require(
        proof["frozen_binaries_before"] == proof["frozen_binaries_after"] == before,
        "CLI or manager changed since separate worker build",
    )
    work = args.work_dir.resolve()
    require(
        not work.exists() and work != repo and repo not in work.parents,
        "runtime artifacts must be new and outside checkout",
    )
    work.mkdir(parents=True, mode=0o700)
    report_path = work / "cli-acceptance.json"
    report = {
        "passed": False,
        "evidence_kind": "real_cli_independent_native_search",
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "commands": [],
        "build_report": str(proof_path),
        "build_report_fingerprint": fingerprint(proof_path),
        "binaries_before": before,
        "artifact_directory": str(work),
        "tooling": {
            name: fingerprint(Path(__file__).with_name(name))
            for name in ("cli_acceptance.py", "acceptance_support.py")
        },
        "scope": "Standalone one-root CLI only; App Server/TUI/GUI are separate gates.",
        "parity_comparison": "Exact ordered JSON values including score/root/path/indices/truncation; no normalization.",
    }
    runner = Runner(work, report, report_path)
    home, base = work / "home", work / "fixture"
    home.mkdir()
    base.mkdir()
    environment = dict(os.environ, CODEX_HOME=str(home))
    management = [manager, "--codex-home", home]
    cases = {
        "lexical-root": ["--json", "--compute-indices", "-C", ROOT, "alpha"],
        "exclude": [
            "--json",
            "--compute-indices",
            "-C",
            ROOT,
            "-e",
            "src/excluded_alpha.txt",
            "alpha",
        ],
        "unicode": ["--json", "--compute-indices", "-C", ROOT, "na"],
        "truncated": ["--json", "--compute-indices", "-C", ROOT, "-l", "1", "alpha"],
        "indices-off": ["--json", "-C", ROOT, "alpha"],
    }
    try:
        prepare_fixture(base)
        report["fixture_files"] = inventory(base)
        baseline = {}
        for label, arguments in cases.items():
            output, _, _ = runner.run(
                "native-" + label, [cli, *arguments], cwd=base, environment=environment
            )
            baseline[label] = parsed_rows(output)
            validate_native_case(label, baseline[label])
        no_pattern, no_pattern_stderr, _ = runner.run(
            "native-no-pattern",
            [cli, "--json", "-C", ROOT],
            cwd=base,
            environment=environment,
        )
        require(
            "No search pattern specified" in no_pattern_stderr,
            "legacy no-pattern diagnostic missing",
        )

        source_package = work / "install-input"
        shutil.copytree(package, source_package)
        require(
            same_content(inventory(source_package), package_files),
            "install input copy differs",
        )
        runner.run(
            "install",
            [*management, "install", source_package],
            cwd=base,
            environment=environment,
        )
        runner.run(
            "select",
            [*management, "select", "file_search", "default", PLUGIN],
            cwd=base,
            environment=environment,
        )
        output, _, _ = runner.run(
            "list-selected", [*management, "list"], cwd=base, environment=environment
        )
        settings = json.loads(output)
        require(
            settings["selections"].get("file_search:default") == PLUGIN,
            "actual catalog does not select the external worker",
        )
        installed = home / "components/objects" / settings["installed"][PLUGIN]
        require(
            same_content(inventory(installed), package_files),
            "immutable installed package differs",
        )
        manifest = json.loads((installed / "codex-component.json").read_text())
        require(
            manifest["id"] == PLUGIN
            and manifest["api_version"] == 1
            and any(
                item["kind"] == "file_search"
                and item["name"] == "default"
                and item["contract_version"] == 1
                for item in manifest["components"]
            ),
            "installed search manifest is incompatible",
        )
        worker = (installed / manifest["entrypoint"]).resolve(strict=True)
        require(
            installed in worker.parents, "installed worker escapes immutable object"
        )
        report["installed_worker"] = {
            "path": str(worker),
            "fingerprint": fingerprint(worker),
        }
        report["installed_package_files"] = inventory(installed)
        source_package.rename(work / "install-input.parked")
        report["install_input_parked_before_execution"] = True
        observed = []
        for label, arguments in cases.items():
            output, _, entry = runner.run(
                "external-" + label,
                [cli, *arguments],
                cwd=base,
                environment=environment,
            )
            require(
                parsed_rows(output) == baseline[label],
                f"{label}: external/native output differs",
            )
            observed.extend(
                item
                for item in entry["observed_processes"]
                if item.get("executable") == str(worker)
            )
        require(observed, "actual installed native worker executable was not observed")
        report["observed_installed_worker_pids"] = sorted(
            {item["pid"] for item in observed}
        )
        report["parity_cases"] = list(cases)
        output, _, entry = runner.run(
            "external-cli-ctrl-c",
            [cli, *cases["lexical-root"]],
            cwd=base,
            environment=environment,
            expected_status=130,
            interrupt_when_executable=worker,
        )
        require(
            not output.strip(), "interrupted search printed partial successful output"
        )
        report["real_cli_ctrl_c"] = {
            "exit_status": entry["status"],
            "all_observed_pids_absent": entry["all_observed_pids_absent"],
            "signal_target": "CLI only, after observing installed worker executable",
            "no_partial_output": True,
        }

        # Exercise failure in the actual installed native worker, not a stub.
        # Only this isolated acceptance CODEX_HOME is edited; package code remains immutable.
        settings_path = home / "components/config.json"
        good_settings = settings_path.read_text()
        changed = json.loads(good_settings)
        changed.setdefault("config", {})[PLUGIN] = {
            "unsupported_acceptance_option": True
        }
        settings_path.write_text(json.dumps(changed, indent=2) + "\n")
        output, errors, _ = runner.run(
            "selected-failure-no-fallback",
            [cli, *cases["lexical-root"]],
            cwd=base,
            environment=environment,
            expected_status="nonzero",
        )
        require(
            not output.strip(),
            "selected worker error produced successful native fallback output",
        )
        require(
            "unsupported field or type" in errors,
            "selected failure did not preserve native configuration rejection",
        )
        report["selected_failure_no_fallback"] = True
        output, errors, entry = runner.run(
            "selected-failure-no-pattern",
            [cli, "--json", "-C", ROOT],
            cwd=base,
            environment=environment,
        )
        require(
            output == no_pattern and errors == no_pattern_stderr,
            "no-pattern listing did not bypass selected provider failure",
        )
        require(
            not any(
                item.get("executable") == str(worker)
                for item in entry["observed_processes"]
            ),
            "no-pattern listing unnecessarily started selected worker",
        )
        settings_path.write_text(good_settings)
        runner.run(
            "remove", [*management, "remove", PLUGIN], cwd=base, environment=environment
        )
        output, _, _ = runner.run(
            "list-removed", [*management, "list"], cwd=base, environment=environment
        )
        settings = json.loads(output)
        require(
            PLUGIN not in settings["installed"]
            and PLUGIN not in settings["enabled"]
            and "file_search:default" not in settings["selections"],
            "remove left activation behind",
        )
        require(
            inventory(installed) == report["installed_package_files"],
            "remove changed the preserved immutable object",
        )
        for label, arguments in cases.items():
            output, _, entry = runner.run(
                "removed-native-" + label,
                [cli, *arguments],
                cwd=base,
                environment=environment,
            )
            require(
                parsed_rows(output) == baseline[label],
                f"{label}: removal did not restore native selection",
            )
            require(
                not any(
                    item.get("executable") == str(worker)
                    for item in entry["observed_processes"]
                ),
                "removed worker executed on a future startup",
            )
        report["removal_affects_future_selection"] = True
        report["immutable_object_retained"] = True
        require(
            inventory(package) == package_files, "original package evidence changed"
        )
        report["passed"] = True
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        report["binaries_after"] = {
            "cli": fingerprint(cli),
            "manager": fingerprint(manager),
        }
        if report["binaries_after"] != before:
            report["passed"] = False
            report["error"] = (
                "CLI or component manager changed during runtime acceptance"
            )
        write_report(report_path, report)
    print(str(report_path))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
