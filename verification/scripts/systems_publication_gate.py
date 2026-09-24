"""Verify final Systems acceptance evidence and generated receipt/cache artifacts.

This does not issue authority, activate the catalogue, or replace Rust trusted
restore. It requires the strict full report, independently pinned original source
identities, and authenticated generated artifacts. No producer replay occurs.
"""
import argparse
import hashlib
import json
from pathlib import Path
import uuid
import zipfile

from frontier_artifact_evidence import checkpoint_evidence, graph_evidence, pin, require

PROFILE = "agentique-sysml-2.0-operational/2"
PROFILES = {
    # Published-semantics implementation completion changed the query/producer
    # identity independently of the operational interpretation. Retain exact
    # historical v2 evidence as well as current v2; v3 begins with query/6.
    PROFILE: (("agq-sysml-query/5", "agq-sysml-query/6"), "standards/sysml-2.0-operational-semantic-v2.json"),
    "agentique-sysml-2.0-operational/3": (("agq-sysml-query/6",), "standards/sysml-2.0-operational-semantic-v3.json"),
}
FAMILIES = set("Syntax CanonicalLowering NamespacesImports DefinitionUsage AttributeItemPart "
               "OccurrenceActionState CalculationConstraintRequirementCase PortConnectionInterfaceFlow "
               "ViewMetadata TypingSpecializationSubsettingRedefinition MayTimeVary StandardBindings "
               "IdentityProvenance".split())
DOC_FIELDS = ("path", "document", "sha256", "profile", "parsed", "byte_exact", "recovery_count",
              "production_count", "construction_gap")
CERT_FIELDS = {"model_digest": "model_digest", "registry_digest": "producer_registry_digest",
               "context_contract_digest": "context_contract_digest", "digest": "digest"}
DOMAIN = uuid.UUID("2abdcfe0-9071-4db6-aa6b-ff4bbd55f188")
ROLES = set("Item Items Subitems Subparts Part Parts Action Actions Subactions OwnedActions Port Ports "
            "Subports OwnedPorts Connection Connections BinaryConnection BinaryConnections Interface Interfaces "
            "MessageAction Messages Flows SuccessionFlows StateAction StateActions Calculation Calculations "
            "ConstraintCheck ConstraintChecks CheckedConstraints RequirementCheck RequirementChecks ConcernCheck "
            "ConcernChecks Case Cases AnalysisCase AnalysisCases VerificationCase VerificationCases UseCase UseCases "
            "Allocation Allocations MetadataItem MetadataItems View Views ViewpointCheck ViewpointChecks Rendering "
            "Renderings BinaryInterface BinaryInterfaces AssignmentActions Assignments WhileLoopActions WhileLoops "
            "TransitionActions TransitionAccepter DecisionTransitions AcceptActions AcceptSubactions PerformedActions "
            "Substates ExclusiveStates OwnedStates StateTransitions".split())


def encoded(value):
    # serde_json::Value uses ordered maps and emits UTF-8 without ASCII escaping.
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def natural(value, name, positive=False):
    require(type(value) is int and value >= int(positive), f"invalid {name}")
    return value


def identifier(value):
    require(isinstance(value, str) and str(uuid.UUID(value)) == value, "invalid canonical identity")
    return value


def stable_id(value):
    return str(uuid.uuid5(DOMAIN, encoded(value).decode()))


def checked_families(value):
    require(set(value) == FAMILIES, "publication capability coverage")
    for family, count in value.items():
        natural(count, family, positive=True)
    require(value["Syntax"] == value["CanonicalLowering"] == 21, "document capability counts")
    # The combined audit also counts the separately accepted KerML anchors.
    require(value["StandardBindings"] >= 69, "binding capability count")


