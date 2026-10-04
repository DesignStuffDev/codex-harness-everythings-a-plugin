"""Chosen-local-revision impact review; no update, fetch or deployment authority."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

import owned_git
import plan_upstream_impact as planner
import validate_checkpoint_lineage as lineage

CONTRACT_VERSION = 1
MAX_REQUEST = 32 * 1024
MAX_RESULT = 8 * 1024
JOB_SECONDS = 90
TOOL_HASHES = {
    "plan_upstream_impact.py": "67a38e355fcfb2041d35793288317cdc58617b8093cd6c9b161844f986c4a19e",
    "validate_checkpoint_lineage.py": "31d61fbcb28f5fcaf9fd298daac95bd286c7102c211b0251dab00f381cc9b91c",
}
REQUIRED = {
    "contract_version",
    "repository",
    "lineage_index",
    "lineage_sha256",
    "candidate_revision",
    "composition_revision",
}
OPTIONAL = {"publication_receipt", "publication_sha256"}


class ReviewUnavailable(ValueError):
    """A stable diagnostic code, never command stderr or credential-bearing text."""


def canonical(value):
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode()


def exact_file(name, expected):
    if not isinstance(name, str) or len(name) > 4096 or not Path(name).is_absolute():
        raise ReviewUnavailable("absolute_input_path_required")
    path = Path(name)
    if path.is_symlink() or not path.is_file():
        raise ReviewUnavailable("regular_input_file_required")
    if (
        not isinstance(expected, str)
        or len(expected) != 64
        or any(c not in "0123456789abcdef" for c in expected)
    ):
        raise ReviewUnavailable("exact_input_digest_required")
    raw = lineage.read_limited(path)
    if hashlib.sha256(raw).hexdigest() != expected:
        raise ReviewUnavailable("input_digest_mismatch")
    return raw


class LocalObjects(lineage.GitObjects):
    """The existing read-only Git adapter with a whole-job admission deadline.

    Each owned Git call retains the 30-second limit and observes active
    cancellation. Linux direct-child parent-death protection complements the
    host's process-group cleanup. Output bounds remain post-capture.
    """

    def __init__(self, repository, stop):
        super().__init__(repository)
        self.stop = stop
        self.deadline = time.monotonic() + JOB_SECONDS
        self.terminal_failure = None

    def command(self, *args):
        if self.terminal_failure is not None:
            raise owned_git.OwnedGitError(self.terminal_failure)
        try:
            result = owned_git.capture(
                ["--no-pager", "-C", str(self.repository), *args],
                env=dict(
                    os.environ,
                    GIT_OPTIONAL_LOCKS="0",
                    GIT_NO_LAZY_FETCH="1",
                    GIT_NO_REPLACE_OBJECTS="1",
                ),
                stop=self.stop,
                deadline=self.deadline,
            )
        except owned_git.OwnedGitError as error:
            self.terminal_failure = str(error)
            raise
        if result.returncode or len(result.stdout) > lineage.MAX_INPUT:
            raise ReviewUnavailable("local_object_unavailable_or_limit")
        return result.stdout


def review_request(request, stop=None, *, full_report=False):
    """Return a bounded, content-addressed review report; never authorize an update."""
    base = {
        "schema": "codex-installed-upstream-impact-review-v1",
        "contract_version": CONTRACT_VERSION,
        "status": "invalid",
        "update_allowed": False,
        "planning_only": True,
        "fetch_source_mutation_or_activation_performed": False,
        "required_next_action": "Correct the exact input bindings and invoke plan again.",
    }
    envelope = dict(base)
    output_limit = lineage.MAX_INPUT if full_report else MAX_RESULT
    try:
        if (
            not isinstance(request, dict)
            or not REQUIRED <= set(request)
            or set(request) - REQUIRED - OPTIONAL
        ):
            raise ReviewUnavailable("invalid_request_shape")
        encoded = canonical(request)
        if (
            len(encoded) > MAX_REQUEST
            or type(request["contract_version"]) is not int
            or request["contract_version"] != CONTRACT_VERSION
        ):
            raise ReviewUnavailable("unsupported_contract_or_request_limit")
        for name in ("candidate_revision", "composition_revision"):
            if not isinstance(request[name], str) or not lineage.OID.fullmatch(
                request[name]
            ):
                raise ReviewUnavailable("exact_commit_id_required")
        repository = request["repository"]
        if (
            not isinstance(repository, str)
            or len(repository) > 4096
            or not Path(repository).is_absolute()
            or not Path(repository).is_dir()
        ):
            raise ReviewUnavailable("absolute_local_repository_required")
        if ("publication_receipt" in request) != ("publication_sha256" in request):
            raise ReviewUnavailable("publication_path_and_digest_required_together")
        for module in (planner, lineage):
            raw = module.__loader__.get_data(module.__file__)
            if (
                hashlib.sha256(raw).hexdigest()
                != TOOL_HASHES[Path(module.__file__).name]
            ):
                raise ReviewUnavailable("packaged_planner_digest_mismatch")
        index_raw = exact_file(request["lineage_index"], request["lineage_sha256"])
        publication = (
            exact_file(request["publication_receipt"], request["publication_sha256"])
            if "publication_receipt" in request
            else None
        )
        stop = stop or threading.Event()
        report = planner.plan(
            lineage.json_bytes(index_raw),
            LocalObjects(repository, stop),
            request["candidate_revision"],
            request["composition_revision"],
            publication,
        )
        envelope.update(
            status="invalid" if report["invalid"] else "review_required",
            request_sha256=hashlib.sha256(encoded).hexdigest(),
            index_sha256=request["lineage_sha256"],
            planner_sha256=TOOL_HASHES["plan_upstream_impact.py"],
            validator_sha256=TOOL_HASHES["validate_checkpoint_lineage.py"],
            candidate_revision=request["candidate_revision"],
            composition_revision=request["composition_revision"],
            report_sha256=hashlib.sha256(canonical(report)).hexdigest(),
            report=report,
            required_next_action="Resolve every unresolved finding with current source ownership and gate evidence; this component cannot approve or apply updates.",
            standalone_recovery_scope="Read-only inspection remains callable without the host. Deployment or state restoration is not implemented.",
        )
        if stop.is_set():
            envelope["status"] = "cancelled"
            envelope["required_next_action"] = (
                "Review the incomplete report and explicitly invoke a new plan when ready. No update was applied."
            )
        if publication is not None:
            envelope["publication_receipt_sha256"] = request["publication_sha256"]
        if len(canonical(envelope)) > output_limit - 128:
            envelope.pop("report")
            envelope.update(
                detailed_report_omitted=True,
                unresolved=["component_report_limit_reached"],
                required_next_action="Use the packaged bootstrap with --full-report to inspect the wider bounded report. Any remaining omitted evidence blocks approval.",
            )
            envelope["report_status"] = report["status"]
            envelope["report_unresolved"] = []
            for reason in report.get("unresolved", [])[:32]:
                if not isinstance(reason, str) or len(reason) > 256:
                    continue
                envelope["report_unresolved"].append(reason)
                if len(canonical(envelope)) > output_limit - 128:
                    envelope["report_unresolved"].pop()
                    break
            envelope["changed_paths_reported"] = []
            for row in report.get("changed_paths", [])[:4]:
                envelope["changed_paths_reported"].append(row)
                if len(canonical(envelope)) > output_limit - 128:
                    envelope["changed_paths_reported"].pop()
                    break
        envelope["plan_id"] = hashlib.sha256(
            canonical(
                {
                    "request_sha256": envelope["request_sha256"],
                    "report_sha256": envelope["report_sha256"],
                    "planner_sha256": envelope["planner_sha256"],
                    "validator_sha256": envelope["validator_sha256"],
                    "contract_version": CONTRACT_VERSION,
                }
            )
        ).hexdigest()
    except (
        ValueError,
        TypeError,
        KeyError,
        OSError,
        RecursionError,
        subprocess.TimeoutExpired,
        owned_git.OwnedGitError,
    ) as error:
        # Do not expose a partly assembled report or retain its success status.
        envelope = dict(
            base,
            diagnostic_code=str(error)
            if isinstance(error, (ReviewUnavailable, owned_git.OwnedGitError))
            else "invalid_or_unavailable_input",
        )
        if isinstance(error, owned_git.OwnedGitError):
            if str(error) in {"cancelled_or_deadline_reached", "local_object_timeout"}:
                envelope.update(
                    status="cancelled",
                    required_next_action="No update was applied. Explicitly invoke a new review after resolving the cancellation or deadline.",
                )
            else:
                envelope["required_next_action"] = (
                    "Direct Git cleanup was not confirmed. Inspect owned process state before retrying; no update was applied."
                )
    if len(canonical(envelope)) > output_limit:
        envelope = dict(base, diagnostic_code="component_report_limit_reached")
    return envelope


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "request",
        type=Path,
        help="Exact JSON request; no host or installed composition is required",
    )
    parser.add_argument(
        "--full-report",
        action="store_true",
        help="Allow up to the planner's existing 8MiB bound outside the model/tool protocol",
    )
    args = parser.parse_args()
    try:
        with args.request.open("rb") as stream:
            raw = stream.read(MAX_REQUEST + 1)
        if len(raw) > MAX_REQUEST:
            raise ValueError("request_limit")
        request = lineage.json_bytes(raw)
        result = review_request(request, full_report=args.full_report)
    except (ValueError, OSError, RecursionError):
        result = {
            "schema": "codex-installed-upstream-impact-review-v1",
            "status": "invalid",
            "update_allowed": False,
            "diagnostic_code": "invalid_or_unavailable_request",
        }
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 1 if result["status"] == "invalid" else 2


if __name__ == "__main__":
    sys.exit(main())
