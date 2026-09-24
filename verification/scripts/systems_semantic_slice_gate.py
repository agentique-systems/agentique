"""Fail-closed gates for the fresh combined16 and prior exact medium13 scopes.

This validates scoped semantic evidence only. It never accepts a publication,
issues a receipt, activates a catalogue, or substitutes for the full finalizer.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import zipfile

from classify_systems_final_audit import classify, summarize
from frontier_artifact_evidence import pin, require
from systems_publication_gate import encoded, identifier, natural, sha, stable_id

PROFILE = "agentique-sysml-2.0-operational/3"
FIXTURES = Path("verification/fixtures/final-audit-semantic-closure")
# Pins identify reviewed JSON values, independent of checkout line endings.
PLAN_DIGEST = "33f6b44287893562c9952d28b646f3ebc96c219dbc6ede86d546487dfcaa7b8a"
CLASSIFICATION_DIGEST = "a52798aa16e4dfa8c236712d16c1344897d97b76ce56f46b7e68a40b2da0c2f9"
SCOPES = {
    "H1-H3-combined": (16, None),
    "H4-medium": (13, 695),
}
AUDIT_FAMILIES = set("DefinitionUsage AttributeItemPart OccurrenceActionState "
                     "CalculationConstraintRequirementCase PortConnectionInterfaceFlow "
                     "ViewMetadata MayTimeVary".split())
CERTIFICATE_DIGESTS = (
    "model_digest", "digest", "semantic_closure_digest", "revalidation_digest",
    "producer_registry_digest", "context_contract_digest",
)
REFERENCE_STATES = set("complete unresolved incomplete ambiguous invalid endpoint_mismatch".split())


def seconds(value, label):
    require(type(value) in (int, float) and math.isfinite(value) and value >= 0,
            f"invalid {label}")
    return value


def pinned_fixture(root, name, expected):
    raw = (root / FIXTURES / name).read_bytes()
    value = json.loads(raw)
    require(sha(encoded(value)) == expected, f"changed reviewed fixture: {name}")
    return value, dict(path=str(FIXTURES / name), sha256=sha(raw), content_digest=expected)


def reviewed_scope(root, scope):
    require(scope in SCOPES, "unsupported slice")
    plan, plan_pin = pinned_fixture(root, "slices.json", PLAN_DIGEST)
    prior, prior_pin = pinned_fixture(root, "prior-classification.json", CLASSIFICATION_DIGEST)
    require(plan["format"] == "agq-final-audit-slice-plan/1"
            and plan["publication_authority"] is False and plan["semantic_gates_run"] is False,
            "slice plan is not publication or execution evidence")
    require(prior["format"] == "agq-final-audit-classification/1"
            and prior["publication_authority"] is False, "classification authority")
    rows = classify(prior["diagnostics"], prior["subjects"])
    summary = summarize(rows)
    require(rows == prior["diagnostics"] and summary == prior["summary"]
            and summary["weighted_findings"] == 674 and summary["distinct_diagnostics"] == 238
            and summary["subjects"] == 50
            and summary["categories"]["Other"] == dict(distinct_diagnostics=0, weighted_findings=0),
            "prior classification is incomplete or unexplained")
    subjects = plan["all_prior_subjects"]
    require(len(subjects) == len({row["subject"] for row in subjects}) == 50
            and {row["subject"] for row in subjects} == {row["subject"] for row in rows},
            "prior subject mapping")
    require(all(type(row["accepted_dependency"]) is bool for row in subjects)
            and sum(row["accepted_dependency"] for row in subjects) == 3,
            "accepted dependency target mapping")
    combined = {plan["documents"][name]["path"]
                for name in plan["slices"]["H1-H3-combined"]["documents"]}
    require(all(row["accepted_dependency"] or row["document"] in combined for row in subjects),
            "combined slice does not cover all prior local subjects")
    count, _ = SCOPES[scope]
    selection = plan["slices"][scope]
    names = selection["documents"]
    require(selection["document_count"] == len(names) == len(set(names)) == count,
            "reviewed slice document population")
    expected = {plan["documents"][name]["path"]: plan["documents"][name] for name in names}
    require(len(expected) == count, "reviewed slice paths")
    return plan, expected, summary, [plan_pin, prior_pin]


def reference_counts(value, expected, label, sparse=False):
    counts = value["counts"]
    require(isinstance(counts, dict) and set(counts) <= REFERENCE_STATES
            and (sparse or set(counts) == REFERENCE_STATES), f"{label}: reference states")
    total = natural(value["total"], f"{label} reference total")
    require(all(natural(count, f"{label} {state}") == (total if state == "complete" else 0)
                for state, count in counts.items())
            and counts.get("complete", 0) == total, f"{label}: incomplete references")
    require(expected is None or total == expected, f"{label}: exact reference count")
    return total


def accept_report(report, scope, expected):
    count, expected_references = SCOPES[scope]
    require(report["format"] == "agq-sysml-systems-publication-audit/1", "report format")
    require(isinstance(report["scope"], list) and len(report["scope"]) == count
            and set(report["scope"]) == set(expected), "exact scoped document population")
    require(report["sysml_profile"] == PROFILE, "explicit v3 interpretation required")
    require(report["scoped_preflight_passed"] is True and report["construction_complete"] is True,
            "scoped construction/preflight")
    require(report["publication_attempted"] is False and report["publication_accepted"] is False
            and "publication_error" not in report, "slice cannot carry publication authority")
    for field in ("systems_documents_parsed", "systems_documents_constructed", "systems_documents_byte_exact"):
        require(natural(report[field], field) == count, field)
    require(natural(report["kernel_obligations"], "kernel obligations") == 0
            and report["kernel_obligation_details"] == [], "kernel obligations")
    require(report["authority_conflicts"] == [], "authority conflicts")
    require(report["accepted_kerml_profile"] == "agentique-kerml-1.0-operational/9"
            and report["kerml_producers_replayed"] is False, "accepted KerML v9 without replay")
    pin(report["accepted_kerml_digest"])
    pin(report["construction_reference_semantic_digest"])
    require(report["reference_audit_scope"] == "construction", "scoped reference audit")
    references = reference_counts(report["mandatory_references"], expected_references, scope)
    require(references > 0 and report["mandatory_references"]["failures"] == [], "mandatory references")
    documents = report["documents"]
    require(len(documents) == len({d["path"] for d in documents}) == count
            and {d["path"] for d in documents} == set(expected), "exact document records")
    per_document_total = 0
    for doc in documents:
        source = expected[doc["path"]]
        require(doc["sha256"] == source["sha256"] and doc["document"] == source["document"],
                "document source identity")
        identifier(doc["document"])
        require(doc["profile"] == PROFILE and doc["parsed"] is True and doc["byte_exact"] is True
                and natural(doc["recovery_count"], "recovery count") == 0
                and doc["construction_gap"] is None, "document parse/construction status")
        natural(doc["production_count"], "production count", True)
        require(doc["reference_audit_scope"] == "construction", "document reference scope")
        per_document_total += reference_counts(doc["mandatory_references"], None, doc["path"], sparse=True)
    require(per_document_total == references, "document/global reference population mismatch")
    producer = report["construction_producers"]
    require(producer["final_predicates"] is True and producer["converged"] is True
            and producer["completeness"] == "Complete" and producer["diagnostics"] == []
            and producer["round_limit_reached"] is False, "producer closure or diagnostics")
    require(natural(producer["rounds"], "producer rounds", True)
            <= natural(producer["round_limit"], "producer round limit", True), "producer round limit")
    certificate = report["producer_closure"]
    require(certificate["fully_closed"] is True
            and natural(certificate["incomplete_pairs"], "incomplete pairs") == 0, "open producer pairs")
    for applicable, closed in (("applicable_pairs", "closed_pairs"), ("required_requirements", "closed_requirements")):
        require(natural(certificate[applicable], applicable, True)
                == natural(certificate[closed], closed, True), f"open {applicable}")
    for field in CERTIFICATE_DIGESTS:
        pin(certificate[field])
    audit = report["effective_sysml_audit"]
    require(audit["publication_authority"] is False and audit["findings"] == [],
            "strict effective SysML audit findings or authority")
    require(set(audit["checked"]) == AUDIT_FAMILIES, "effective audit capability coverage")
    for family, checked in audit["checked"].items():
        natural(checked, f"effective {family} checks", True)
    require(type(audit["workers"]) is int and audit["workers"] in (1, 2), "bounded effective audit workers")
    seconds(audit["elapsed_seconds"], "effective audit duration")
    seconds(report["elapsed_seconds"], "construction report duration")
    if report["checkpoint_session"] is not None:
        require(report["checkpoint_session"]["accepted_authority"] is False, "checkpoint authority")
    return references


def source_pins(root, report, plan, expected):
    manifest = json.loads((root / "standards/normative/sysml-2.0/library-set.json").read_bytes())
    systems, = [entry for entry in manifest["artifacts"] if entry["specification"] == "SysML"]
    kerml = json.loads((root / "standards/kerml-accepted-publication.json").read_bytes())
    accepted = kerml["complete_overlay"]["identity"]
    require(kerml["status"] == "accepted" and kerml["source_content_set"] == manifest["id"]
            == plan["source_content_set"], "source content-set authority")
    require(report["accepted_kerml_digest"] == accepted["semantic_digest"]
            and report["accepted_kerml_profile"] == accepted["operational_profile"], "accepted KerML identity")
    require(pin(kerml["complete_overlay"]["graph_sha256"]) == plan["accepted_kerml_graph_sha256"],
            "accepted KerML mapped target graph")
    require(systems["sha256"] == plan["systems_kpar_sha256"], "Systems archive identity")
    library = stable_id(["agentique-pinned-library/1", systems["specification"], systems["version"],
                         systems["source"], systems["sha256"], systems["kpar"]["metamodel"]])
    with (root / systems["path"]).open("rb") as source:
        require(hashlib.file_digest(source, "sha256").hexdigest() == systems["sha256"], "Systems KPAR bytes")
        source.seek(0)
        with zipfile.ZipFile(source) as archive:
            entries = {entry["path"]: entry for entry in systems["entries"]}
            for path, doc in expected.items():
                entry, raw = entries[path], archive.read(path)
                require(len(raw) == entry["bytes"] and sha(raw) == entry["sha256"] == doc["sha256"],
                        "pinned document bytes")
                require(doc["document"] == stable_id(["agentique-library-document/1", library, path, doc["sha256"]]),
                        "pinned document identity")
    return manifest["id"], systems["sha256"]


def validate(report_path, root, scope):
    plan, expected, prior, fixtures = reviewed_scope(root, scope)
    raw = report_path.read_bytes()
    report = json.loads(raw)
    references = accept_report(report, scope, expected)
    content_set, kpar = source_pins(root, report, plan, expected)
    covered = [row for row in plan["all_prior_subjects"]
               if row["accepted_dependency"] or row["document"] in expected]
    return dict(format="agq-systems-semantic-slice-gate/1", passed=True, publication_authority=False,
        slice=scope, documents=len(expected), mandatory_references_complete=references,
        reference_count_basis="retained exact medium13 count" if scope == "H4-medium"
            else "fresh source construction, complete per-document/global population; no retained exact count",
        closed_producer_pairs=report["producer_closure"]["closed_pairs"],
        closed_requirements=report["producer_closure"]["closed_requirements"],
        effective_findings=0, effective_checked=report["effective_sysml_audit"]["checked"],
        effective_audit_elapsed_seconds=report["effective_sysml_audit"]["elapsed_seconds"],
        construction_report_elapsed_seconds=report["elapsed_seconds"],
        workers=report["effective_sysml_audit"]["workers"],
        prior_classification=prior,
        prior_subject_coverage=dict(local=sum(not row["accepted_dependency"] for row in covered),
            accepted_dependency_targets=sum(row["accepted_dependency"] for row in covered),
            total=len(covered), meaning="source/dependency coverage; not individual diagnostic closure evidence"),
        certificate_digests={field: pin(report["producer_closure"][field]) for field in CERTIFICATE_DIGESTS},
        reference_results_digest=pin(report["construction_reference_semantic_digest"]),
        sysml_profile=PROFILE, source_content_set=content_set, systems_kpar_sha256=kpar,
        accepted_kerml_digest=pin(report["accepted_kerml_digest"]),
        evidence=[dict(path=str(report_path), sha256=sha(raw)), *fixtures])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--slice", choices=SCOPES, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = validate(args.report, Path(__file__).resolve().parents[2], args.slice)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"{args.slice}: PASS; documents={result['documents']} references={result['mandatory_references_complete']} "
          f"effective_findings=0 audit_seconds={result['effective_audit_elapsed_seconds']:.3f}; publication_authority=false")


if __name__ == "__main__":
    main()
