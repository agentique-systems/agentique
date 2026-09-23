"""Fail-closed semantic gate before the single full Systems publication attempt.

Report digests bind exact graph/certificate semantics; counts are additional
acceptance requirements. Synthetic tests separately compare complete structures.
This script neither issues a trusted publication nor changes accepted authority.
"""
import argparse
import hashlib
import json
from pathlib import Path
from frontier_artifact_evidence import checkpoint_evidence


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(value, name):
    require(isinstance(value, list) and len(value) == 32
            and all(type(x) is int and 0 <= x <= 255 for x in value),
            f"invalid or missing {name}")
    return bytes(value).hex()


IDENTITY_FIELDS = (
    "scope", "sysml_profile", "accepted_kerml_digest", "accepted_kerml_profile",
    "documents", "authority_targets", "authority_conflicts", "local_elements",
    "mandatory_references", "reference_audit_scope", "closure_explanations",
)
CERTIFICATE_DIGESTS = (
    "model_digest", "semantic_closure_digest", "digest", "context_contract_digest",
    "producer_registry_digest",
)
BASELINE_SHA256 = "9aa5332692a443a1bc8cb1a761a7207714f83116e816a1e7a6d0201dfaad3288"


def accept(report, label):
    require(report.get("scoped_preflight_passed") is True, f"{label}: preflight")
    require(report.get("construction_complete") is True, f"{label}: construction")
    for key in ("systems_documents_parsed", "systems_documents_constructed",
                "systems_documents_byte_exact"):
        require(report.get(key) == 13, f"{label}: {key}")
    require(report.get("kernel_obligations") == 0, f"{label}: obligations")
    require(report.get("kernel_obligation_details") == [], f"{label}: obligations detail")
    refs = report["mandatory_references"]
    require(refs["total"] == 695 and refs["failures"] == [], f"{label}: references")
    require(refs["counts"] == dict(complete=695, unresolved=0, incomplete=0,
                                  ambiguous=0, invalid=0, endpoint_mismatch=0),
            f"{label}: reference acceptance")
    require(report.get("authority_conflicts") == [], f"{label}: authority conflicts")
    require(report.get("publication_accepted") is False
            and report.get("publication_attempted") is False,
            f"{label}: medium is unaccepted research evidence")
    require(report.get("kerml_producers_replayed") is False, f"{label}: KerML replay")
    producer = report["construction_producers"]
    require(producer["converged"] is True and producer["completeness"] == "Complete"
            and producer["final_predicates"] is True
            and producer["round_limit_reached"] is False
            and producer["diagnostics"] == [], f"{label}: producer closure")
    cert = report["producer_closure"]
    require(cert["fully_closed"] is True and cert["incomplete_pairs"] == 0
            and cert["applicable_pairs"] == cert["closed_pairs"] == 14791
            and cert["closed_requirements"] == cert["required_requirements"] == 417786,
            f"{label}: certificate closure")
    digest(report.get("accepted_kerml_digest"), "accepted KerML")
    for key in CERTIFICATE_DIGESTS:
        digest(cert.get(key), key)


def compare(baseline, uninterrupted, resumed):
    for label, report in (("baseline", baseline), ("uninterrupted", uninterrupted),
                          ("resumed", resumed)):
        accept(report, label)
    for key in IDENTITY_FIELDS:
        require(baseline[key] == uninterrupted[key] == resumed[key], f"changed {key}")
    for key in CERTIFICATE_DIGESTS:
        require(baseline["producer_closure"][key] == uninterrupted["producer_closure"][key]
                == resumed["producer_closure"][key], f"changed certificate {key}")
    for report in (uninterrupted, resumed):
        require(report.get("incremental_certificate_reference_check") is True,
                "missing full-rebuild reference comparison")
        require(report["checkpoint_session"]["accepted_authority"] is False,
                "checkpoint cannot establish acceptance")
        digest(report.get("construction_reference_semantic_digest"), "reference results")
        digest(report["producer_closure"].get("revalidation_digest"), "transport evidence")
    require(uninterrupted["construction_reference_semantic_digest"]
            == resumed["construction_reference_semantic_digest"], "changed query results")
    require(uninterrupted["producer_closure"]["revalidation_digest"]
            == resumed["producer_closure"]["revalidation_digest"], "changed transport evidence")
    initial = uninterrupted["checkpoint_session"]["statistics"]
    require(initial["restored_invocations"] == 0 and initial["committed_checkpoints"] > 0,
            "uninterrupted run did not create checkpoints")
    restored = resumed["checkpoint_session"]["statistics"]
    require(restored["restored_invocations"] > restored["restored_completed_invocations"]
            and restored["skipped_rounds"] > 0 and restored["committed_checkpoints"] > 0,
            "resume must continue an unfinished scheduler invocation")
    return {"format": "agq-medium-frontier-equivalence/1", "passed": True,
            "references": 695, "kernel_obligations": 0,
            "certificate_digests": {key: digest(uninterrupted["producer_closure"][key], key)
                                    for key in (*CERTIFICATE_DIGESTS, "revalidation_digest")},
            "reference_results_digest": digest(uninterrupted["construction_reference_semantic_digest"],
                                               "reference results"),
            "resumed_scheduler_statistics": restored}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("baseline", "uninterrupted", "resumed", "output"):
        parser.add_argument(f"--{name}", required=True, type=Path)
    args = parser.parse_args()
    paths = (args.baseline, args.uninterrupted, args.resumed)
    data = [path.read_bytes() for path in paths]
    require(hashlib.sha256(data[0]).hexdigest() == BASELINE_SHA256,
            "baseline differs from retained prior medium evidence")
    result = compare(*(json.loads(raw) for raw in data))
    current = [checkpoint_evidence(json.loads(raw)) for raw in data[1:]]
    for key in ("source_identity", "contributions", "selected_reference_digest"):
        require(current[0][key] == current[1][key], f"changed checkpoint evidence: {key}")
    result["checkpoint_artifact_evidence"] = current
    result["historical_comparison_scope"] = (
        "Pinned prior report graph/aggregate proof/search and certificate digests; "
        "prior report has no selected-contribution archive. Authenticated selected "
        "contribution proofs/searches additionally match uninterrupted and resumed runs.")
    result["evidence"] = [{"path": str(path), "sha256": hashlib.sha256(raw).hexdigest()}
                          for path, raw in zip(paths, data)]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print("Medium exact semantic equivalence: PASS")


if __name__ == "__main__":
    main()
