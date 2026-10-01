#!/usr/bin/env python3
"""Read-only local-object checks; exit 2 means unresolved, 1 means invalid evidence.

No fetch, checkout, index, refs, object writes, source execution or runtime acceptance.
Historical schema adapters check immutable metadata and literal anchors, not semantics.
"""

import argparse
from collections import Counter
from collections import OrderedDict
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

OID = re.compile(r"[0-9a-f]{40}\Z")
MAX_INPUT = 8 * 1024 * 1024
MAX_BLOB = 4 * 1024 * 1024
# Filled from the reviewed, published historical schema shapes. A new shape is unresolved.
SCHEMA_SIGNATURES = {
    "upstream/lineage.json": "f6342a09602cd3aac06401be1bb886221c357706bc1a90e44a4eec1bbbcc68c7",
    "upstream/p01-migration-lineage.json": "120a249bdc4cd1bb01a9a14faa7c36f5f78b511076c9dc7658aab55da1fda986",
    "upstream/p02-search-lineage.json": "f64e85a3a3b1c11447613c71f42ee696ff0fcbb89baff0a3fdff93bc57eb51ff",
    "upstream/p02b-app-server-lineage.json": "6fa6170218ed548a2ca06319e43d18f03d4ba9653b0f34137b0381e9eb94775b",
    "upstream/p02b-bounded-lineage.json": "7ecdd17c2f1752668fa8d94fe54b9728ddfcc5e449fc057493d368c2558149ba",
    "upstream/p02b-contract-lineage.json": "14c3306f9040f86b809d1870f602d068c040d3c235868590f94cdde813996d59",
    "upstream/p02b-integration-lineage.json": "95e642db56460fe562d32e23757bdbddbe9687e3abdaeacd87a067c41a24f90b",
    "upstream/p02b-matrix-lineage.json": "dffbd99e0e60352d75d53904c6e3e61adf2d84c1c8b33864ead2198b5586105f",
    "upstream/p02b-native-backend-lineage.json": "c96c292703ff6fa73681028c861e745b3b708297a546bed2d3b9b9ffe78161ca",
    "upstream/p02b-preparing-app-server-lineage.json": "d7257e26695a519bc3e3f44ebe037dbc1bf7a17a2439c46e32a7815529ab8944",
    "upstream/p02b-preparing-native-backend-lineage.json": "e4a9b3daf3c0a9b991617e887fec91135b8c375d013265df3bbe5bd8c69ef309",
    "upstream/p02b-preparing-native-owner-lineage.json": "f5c3a507df2b15f1caa3f9d78a6cbfb8031b35d9044a0c4f8fd290d285b2ed0d",
    "upstream/p02b-preparing-process-lineage.json": "35774597b497240832123bd04d2faf69ef9c83ceb962f7b8ad64eabc6e8c6857",
    "upstream/p02b-preparing-tui-consumer-lineage.json": "099b59b73070d9dcba285f93ca520dbb6e8cb738239db55ccf379ef46b1afb9f",
    "upstream/p02b-public-stop-lineage.json": "43c9750d467d00028b1da955219341ca0fd5557b5d7a8257a8ecfd09648add01",
    "upstream/p02b-required-start-service-lineage.json": "bd64b06d20c77ebe041a922c621a35a932ca58fe3477396cd72313331ea8d537",
    "upstream/p02b-runtime-preparing-lineage.json": "c4bd1a2aece6e8c6d02e0550a79ad4853fbf346ccd83cb60aac954c139d0dbb8",
    "upstream/p02b-search-worker04-lineage.json": "511a1b54a219aa86c18d4d55a7bbd90d4e4c8176dee331202e1cf43b03218744",
    "upstream/p02b-selected-search-lineage.json": "98c8ac6e111435c284c14abb06e3234b5d02267f1359f9b60b8445456df1e94c",
    "upstream/p02b-startup-sdk-lineage.json": "53e7c23273884f42815cb31029904c5cb23230da59baf1fc70ff1f21eee8a483",
    "upstream/p02b-tui-consumer-lineage.json": "2f36074be938a18cf7788fbbf4a299683ffbfe07f70f2af05d6776eed761c1b2",
    "upstream/p02b-worker04-old-host-compat-lineage.json": "500408442500cb844ef9ab35b06a73977476892d62284b229194f2f8df889c4f",
    "upstream/p03-broker-grants-lineage.json": "0d4df0cbfb07cd1781fa99f3e0d378532a8dda7f45edde91ed3943bcef667fe7",
    "upstream/p03-broker-limits-lineage.json": "4f2a29a74ae08d765d908f727f38dd8c6e490eefcb3e755ea8dcb16951863fb8",
    "upstream/p03-broker-offer-lineage.json": "d5f6305e16744bc40008d20dc1d62caece442c86ee6207da6e19dcaa9b19242b",
    "upstream/p03-service-declarations-lineage.json": "16c31cac737178469273fbb11519f571abc60eac620c11328e78d885400919e3",
    "upstream/p03-wire-slice1-lineage.json": "da23691a79588c9e7c19066da588526deefd5fa347e9c1d171b43d275e390458",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def schema_signature(value):
    marker = {
        k: value[k]
        for k in ("schema", "schema_version", "kind", "artifact_kind")
        if k in value
    }
    return digest(json.dumps([sorted(value), marker], sort_keys=True).encode())


def valid_path(path):
    return (
        isinstance(path, str)
        and path
        and not path.startswith("/")
        and all(part not in ("", ".", "..") for part in path.split("/"))
        and "\x00" not in path
        and "\n" not in path
    )


def json_bytes(data):
    if len(data) > MAX_INPUT:
        raise ValueError("input_limit")

    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate_json_key")
            result[key] = value
        return result

    return json.loads(data, object_pairs_hook=unique)


class GitObjects:
    def __init__(self, repository):
        self.repository = repository
        self.cache = OrderedDict()
        self.cache_bytes = 0

    def command(self, *args):
        env = dict(
            os.environ,
            GIT_OPTIONAL_LOCKS="0",
            GIT_NO_LAZY_FETCH="1",
            GIT_NO_REPLACE_OBJECTS="1",
        )
        result = subprocess.run(
            ["git", "--no-pager", "-C", str(self.repository), *args],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            env=env,
            timeout=30,
            check=False,
        )
        if result.returncode or len(result.stdout) > MAX_INPUT:
            raise ValueError("object_unavailable_or_limit")
        return result.stdout

    def tree(self, oid):
        if not OID.fullmatch(oid):
            raise ValueError("invalid_object_id")
        if self.command("cat-file", "-t", oid).strip() != b"tree":
            raise ValueError("expected_tree")
        result = {}
        for entry in self.command("ls-tree", "-r", "-z", oid).split(b"\0"):
            if not entry:
                continue
            metadata, path = entry.split(b"\t", 1)
            mode, kind, blob = metadata.decode().split()
            path = path.decode("utf-8")
            if not valid_path(path) or path in result:
                raise ValueError("invalid_tree_path")
            result[path] = {"mode": mode, "type": kind, "blob": blob}
        return result

    def commit_tree(self, oid):
        if not isinstance(oid, str) or not OID.fullmatch(oid):
            raise ValueError("invalid_object_id")
        first = self.command("cat-file", "commit", oid).split(b"\n", 1)[0]
        if not first.startswith(b"tree "):
            raise ValueError("expected_commit")
        return first[5:].decode("ascii")

    def blob(self, oid):
        if not isinstance(oid, str) or not OID.fullmatch(oid):
            raise ValueError("invalid_object_id")
        if oid not in self.cache:
            size = int(self.command("cat-file", "-s", oid))
            if size > MAX_BLOB:
                raise ValueError("blob_limit")
            value = self.command("cat-file", "blob", oid)
            if (
                hashlib.sha1(
                    b"blob " + str(len(value)).encode() + b"\0" + value
                ).hexdigest()
                != oid
            ):
                raise ValueError("object_hash_mismatch")
            while self.cache and self.cache_bytes + len(value) > 16 * 1024 * 1024:
                _, retired = self.cache.popitem(last=False)
                self.cache_bytes -= len(retired)
            self.cache[oid] = value
            self.cache_bytes += len(value)
        self.cache.move_to_end(oid)
        return self.cache[oid]


def validate(index, objects, publication=None):
    try:
        return validate_document(index, objects, publication)
    except (
        AttributeError,
        TypeError,
        KeyError,
        IndexError,
        ValueError,
        RecursionError,
    ):
        return outcome(Counter({"invalid:malformed_document": 1}), [])


def validate_document(index, objects, publication=None):
    counts = Counter()
    details = []

    def report(level, code, context=""):
        counts[level + ":" + code] += 1
        if len(details) < 16 and counts[level + ":" + code] <= 2:
            details.append(
                {"level": level, "code": code, "context": str(context)[:160]}
            )

    if (
        not isinstance(index, dict)
        or type(index.get("schema_version")) is not int
        or index["schema_version"] != 1
    ):
        report("unresolved", "unsupported_index_schema")
        return outcome(counts, details)
    try:
        original = objects.tree(index["source"]["import_tree"])
        current = objects.tree(index["source"]["checkpoint_tree"])
    except (KeyError, ValueError, OSError, subprocess.TimeoutExpired):
        report("unresolved", "source_tree_unavailable")
        return outcome(counts, details)
    for key in ("upstream_revision", "import_commit"):
        try:
            if (
                objects.commit_tree(index["source"][key])
                != index["source"]["import_tree"]
            ):
                report("invalid", "source_revision_tree_mismatch", key)
        except (
            AttributeError,
            KeyError,
            ValueError,
            OSError,
            subprocess.TimeoutExpired,
        ):
            report("unresolved", "source_revision_tree_unavailable", key)
    try:
        checkpoint_tree = objects.commit_tree(index["source"]["checkpoint_commit"])
    except (AttributeError, KeyError, ValueError, OSError, subprocess.TimeoutExpired):
        checkpoint_tree = None
    if checkpoint_tree is not None:
        if checkpoint_tree != index["source"]["checkpoint_tree"]:
            report("invalid", "checkpoint_revision_tree_mismatch")
    else:
        binding = index["source"]["publication"]
        if publication is None:
            report("unresolved", "publication_receipt_unavailable")
        else:
            try:
                receipt = json_bytes(publication)
                if digest(publication) != binding["sha256"] or (
                    receipt.get("sha"),
                    receipt.get("tree"),
                    receipt.get("remote_verified"),
                ) != (
                    index["source"]["checkpoint_commit"],
                    index["source"]["checkpoint_tree"],
                    True,
                ):
                    report("invalid", "publication_binding_mismatch")
            except (ValueError, KeyError):
                report("invalid", "publication_receipt_invalid")
    expected = {
        p for p in original.keys() | current.keys() if original.get(p) != current.get(p)
    }
    rows = {}
    for row in index.get("paths", []):
        path = row.get("path")
        if not valid_path(path) or path in rows:
            report("invalid", "duplicate_or_invalid_path")
            continue
        rows[path] = row
        for key, tree in (("before", original), ("after", current)):
            if row.get(key) != tree.get(path):
                report("invalid", "path_object_mismatch", path)
        if not row.get("inventory_owners") or any(
            not re.fullmatch(r"C(?:0[0-9]|1[0-9]|2[0-7])", x)
            for x in row.get("inventory_owners", [])
        ):
            report("unresolved", "inventory_owner_missing", path)
        if row.get("semantic_status") != "reviewed_edge":
            report("unresolved", "path_semantics_unresolved", path)
    if set(rows) != expected:
        report("invalid", "changed_path_coverage_mismatch")
    wanted_maps = {
        p for p in current if p.startswith("upstream/") and p.endswith("lineage.json")
    }
    found_maps = set()
    maps = {}
    for reference in index.get("historical_maps", []):
        path = reference.get("path")
        if path in found_maps or path not in wanted_maps:
            report("invalid", "historical_map_duplicate_or_unexpected", path)
            continue
        found_maps.add(path)
        try:
            if reference["blob"] != current[path]["blob"]:
                report("invalid", "historical_map_object_mismatch", path)
                continue
            content = objects.blob(reference["blob"])
            if digest(content) != reference["sha256"]:
                report("invalid", "historical_map_hash_mismatch", path)
                continue
            value = json_bytes(content)
            if not isinstance(value, dict) or schema_signature(
                value
            ) != SCHEMA_SIGNATURES.get(path):
                report("unresolved", "historical_schema_unsupported", path)
                continue
            maps[path] = value
            check_historical(value, objects, report, path)
        except (KeyError, TypeError, ValueError, OSError, subprocess.TimeoutExpired):
            report("unresolved", "historical_map_unavailable", path)
    if found_maps != wanted_maps:
        report("invalid", "historical_map_coverage_mismatch")
    edge_ids = set()
    bound_destinations = {}
    for edge in index.get("boundary_edges", []):
        identity = edge.get("id")
        if not identity or identity in edge_ids:
            report("invalid", "duplicate_or_missing_edge_id")
            continue
        edge_ids.add(identity)
        relation = edge.get("relation")
        if relation not in (
            "native_derivation",
            "native_adaptation",
            "original_custom",
            "dependency_use",
        ):
            report("unresolved", "edge_relation_unsupported", identity)
            continue
        sources = edge.get("origins", [])
        if not edge.get("destinations"):
            report("invalid", "edge_destination_missing", identity)
        if relation in ("native_derivation", "native_adaptation") and not sources:
            report("invalid", "native_origin_missing", identity)
        if (
            relation in ("native_derivation", "native_adaptation")
            and sources
            and not any(
                source.get("repository") == index["source"]["upstream_repository"]
                for source in sources
            )
        ):
            report("unresolved", "native_origin_unverified", identity)
        if relation == "dependency_use" and not sources:
            report("unresolved", "dependency_origin_missing", identity)
        if relation == "original_custom" and sources:
            report("invalid", "custom_code_has_native_origin", identity)
        for ref in sources + edge.get("destinations", []):
            check_anchor(ref, objects, report, identity)
        for source in sources:
            if source.get("repository") == index["source"].get("checkpoint_repository"):
                if source.get("revision") != index["source"][
                    "checkpoint_commit"
                ] or current.get(source.get("path"), {}).get("blob") != source.get(
                    "blob"
                ):
                    report("invalid", "custom_dependency_tree_mismatch", identity)
            elif source.get("repository") != index["source"]["upstream_repository"]:
                report("unresolved", "foreign_origin_binding_unsupported", identity)
            if source.get("repository") == index["source"]["upstream_repository"]:
                if source.get("revision") != index["source"][
                    "upstream_revision"
                ] or original.get(source.get("path"), {}).get("blob") != source.get(
                    "blob"
                ):
                    report("invalid", "native_origin_tree_mismatch", identity)
        bound_destinations[identity] = set()
        for destination in edge.get("destinations", []):
            if (
                destination.get("repository")
                != index["source"].get("checkpoint_repository")
                or destination.get("revision") != index["source"]["checkpoint_commit"]
            ):
                report(
                    "unresolved", "destination_revision_binding_unsupported", identity
                )
            elif current.get(destination.get("path"), {}).get(
                "blob"
            ) != destination.get("blob"):
                report("invalid", "destination_tree_mismatch", identity)
            else:
                bound_destinations[identity].add(destination["path"])
        for evidence in edge.get("mapping_refs", []):
            document = maps.get(evidence.get("path"))
            try:
                for part in evidence["pointer"].strip("/").split("/"):
                    if part:
                        part = part.replace("~1", "/").replace("~0", "~")
                        document = (
                            document[int(part)]
                            if isinstance(document, list)
                            else document[part]
                        )
                if document is None:
                    raise ValueError("missing")
            except (KeyError, TypeError, ValueError, IndexError):
                report("unresolved", "mapping_pointer_unavailable", identity)
        if not edge.get("mapping_refs"):
            report("unresolved", "mapping_reference_missing", identity)
        if edge.get("current_semantic_closure") is not True:
            report("unresolved", "boundary_current_closure_pending", identity)
    for path, row in rows.items():
        references = set(row.get("edge_ids", []))
        if references - edge_ids:
            report("invalid", "path_edge_unknown", path)
        if row.get("semantic_status") == "reviewed_edge" and not any(
            path in bound_destinations.get(identity, set()) for identity in references
        ):
            report("invalid", "reviewed_path_without_exact_edge", path)
    report("unresolved", "release_evidence_and_updater_gates_not_validated")
    return outcome(counts, details, len(rows), len(maps), len(edge_ids))


def check_anchor(ref, objects, report, context):
    if (
        not valid_path(ref.get("path"))
        or not isinstance(ref.get("revision"), str)
        or not OID.fullmatch(ref["revision"])
    ):
        report("unresolved", "anchor_identity_incomplete", context)
        return
    try:
        content = objects.blob(ref["blob"])
        anchors = ref.get("symbols", [])
        if not anchors:
            report("unresolved", "anchor_missing", context)
        for anchor in anchors:
            if (
                not isinstance(anchor, str)
                or not anchor
                or anchor.encode() not in content
            ):
                report("invalid", "anchor_not_found", context)
    except (KeyError, ValueError, OSError, subprocess.TimeoutExpired):
        report("unresolved", "anchor_object_unavailable", context)


def check_historical(value, objects, report, context):
    """Normalize literal blob/anchor forms; other provenance shapes stay explicit gaps."""
    if isinstance(value, list):
        for item in value:
            check_historical(item, objects, report, context)
    elif isinstance(value, dict):
        blob = value.get("blob")
        if isinstance(blob, str) and OID.fullmatch(blob):
            try:
                content = objects.blob(blob)
                symbols = value.get("symbols", [])
                for symbol in symbols if isinstance(symbols, list) else []:
                    anchor = symbol.get("anchor") if isinstance(symbol, dict) else None
                    if isinstance(anchor, str) and anchor.encode() not in content:
                        report("unresolved", "historical_anchor_not_found", context)
                if (
                    isinstance(value.get("symbol"), str)
                    and value["symbol"].encode() not in content
                ):
                    report(
                        "unresolved", "historical_nonliteral_or_missing_anchor", context
                    )
            except (ValueError, OSError, subprocess.TimeoutExpired):
                report("unresolved", "historical_object_unavailable", context)
        for item in value.values():
            if isinstance(item, (dict, list)):
                check_historical(item, objects, report, context)


def outcome(counts, details, paths=0, maps=0, edges=0):
    invalid = sum(v for k, v in counts.items() if k.startswith("invalid:"))
    unresolved = sum(v for k, v in counts.items() if k.startswith("unresolved:"))
    return {
        "status": "invalid" if invalid else "unresolved" if unresolved else "valid",
        "invalid": invalid,
        "unresolved": unresolved,
        "checked_paths": paths,
        "supported_historical_maps": maps,
        "checked_edges": edges,
        "counts": dict(sorted(counts.items())),
        "first_findings": details,
        "findings_truncated": sum(counts.values()) > len(details),
        "runtime_or_updater_acceptance": False,
    }


def read_limited(path):
    with path.open("rb") as stream:
        value = stream.read(MAX_INPUT + 1)
    if len(value) > MAX_INPUT:
        raise ValueError("input_limit")
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("index", type=Path)
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--publication-receipt", type=Path)
    args = parser.parse_args()
    try:
        raw = read_limited(args.index)
        try:
            publication = (
                read_limited(args.publication_receipt)
                if args.publication_receipt
                else None
            )
        except OSError:
            publication = None
        report = validate(json_bytes(raw), GitObjects(args.repository), publication)
    except (
        ValueError,
        TypeError,
        KeyError,
        OSError,
        RecursionError,
        subprocess.TimeoutExpired,
    ):
        report = outcome(Counter({"invalid:input_or_object_error": 1}), [])
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if report["invalid"] else 2 if report["unresolved"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
