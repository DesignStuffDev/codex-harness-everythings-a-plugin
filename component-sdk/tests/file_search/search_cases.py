"""Actual App Server search scenarios; no model/search stand-ins."""

import hashlib
import time

from acceptance_support import require
from rpc_support import SEARCH_METHODS

ROOT = "./project//"
OTHER_ROOT = "./other//"
PLUGIN = "native.file-search-local"


def prepare_fixture(base):
    project = base / "project"
    (project / "src").mkdir(parents=True)
    (project / ".git").mkdir()
    for relative in (
        "alpha.txt",
        "alphabet.md",
        "src/alpha_child.txt",
        "ignored_alpha.txt",
        "naïve.txt",
        "beta.txt",
    ):
        (project / relative).write_text("actual App Server native search acceptance\n")
    (project / ".gitignore").write_text("ignored_alpha.txt\n")
    (project / "many").mkdir()
    for number in range(60):
        # Nucleo breaks equal-score ties by length, then injection index. Give
        # each candidate a different length so parallel traversal cannot change
        # the top50 cutoff used by exact native/installed parity.
        (project / "many" / f"capresult_{number:03d}_{'x' * number}.txt").touch()
    (base / "other").mkdir()
    (base / "other" / "alpha_other.txt").touch()


def cases():
    return {
        "lexical-alpha": {"query": "alpha", "roots": [ROOT]},
        "unicode": {"query": "naïve", "roots": [ROOT]},
        "beta": {"query": "beta", "roots": [ROOT]},
        "no-match": {"query": "no-such-needle-910284", "roots": [ROOT]},
        "empty": {"query": "", "roots": [ROOT]},
        "limit50": {"query": "capresult", "roots": [ROOT]},
        "multi-root": {"query": "alpha", "roots": [ROOT, OTHER_ROOT]},
    }


def one_shots(server, baseline=None):
    results = {}
    for name, params in cases().items():
        _, response = server.request("fuzzyFileSearch", params)
        result = response["result"]
        require(set(result) == {"files"}, "one-shot search result shape changed")
        rows = result["files"]
        for row in rows:
            require(
                row["root"] in params["roots"], "original lexical root was not retained"
            )
            require(
                not row["path"].startswith("/"), "search path is no longer relative"
            )
            require(
                type(row["score"]) is int and row["score"] >= 0, "native score missing"
            )
            require(row["match_type"] in {"file", "directory"}, "match type missing")
            require(isinstance(row["file_name"], str), "file name missing")
            indices = row["indices"]
            require(
                isinstance(indices, list) and indices == sorted(set(indices)),
                "native indices malformed",
            )
            require(
                all(type(i) is int and 0 <= i < len(row["path"]) for i in indices),
                "indices are not character offsets",
            )
            require(row["path"] != "ignored_alpha.txt", "gitignore was not honored")
        paths = {row["path"] for row in rows}
        if name == "lexical-alpha":
            require(
                paths == {"alpha.txt", "alphabet.md", "src/alpha_child.txt"},
                "alpha fixture changed",
            )
            require(all(row["indices"] for row in rows), "alpha highlighting missing")
        elif name == "unicode":
            require(
                paths == {"naïve.txt"} and rows[0]["indices"],
                "Unicode matching/highlighting failed",
            )
        elif name == "beta":
            require(paths == {"beta.txt"}, "beta fixture changed")
        elif name in {"empty", "no-match"}:
            require(rows == [], "empty/no-match returned files")
        elif name == "limit50":
            require(
                len(rows) == 50 and all("capresult_" in row["path"] for row in rows),
                "50-result App Server limit changed",
            )
        elif name == "multi-root":
            require(
                paths
                == {
                    "alpha.txt",
                    "alphabet.md",
                    "src/alpha_child.txt",
                    "alpha_other.txt",
                },
                "multi-root matching changed",
            )
        if baseline is not None:
            require(
                result == baseline[name],
                f"{name}: exact ordered native/installed result differs",
            )
        results[name] = result
    server.entry["rpc_checks"].append({"one_shot_exact_cases": list(results)})
    return results


def start_pair(server):
    requests = {
        session: server.send(
            "fuzzyFileSearch/sessionStart", {"sessionId": session, "roots": [ROOT]}
        )
        for session in ("first", "sibling")
    }
    for request in requests.values():
        server.response(request)
    # Runtime query zero is internal initial state; its callbacks are suppressed.
    # Empty presentation is observable only after a real admitted update (id>0).
    # Await both starts, then own and drain explicit empty-query cycles.
    empty_updates = {
        session: server.send(
            "fuzzyFileSearch/sessionUpdate", {"sessionId": session, "query": ""}
        )
        for session in requests
    }
    for session, request in empty_updates.items():
        require(
            server.cycle(request, session, "") == [],
            "submitted empty-query presentation changed",
        )


def update(server, session, query, expected):
    request = server.send(
        "fuzzyFileSearch/sessionUpdate", {"sessionId": session, "query": query}
    )
    rows = server.cycle(request, session, query)
    require(
        rows == expected["files"],
        "stream final snapshot differs from exact one-shot native result",
    )