def accept_report(report):
    require(report["format"] == "agq-sysml-systems-publication-audit/1", "report format")
    require(report["scope"] is None, "full corpus required")
    require(report["publication_attempted"] is True and report["publication_accepted"] is True,
            "strict publication was not accepted")
    require("publication_error" not in report, "publication error")
    require(report["reference_audit_scope"] == "accepted_publication", "strict reference audit")
    require(report["sysml_profile"] in PROFILES and report["construction_complete"] is True,
            "profile/construction")
    for key in ("systems_documents_parsed", "systems_documents_constructed", "systems_documents_byte_exact"):
        require(type(report[key]) is int and report[key] == 21, key)
    require(type(report["kernel_obligations"]) is int and report["kernel_obligations"] == 0
            and report["kernel_obligation_details"] == [], "kernel obligations")
    references = report["mandatory_references"]
    expected = dict(complete=1327, unresolved=0, incomplete=0, ambiguous=0, invalid=0, endpoint_mismatch=0)
    require(type(references["total"]) is int and references["total"] == 1327
            and references["failures"] == [] and references["counts"] == expected
            and all(type(v) is int for v in references["counts"].values()), "mandatory references")
    require(report["authority_conflicts"] == [], "authority conflicts")
    require(report["kerml_producers_replayed"] is False, "accepted KerML replay")
    require(report["publication_producers"]["converged"] is True
            and report["publication_producers"]["completeness"] == "Complete", "producer closure")
    certificate = report["producer_closure"]
    require(certificate["fully_closed"] is True and natural(certificate["incomplete_pairs"], "incomplete pairs") == 0,
            "certificate not fully closed")
    for a, b in (("applicable_pairs", "closed_pairs"), ("required_requirements", "closed_requirements")):
        require(natural(certificate[a], a, True) == natural(certificate[b], b, True), a)
    for field in (*CERT_FIELDS.values(), "semantic_closure_digest", "revalidation_digest"):
        pin(certificate[field])
    for field in ("publication_digest", "semantic_digest", "accepted_kerml_digest"):
        pin(report[field])
    require(report["semantic_digest"] == certificate["model_digest"], "accepted graph identity")
    audit = report["publication_gate"]
    require(audit["findings"] == [] and audit["mandatory_references"] == audit["complete_references"] == 1327,
            "publication capability/identity/provenance findings")
    checked_families(audit["checked"])
    require(report["accepted_bindings"] == 69 and report["bindings_stale_check"] is True,
            "accepted binding manifest")
    require(report["candidate_cache_restored"] is True
            and report["candidate_restore_producers_replayed"] is False
            and report["artifact_promotion"] == "atomic_directory_rename", "transactional artifact issuance")
    if report.get("converged_checkpoint_authenticated"):
        require(report["systems_producers_replayed"] is False
                and type(report["finalization_producer_evaluations"]) is int
                and report["finalization_producer_evaluations"] == 0, "finalization must not replay producers")
    require(report["checkpoint_session"]["accepted_authority"] is False, "checkpoint authority")
    documents = report["documents"]
    require(len(documents) == len({d["path"] for d in documents}) == 21, "document population")
    for doc in documents:
        identifier(doc["document"])
        require(doc["profile"] == report["sysml_profile"] and doc["parsed"] is True and doc["byte_exact"] is True
                and type(doc["recovery_count"]) is int and doc["recovery_count"] == 0
                and doc["construction_gap"] is None, "document status")
        natural(doc["production_count"], "production count", True)


def source_pins(root, report):
    manifest = json.loads((root / "standards/normative/sysml-2.0/library-set.json").read_bytes())
    systems, = [a for a in manifest["artifacts"] if a["specification"] == "SysML"]
    kerml = json.loads((root / "standards/kerml-accepted-publication.json").read_bytes())
    accepted = kerml["complete_overlay"]["identity"]
    require(kerml["status"] == "accepted" and kerml["source_content_set"] == manifest["id"], "KerML authority")
    require(report["accepted_kerml_digest"] == accepted["semantic_digest"]
            and report["accepted_kerml_profile"] == accepted["operational_profile"]
            == "agentique-kerml-1.0-operational/9", "accepted KerML identity")
    library = stable_id(["agentique-pinned-library/1", systems["specification"], systems["version"],
                         systems["source"], systems["sha256"], systems["kpar"]["metamodel"]])
    archive = root / systems["path"]
    with archive.open("rb") as source:
        require(hashlib.file_digest(source, "sha256").hexdigest() == systems["sha256"], "Systems KPAR hash")
        source.seek(0)
        with zipfile.ZipFile(source) as zipped:
            entries = {e["path"]: e for e in systems["entries"] if e["path"].endswith(".sysml")}
            require(set(entries) == {d["path"] for d in report["documents"]}, "exact Systems source population")
            docs = {}
            for doc in report["documents"]:
                entry = entries[doc["path"]]
                raw = zipped.read(doc["path"])
                require(len(raw) == entry["bytes"] and sha(raw) == entry["sha256"] == doc["sha256"], "source bytes")
                expected = stable_id(["agentique-library-document/1", library, doc["path"], doc["sha256"]])
                require(doc["document"] == expected, "source document identity")
                docs[expected] = dict(path=doc["path"], sha256=doc["sha256"], raw=raw,
                    revision=stable_id(["agentique-library-source-revision/1", expected, doc["sha256"]]))
    return manifest["id"], systems["sha256"], library, docs


