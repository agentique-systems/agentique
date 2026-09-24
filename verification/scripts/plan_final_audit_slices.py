"""Plan bounded corpus gates from pinned bytes and authenticated prior ownership.

This reads a historical rejected frontier; it neither restores semantic authority
nor evaluates queries. Fresh slice closure and strict effective audits remain required.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import uuid
import zipfile


ROOT = Path(__file__).resolve().parents[2]
DOMAIN = uuid.UUID("2abdcfe0-9071-4db6-aa6b-ff4bbd55f188")
BASE = {"Actions", "Connections", "Constraints", "Flows", "Items", "Parts", "Ports", "States"}
MEDIUM = BASE | {"Attributes", "Calculations", "Interfaces", "Requirements", "Views"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def identity(parts):
    return str(uuid.uuid5(DOMAIN, json.dumps(parts, separators=(",", ":"), ensure_ascii=False)))


def pinned_sources():
    manifest = json.loads((ROOT / "standards/normative/sysml-2.0/library-set.json").read_bytes())
    artifact = next(a for a in manifest["artifacts"] if a["specification"] == "SysML")
    archive = ROOT / artifact["path"]
    require(digest(archive) == artifact["sha256"], "Systems KPAR identity changed")
    library = identity(["agentique-pinned-library/1", artifact["specification"], artifact["version"],
                        artifact["source"], artifact["sha256"], artifact["kpar"]["metamodel"]])
    sources, anchors = {}, {}
    with zipfile.ZipFile(archive) as zipped:
        for entry in artifact["entries"]:
            if not entry["path"].endswith(".sysml"):
                continue
            raw = zipped.read(entry["path"])
            require(hashlib.sha256(raw).hexdigest() == entry["sha256"], "source entry identity changed")
            text = raw.decode("utf-8")
            name = Path(entry["path"]).stem
            require(text.startswith("standard library package " + name + " {"), "unexpected root syntax")
            document = identity(["agentique-library-document/1", library, entry["path"], entry["sha256"]])
            # The pinned documents each have one LibraryPackage occupying this
            # exact range. Recompute its canonical source locator, then require
            # the retained graph to contain that identity and matching name.
            end = raw.rfind(b"}") + 1
            anchor = identity(["agentique-library-element/1", document, entry["sha256"],
                               0, end, "LibraryPackage", 0])
            anchors[anchor] = name
            sources[name] = dict(path=entry["path"], sha256=entry["sha256"], document=document,
                                 root=anchor, root_range=[0, end], text=text)
    return manifest["id"], artifact["sha256"], sources, anchors


def descriptor_ids():
    ids = {}
    for language in ("kerml", "sysml"):
        text = (ROOT / f"crates/{language}/src/generated/typed_views.rs").read_text(encoding="utf-8")
        for name, value in re.findall(
                r"pub const (\w+): agq_kernel::(?:MetaclassId|PropertyId) = "
                r"agq_kernel::(?:MetaclassId|PropertyId)::from_u128\(0x([0-9a-f]+)\)", text):
            ids[name] = str(uuid.UUID(hex=value))
    return ids


def ownership_records(archive, member, graph_digest):
    ids = descriptor_ids()
    properties = {ids[name]: name for name in (
        "ELEMENT_OWNED_RELATIONSHIP", "RELATIONSHIP_OWNED_RELATED_ELEMENT", "ELEMENT_DECLARED_NAME")}
    records = {}
    decoded = hashlib.sha256()
    with zipfile.ZipFile(archive).open(member) as stream:
        for line in stream:
            decoded.update(line)
            if not line.startswith(b'{"Record":'):
                continue
            record = json.loads(line)["Record"]
            slots = {properties[prop]: slot["value"] for prop, slot in record["slots"]
                     if prop in properties}
            records[record["id"]] = dict(metaclass=record["class"], slots=slots)
    require(decoded.hexdigest() == graph_digest, "decoded graph identity")
    def values(record, prop):
        slot = record["slots"].get(prop, {})
        values = [slot["Scalar"]] if "Scalar" in slot else next(iter(slot.values()), [])
        return [next(iter(value.values())) for value in values]
    parents = {}
    for subject, record in records.items():
        for prop in ("ELEMENT_OWNED_RELATIONSHIP", "RELATIONSHIP_OWNED_RELATED_ELEMENT"):
            for child in values(record, prop):
                require(child not in parents or parents[child]["owner"] == subject,
                        f"multiple canonical owners for {child}")
                parents[child] = dict(owner=subject, property=prop)
    return records, parents


def retained_ownership(frontier, authentication, anchors):
    require(digest(frontier) == authentication["archive_sha256"], "prior frontier archive identity")
    records, parents = ownership_records(
        frontier, "graph.jsonl", authentication["graph_evidence"]["graph_sha256"])
    ids = descriptor_ids()
    for anchor, name in anchors.items():
        require(anchor in records, f"missing source-identified package {name}")
        require(records[anchor]["metaclass"] == ids["LIBRARY_PACKAGE"], f"invalid package kind {name}")
        require(records[anchor]["slots"].get("ELEMENT_DECLARED_NAME") == {"Scalar": {"String": name}},
                f"invalid package name {name}")
    return records, parents


def accepted_ownership(cache):
    receipt = json.loads((ROOT / "standards/kerml-accepted-publication.json").read_bytes())
    graph_digest = bytes(receipt["complete_overlay"]["graph_sha256"]).hex()
    records, parents = ownership_records(cache, "kernel.jsonl", graph_digest)
    manifest = json.loads((ROOT / "standards/normative/sysml-2.0/library-set.json").read_bytes())
    anchors = {}
    for artifact in manifest["artifacts"]:
        if artifact["specification"] != "KerML":
            continue
        archive = ROOT / artifact["path"]
        require(digest(archive) == artifact["sha256"], "KerML KPAR identity changed")
        library = identity(["agentique-pinned-library/1", artifact["specification"], artifact["version"],
                            artifact["source"], artifact["sha256"], artifact["kpar"]["metamodel"]])
        with zipfile.ZipFile(archive) as zipped:
            for entry in artifact["entries"]:
                if not entry["path"].endswith(".kerml"):
                    continue
                raw = zipped.read(entry["path"])
                require(hashlib.sha256(raw).hexdigest() == entry["sha256"], "KerML source identity")
                name = Path(entry["path"]).stem
                if not raw.startswith(("standard library package " + name + " {").encode()):
                    continue
                document = identity(["agentique-library-document/1", library, entry["path"], entry["sha256"]])
                anchor = identity(["agentique-library-element/1", document, entry["sha256"],
                                   0, raw.rfind(b"}") + 1, "LibraryPackage", 0])
                if anchor in records:
                    anchors[anchor] = entry["path"]
    return graph_digest, records, parents, anchors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--frontier", required=True, type=Path)
    parser.add_argument("--kerml-cache", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    prior = json.loads((ROOT / "verification/fixtures/final-audit-semantic-closure/prior-classification.json").read_bytes())
    authentication = json.loads((ROOT / "verification/summaries/systems-finalization/authentication.json").read_bytes())
    content_set, artifact, sources, anchors = pinned_sources()
    records, parents = retained_ownership(args.frontier, authentication, anchors)
    dependency_digest, dependency_records, dependency_parents, dependency_anchors = accepted_ownership(args.kerml_cache)
    require(dependency_digest == prior["accepted_kerml_graph_sha256"], "classifier dependency graph identity")
    local_records = set(records)
    records.update(dependency_records)
    parents.update(dependency_parents)
    package_documents = {anchor: sources[name]["path"] for anchor, name in anchors.items()} | dependency_anchors
    all_attribution = []
    for subject in sorted(prior["subjects"]):
        current, seen, chain = subject, set(), []
        while current not in package_documents:
            require(current in records and current not in seen and current in parents,
                    f"no source package ownership path for {subject}")
            seen.add(current)
            edge = parents[current]
            chain.append(dict(child=current, **edge))
            current = edge["owner"]
        categories = sorted({row["category"] for row in prior["diagnostics"] if row["subject"] == subject})
        all_attribution.append(dict(subject=subject, **prior["subjects"][subject], categories=categories,
                                    document=package_documents[current], ownership=chain,
                                    accepted_dependency=subject not in local_records,
                                    source_package=current))
    attribution = [row for row in all_attribution if "ResultCycle" in row["categories"]]
    require(len(attribution) == 21, "prior result subject population changed")
    dependencies = {}
    enumeration_definitions = {}
    for name, source in sources.items():
        text = re.sub(r"/\*.*?\*/|//[^\n]*", "", source["text"], flags=re.S)
        dependencies[name] = sorted(set(re.findall(r"\b(\w+)\s*::", text)) & sources.keys() - {name})
        enums = re.findall(r"\benum\s+def\s+(\w+)", text)
        if enums:
            enumeration_definitions[name] = enums
    def closure(seeds):
        seen, todo = set(), list(seeds)
        while todo:
            name = todo.pop()
            if name not in seen:
                seen.add(name)
                todo.extend(dependencies[name])
        return seen
    enum_seeds = set(enumeration_definitions) | {"SysML", "States", "Requirements", "VerificationCases"}
    selections = {
        "H1-enumerations": closure(enum_seeds) | BASE | {"Metadata", "Calculations"},
        "H2-flow-interface": closure({"Flows", "Connections", "Interfaces"}) | BASE | {"Calculations"},
        "H3-result-cycles": BASE,
        "H4-medium": MEDIUM,
    }
    combined = selections["H1-enumerations"] | selections["H2-flow-interface"] | selections["H3-result-cycles"]
    require(combined == selections["H1-enumerations"], "combined slice exceeds enumeration slice")
    require(all(row["accepted_dependency"] or Path(row["document"]).stem in combined
                for row in all_attribution), "prior subject outside combined slice")
    selections["H1-H3-combined"] = combined
    output = dict(
        format="agq-final-audit-slice-plan/1", publication_authority=False,
        semantic_gates_run=False, source_content_set=content_set, systems_kpar_sha256=artifact,
        retained_frontier_archive_sha256=authentication["archive_sha256"],
        retained_frontier_graph_sha256=authentication["graph_evidence"]["graph_sha256"],
        accepted_kerml_graph_sha256=dependency_digest,
        all_subject_counts=dict(sorted(Counter(row["document"] for row in all_attribution).items())),
        all_prior_subjects=all_attribution,
        result_cycle_subject_counts=dict(sorted(Counter(row["document"] for row in attribution).items())),
        result_cycle_subjects=[row["subject"] for row in attribution], enumeration_definitions=enumeration_definitions,
        textual_qualified_document_dependencies=dependencies,
        slices={name: dict(documents=sorted(selected), document_count=len(selected),
                           documents_argument="--documents=" + ",".join(sorted(selected)))
                for name, selected in selections.items()},
        documents={name: {key: value for key, value in source.items() if key != "text"}
                   for name, source in sorted(sources.items())},
        dependency_scope=("Lexical qualified references only, with reviewed semantic supplements: "
                          "the previously closed eight-document Actions foundation; Metadata for "
                          "SysML's metadata definitions; Calculations for Interfaces::excludingOnce. "
                          "This plan does not prove new reference or producer closure."),
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"result_cycle_subject_counts": output["result_cycle_subject_counts"],
                      "all_subject_counts": output["all_subject_counts"],
                      "slices": output["slices"]}, indent=2))


if __name__ == "__main__":
    main()