def streams(server, baseline):
    start_pair(server)
    # Two leases exist concurrently under the same selected provider. Their
    # updates are also in flight together, not merely sequential creation.
    first = server.send(
        "fuzzyFileSearch/sessionUpdate", {"sessionId": "first", "query": "alpha"}
    )
    sibling = server.send(
        "fuzzyFileSearch/sessionUpdate", {"sessionId": "sibling", "query": "beta"}
    )
    require(
        server.cycle(first, "first", "alpha") == baseline["lexical-alpha"]["files"],
        "first concurrent result differs",
    )
    require(
        server.cycle(sibling, "sibling", "beta") == baseline["beta"]["files"],
        "sibling concurrent result differs",
    )
    stop_sequence, _ = server.request(
        "fuzzyFileSearch/sessionStop", {"sessionId": "first"}
    )
    server.request(
        "fuzzyFileSearch/sessionUpdate",
        {"sessionId": "first", "query": "alpha"},
        error_code=-32600,
    )
    update(server, "sibling", "alpha", baseline["lexical-alpha"])
    update(server, "sibling", "beta", baseline["beta"])
    update(server, "sibling", "alpha", baseline["lexical-alpha"])
    update(server, "sibling", "", baseline["empty"])
    update(server, "sibling", "no-such-needle-910284", baseline["no-match"])
    server.assert_quiet("first", stop_sequence)
    server.entry["rpc_checks"].append(
        {
            "concurrent_sessions": 2,
            "concurrent_updates": True,
            "sibling_after_joined_stop": True,
            "aba_query_cycles": True,
            "explicit_empty_query_clear": True,
            "no_match_completion": True,
            "notifications_after_stop_ack": 0,
            "active_session_left_for_eof_shutdown": "sibling",
        }
    )


def exhaust_real_snapshot(server, baseline):
    start_pair(server)
    update(server, "first", "alpha", baseline["lexical-alpha"])
    # Legal raw query limit exactly 64KiB, zero positive atoms. All files match,
    # so native generation must add their FileMatch/path/root storage to the raw
    # query charge. A configured 64KiB native snapshot cap is then exceeded.
    query = " " * 65536
    request = server.send(
        "fuzzyFileSearch/sessionUpdate", {"sessionId": "first", "query": query}
    )
    response_seen = failed = False
    failed_payload = None
    deadline = time.monotonic() + 90
    while not response_seen or not failed:
        _, message = server.receive(
            lambda msg: (
                msg.get("id") == request
                or (
                    msg.get("method") in SEARCH_METHODS
                    and msg.get("params", {}).get("sessionId") == "first"
                )
            ),
            timeout=max(0, deadline - time.monotonic()),
        )
        if message.get("id") == request:
            response_seen = True
            # Native update can be acknowledged before the real worker reaches
            # its output preflight, or observe that retained error first.
            if "error" in message:
                require(
                    message["error"]["code"] == -32603, "wrong failed update RPC code"
                )
                assert_resource_cause(message["error"]["message"], "update")
            else:
                require(
                    message.get("result") == {}, "unexpected accepted update response"
                )
        elif message["method"] == "fuzzyFileSearch/sessionFailed":
            require(not failed, "duplicate terminal failure")
            failed_payload = message["params"]
            require(
                failed_payload["query"] == query, "failure attributed to wrong query"
            )
            error = failed_payload["error"]
            require(
                error["kind"] == "resourceExhausted",
                "actual exhaustion typed category changed",
            )
            assert_resource_cause(error["message"], "failure notification")
            require(
                len(error["message"].encode()) <= 2048,
                "failure diagnostic exceeds contract",
            )
            failed = True
        else:
            raise AssertionError("exhausted query produced a success notification")
    # The failed lease retains its operation error through cleanup. Another
    # lease must remain usable; operation failure is not provider-wide cancel.
    update(server, "sibling", "beta", baseline["beta"])
    stop_sequence, response = server.request(
        "fuzzyFileSearch/sessionStop", {"sessionId": "first"}, error_code=-32603
    )
    assert_resource_cause(response["error"]["message"], "stop")
    server.request("fuzzyFileSearch/sessionStop", {"sessionId": "sibling"})
    server.assert_quiet("first", stop_sequence)
    require(
        not any(
            message.get("method") in SEARCH_METHODS
            and message.get("params", {}).get("sessionId") == "first"
            for _sequence, message in server.pending
        ),
        "failed session emitted duplicate terminal or later success notification",
    )
    server.entry["rpc_checks"].append(
        {
            "real_native_snapshot_exhaustion": True,
            "native_snapshot_bytes": 65536,
            "query_utf8_bytes": len(query),
            "query_sha256": hashlib.sha256(query.encode()).hexdigest(),
            "failure_kind": failed_payload["error"]["kind"],
            "sibling_after_failure": True,
            "stop_retained_operation_error": True,
            "expected_eof_exit": "positive nonzero: retained operation failure; cleanup checked separately",
        }
    )


def assert_resource_cause(message, observation):
    require(
        "snapshot exceeds its allocation limit" in message,
        f"{observation}: retained native resource failure was erased",
    )
    lowered = message.lower()
    require(
        not any(
            marker in lowered
            for marker in (
                "unconfirmed",
                "forcedshutdown",
                "forced shutdown",
                "closedlease",
                "transportlost",
            )
        ),
        f"{observation}: resource failure included a consequence error or uncertain cleanup",
    )