def source_origin(origin, docs):
    doc = docs[origin["document"]]
    require(origin["revision"] == doc["revision"], "source revision")
    identifier(origin["syntax_node"])
    start, end = origin["range"]["start"], origin["range"]["end"]
    natural(start, "source start")
    natural(end, "source end")
    require(start <= end <= len(doc["raw"]), "source range")
    doc["raw"][start:end].decode("utf-8")
    return doc


def validate(report_path, root):
    raw = report_path.read_bytes()
    report = json.loads(raw)
    accept_report(report)
    source_set, kpar, library, docs = source_pins(root, report)
    directory = report_path.parent
    receipt_raw = (directory / "accepted-publication.json").read_bytes()
    binding_raw = (directory / "standard-bindings.json").read_bytes()
    receipt, bindings = json.loads(receipt_raw), json.loads(binding_raw)
    require(receipt["format"] == "agq-sysml-accepted-publication/1" and receipt["status"] == "accepted", "receipt format")
    require(bindings["format"] == "agq-sysml-accepted-bindings/1", "bindings format")
    require(receipt["source_content_set"] == bindings["source_content_set"] == source_set, "source content set")
    require(pin(receipt["binding_manifest_sha256"]) == sha(encoded(bindings)), "binding manifest digest")
    identity = receipt["identity"]
    for a, b in (("publication_digest", "accepted_systems_digest"), ("semantic_digest", "semantic_digest"),
                 ("accepted_kerml_digest", "accepted_kerml_digest"), ("systems_kpar", "systems_kpar"),
                 ("systems_source_content_set", "systems_source_content_set"), ("operational_profile", "operational_profile"),
                 ("rule_set", "rule_set"), ("producer_registry_digest", "producer_registry_digest"),
                 ("producer_closure_digest", "producer_closure_digest")):
        require(identity[a] is not None and identity[a] == bindings[b], f"receipt binding {a}")
    for field in ("publication_digest", "semantic_digest", "accepted_kerml_digest"):
        require(identity[field] == report[field], f"receipt report {field}")
    rule_sets, semantic_manifest = PROFILES[report["sysml_profile"]]
    require(identity["operational_profile"] == report["sysml_profile"]
            and identity["rule_set"] in rule_sets, "interpretation profile")
    require(identity["systems_kpar"] == kpar and bindings["systems_library"] == library
            and pin(identity["systems_source_content_set"]) == source_set.removeprefix("sha256:"), "Systems identity")
    for field in ("dependency_contract_digest", "combined_descriptor_graph"):
        pin(identity[field])
    for field, path in (("grammar_compatibility_manifest", "standards/grammar/sysml-2.0-operational-v1.json"),
                        ("semantic_correction_manifest", semantic_manifest)):
        require(pin(identity[field]) == sha((root / path).read_bytes().replace(b"\r\n", b"\n")), field)
    for a, b in (("producer_registry_digest", "producer_registry_digest"), ("producer_closure_digest", "digest"),
                 ("producer_context_contract_digest", "context_contract_digest")):
        require(identity[a] == report["producer_closure"][b], f"receipt certificate {a}")
    anchors = bindings["bindings"]
    require(len(anchors) == 69 and {a["role"] for a in anchors} == ROLES, "binding roles")
    for anchor in anchors:
        for field in ("element", "metaclass", "expected_metaclass"):
            identifier(anchor[field])
        doc = source_origin(anchor["source"], docs)
        require(anchor["visibility"] == "public" and anchor["library"] == library
                and anchor["source_path"] == doc["path"] and anchor["source_sha256"] == doc["sha256"], "binding provenance")
        require(anchor["qualified_path"] and all(isinstance(v, str) and v for v in anchor["qualified_path"]), "binding path")
    checkpoint = checkpoint_evidence(report)
    cache_path = directory / "canonical.publication.zip"
    require(Path(report["exported_cache"]).resolve() == cache_path.resolve(), "exported cache path")
    with cache_path.open("rb") as source:
        cache_digest = hashlib.file_digest(source, "sha256").hexdigest()
        source.seek(0)
        with zipfile.ZipFile(source) as archive:
            names = ["closure.json", "facade.json", "kernel.jsonl"]
            require(sorted(archive.namelist()) == names and sorted(receipt["entries"]) == names, "cache entry inventory")
            decoded = {}
            for name in names:
                expected = receipt["entries"][name]
                require(archive.getinfo(name).file_size == natural(expected["bytes"], "entry bytes", True), "entry size")
                with archive.open(name) as entry:
                    if name == "kernel.jsonl":
                        graph = graph_evidence(entry)
                        actual = graph["graph_sha256"]
                    else:
                        require(expected["bytes"] <= 256 * 1024 * 1024, "metadata size limit")
                        data = entry.read(expected["bytes"] + 1)
                        require(len(data) == expected["bytes"], "decoded entry size")
                        actual, decoded[name] = sha(data), json.loads(data)
                require(actual == pin(expected["sha256"]), f"cache entry digest: {name}")
    require(graph["header"]["DependentHeader"]["format"] == "agq-kernel-dependent-evidence-archive/1", "lossless strict graph format")
    for field in ("graph_payload_sha256", "contributions", "selected_reference_digest"):
        require(graph[field] == checkpoint[field], f"cache/checkpoint graph evidence: {field}")
    closure, facade = decoded["closure.json"], decoded["facade.json"]
    require(sha(encoded(closure)) == checkpoint["certificate_receipt_digest"], "cache/checkpoint complete certificate")
    require(closure["format"] == "agq-producer-closure-certificate/2", "closure format")
    for a, b in CERT_FIELDS.items():
        require(closure[a] == report["producer_closure"][b], f"cache closure binding: {a}")
    require(facade["format"] == "agq-sysml-publication-facade/1" and facade["source_content_set"] == source_set, "facade identity")
    require(facade["mandatory_references"] == facade["complete_references"] == 1327, "facade references")
    require(facade["checked"] == report["publication_gate"]["checked"], "facade capabilities")
    project = lambda values: sorted([{k: d[k] for k in DOC_FIELDS} for d in values], key=lambda d: d["path"])
    require(project(facade["documents"]) == project(report["documents"]), "facade documents")
    require(facade["roots"] and len(set(facade["roots"])) == len(facade["roots"]), "facade roots")
    for element in facade["roots"]:
        identifier(element)
    seen = {}
    require(facade["source_map"], "source map missing")
    for fact, origin in facade["source_map"]:
        key = encoded(fact)
        require(key not in seen, "duplicate provenance fact")
        seen[key] = origin
        source_origin(origin, docs)
    for anchor in anchors:
        require(seen.get(encoded({"Element": anchor["element"]})) == anchor["source"],
                "binding source map provenance")
    return dict(format="agq-systems-publication-artifact-verification/1", passed=True,
                grants_publication_authority=False, trusted_restore_still_required=True,
                references=1327, documents=21, kernel_obligations=0, accepted_bindings=69,
                report_sha256=sha(raw), receipt_sha256=sha(receipt_raw), bindings_sha256=sha(binding_raw),
                cache_sha256=cache_digest, source_content_set=source_set,
                publication_digest=pin(report["publication_digest"]),
                semantic_digest=pin(report["semantic_digest"]), checkpoint_evidence=checkpoint)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--repository", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    verification = args.repository.resolve() / "verification"
    require(any(args.output.resolve().is_relative_to(verification / part) for part in ("generated", "summaries")),
            "output must be verification evidence, outside authority/source directories")
    protected = [args.report, *(args.report.parent / name for name in
                 ("canonical.publication.zip", "accepted-publication.json", "standard-bindings.json"))]
    require(args.output.resolve() not in [p.resolve() for p in protected], "output would overwrite evidence")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    try:
        result = validate(args.report, args.repository)
    except Exception as error:
        # A malformed or missing artifact must not leave an older PASS output.
        args.output.write_text(json.dumps(dict(passed=False, grants_publication_authority=False,
                                              error=str(error)), indent=2) + "\n", encoding="utf-8")
        raise
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print("Systems full publication report/artifact verification: PASS (trusted restore remains separate)")


if __name__ == "__main__":
    main()
