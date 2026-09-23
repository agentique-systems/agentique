"""Read-only diagnosis of literal typing in an authenticated unaccepted frontier.

Exit zero confirms the diagnosed missing-typing shape, never publication acceptance.
No archive, source model, producer state or certificate is modified.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import uuid
import zipfile


DATA_VALUES = "170bd54f-53f0-512b-b16b-2c96e7fe9021"
SOURCE_ROLES = {
    "SPECIALIZATION_SPECIFIC", "FEATURE_TYPING_TYPED_FEATURE",
    "SUBSETTING_SUBSETTING_FEATURE", "REDEFINITION_REDEFINING_FEATURE",
    "SUBCLASSIFICATION_SUBCLASSIFIER",
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def references(value):
    if isinstance(value, dict):
        if "Reference" in value:
            yield value["Reference"]
        for child in value.values():
            yield from references(child)
    elif isinstance(value, list):
        for child in value:
            yield from references(child)


def diagnose(repository, archive_path, archive_sha256, graph_sha256):
    names = {}
    descriptor_tables = {}
    for crate in ("sysml", "kerml"):
        path = Path(f"crates/{crate}/src/generated/typed_views.rs")
        raw = (repository / path).read_bytes()
        descriptor_tables[path.as_posix()] = hashlib.sha256(raw).hexdigest()
        for name, identity in re.findall(
            r"pub const (\w+):.*?from_u128\(0x([0-9a-f]+)\)", raw.decode("utf-8")
        ):
            names[str(uuid.UUID(int=int(identity, 16)))] = name
    records, occurrences, tags = {}, {}, Counter()
    graph_hash = hashlib.sha256()
    with archive_path.open("rb") as source:
        require(hashlib.file_digest(source, "sha256").hexdigest() == archive_sha256,
                "independent archive SHA-256 mismatch")
        source.seek(0)
        with zipfile.ZipFile(source) as archive:
            require(sorted(archive.namelist()) == ["graph.jsonl", "state.json"],
                    "unexpected frontier archive inventory")
            with archive.open("graph.jsonl") as graph:
                for line in graph:
                    graph_hash.update(line)
                    row = json.loads(line)
                    if isinstance(row, str):
                        tags[row] += 1
                        continue
                    require(len(row) == 1, "unexpected canonical row")
                    kind, value = next(iter(row.items()))
                    tags[kind] += 1
                    if kind == "Record":
                        # Overlay replacements follow declared rows. Retain each
                        # final canonical record once, independent of ownership.
                        records[value["id"]] = {
                            "id": value["id"],
                            "class": names.get(value["class"], value["class"]),
                            "slots": {names.get(key, key): slot["value"]
                                      for key, slot in value["slots"]},
                        }
                    elif kind == "Occurrence":
                        occurrences[value["id"]] = value
    require(graph_hash.hexdigest() == graph_sha256, "decoded graph SHA-256 mismatch")
    require(tags["End"] == 1 and tags["DependentHeader"] == 1,
            "expected one complete dependent frontier")
    literals = {key: row for key, row in records.items()
                if row["class"] == "ENUMERATION_USAGE"}
    owners = {}
    for key, row in records.items():
        for related in references(row["slots"].get("ELEMENT_OWNED_RELATIONSHIP", {})):
            require(related not in owners or owners[related] == key, "multiple relationship owners")
            owners[related] = key
    incoming, base_incoming, dependency_source_edges = [], [], []
    for key, row in records.items():
        for property_name, value in row["slots"].items():
            targets = set(references(value))
            if property_name in SOURCE_ROLES:
                dependency_source_edges.extend({"relationship": key, "class": row["class"],
                                                "property": property_name, "source": target}
                                               for target in targets - records.keys())
            for target in sorted(targets & literals.keys()):
                incoming.append({"relationship": key, "class": row["class"],
                                 "property": property_name, "target": target})
            if DATA_VALUES in targets:
                base_incoming.append({"relationship": key, "class": row["class"],
                                      "property": property_name})
    occurrence_hits = [row for row in occurrences.values()
                       if any(target in literals or target == DATA_VALUES
                              for _, target in row["ends"])]
    external_occurrence_ends = [
        {"occurrence": row["id"], "end": end, "target": target}
        for row in occurrences.values() for end, target in row["ends"]
        if target not in records
    ]
    source_edges = [row for row in incoming if row["property"] in SOURCE_ROLES]
    base_source_edges = [row for row in base_incoming if row["property"] in SOURCE_ROLES]
    require(len(literals) == 21 and len(source_edges) == 21,
            "literal/source-edge population differs from diagnosed checkpoint")
    require(not occurrence_hits and not base_source_edges,
            "additional canonical typing paths require review")
    require(not dependency_source_edges,
            "local relationship adds an outgoing edge to an accepted dependency subject")
    require(not external_occurrence_ends,
            "local association endpoint extends the accepted dependency; inspect its role")
    require(Counter(row["property"] for row in base_incoming) == Counter({
        "MEMBERSHIP_MEMBER_ELEMENT": 1, "SUBSETTING_SUBSETTED_FEATURE": 57,
    }), "additional local DataValues references require descriptor-role review")
    require(Counter(row["property"] for row in incoming) == Counter({
        "RELATIONSHIP_OWNED_RELATED_ELEMENT": 21,
        "SUBSETTING_SUBSETTING_FEATURE": 21,
        "MEMBERSHIP_MEMBER_ELEMENT": 2,
        "REFERENCE_SUBSETTING_REFERENCED_FEATURE": 2,
    }), "additional literal references require descriptor-role review")
    details = []
    for literal_id, literal in sorted(literals.items()):
        edges = [edge for edge in source_edges if edge["target"] == literal_id]
        require(len(edges) == 1, "literal has additional/missing source relationships")
        edge = records[edges[0]["relationship"]]
        require(edge["class"] == "SUBSETTING" and set(references(
            edge["slots"].get("SUBSETTING_SUBSETTED_FEATURE", {}))) == {DATA_VALUES},
            "literal has a different specialization target")
        owned = set(references(literal["slots"].get("ELEMENT_OWNED_RELATIONSHIP", {})))
        require(owned == {edge["id"]}, "owned typing/conjugation/chaining requires review")
        membership = [row["relationship"] for row in incoming
                      if row["target"] == literal_id and row["class"] == "VARIANT_MEMBERSHIP"]
        require(len(membership) == 1, "literal variant membership is not unique")
        owner = records[owners[membership[0]]]
        require(owner["class"] == "ENUMERATION_DEFINITION", "unexpected variant owner")
        details.append({
            "literal": literal_id,
            "name": literal["slots"].get("ELEMENT_DECLARED_NAME"),
            "membership": membership[0], "enumeration_definition": owner["id"],
            "enumeration_is_variation": owner["slots"].get("ENUMERATION_DEFINITION_IS_VARIATION"),
            "only_source_specialization": edge["id"], "general": DATA_VALUES,
            "enumeration_owner_typing_present": False,
        })
    return {
        "format": "agq-unaccepted-enumeration-frontier-diagnosis/1",
        "archive_sha256": archive_sha256, "graph_sha256": graph_sha256,
        "generated_descriptor_tables_sha256": descriptor_tables,
        "publication_authority": False, "producer_evaluations": 0,
        "serialized_record_rows": tags["Record"], "final_local_records": len(records),
        "canonical_occurrences_checked": len(occurrences),
        "literal_or_data_values_occurrence_endpoints": 0,
        "literal_incoming_reference_properties": dict(sorted(Counter(
            row["property"] for row in incoming).items())),
        "data_values_local_incoming_properties": dict(sorted(Counter(
            row["property"] for row in base_incoming).items())),
        "data_values_local_source_relationships": 0,
        "local_source_relationships_extending_accepted_dependency": dependency_source_edges,
        "association_endpoints_outside_local_records": external_occurrence_ends,
        "literal_count": len(literals), "missing_enumeration_owner_typing": len(details),
        "all_literal_incoming_references": incoming, "literals": details,
        "dependency_boundary": "Accepted immutable KerML contains no local EnumerationDefinition; local incoming source/occurrence additions to DataValues were also checked.",
        "result": "diagnosed missing canonical enumeration typing; not publication acceptance",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--archive-sha256", required=True)
    parser.add_argument("--graph-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(args.output.resolve() != args.archive.resolve(), "output must not overwrite archive")
    report = diagnose(args.repository, args.archive, args.archive_sha256, args.graph_sha256)
    encoded = (json.dumps(report, indent=2, sort_keys=True) + "\n").encode("utf-8")
    args.output.write_bytes(encoded)
    print(json.dumps({"result": report["result"], "missing_typing": report["missing_enumeration_owner_typing"],
                      "local_records": report["final_local_records"], "occurrences": report["canonical_occurrences_checked"],
                      "output_sha256": hashlib.sha256(encoded).hexdigest()}, sort_keys=True))


if __name__ == "__main__":
    main()
