"""Separate publication gate and conformance report over measured audit output.

Reporting conformance exits zero when a report was generated, irrespective of its
verdict. The publication gate exits nonzero until every structural obligation is
proved. Neither operation acquires authority or accepts a library facade.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from authority import verify
from classify import build

ROOT = Path(__file__).resolve().parents[3]
MILESTONE = ROOT / "verification/kerml-publication-convergence"
GENERATED = ROOT / "verification/generated/kerml-publication-convergence"


def publication_blockers(audit, review, derived):
    blocked = [b["id"] for b in review["blockers"]
               if b["impact"] == "PublicationBlockingAuthorityConflict"]
    if not audit.get("kernel_storage_valid"):
        blocked.append("strict-kernel-snapshot")
    if audit.get("profile") != review["profile"]:
        blocked.append("exact-profile")
    refs = audit.get("expanded_references")
    if audit.get("validator_scope") != "CompletePublicationOverlay":
        blocked.append("complete-publication-overlay-not-measured")
    if not refs:
        refs = {k: audit.get(k) for k in ["unresolved", "incomplete", "ambiguous"]}
        refs["invalid"] = audit.get("invalid_references")
    if any(refs.get(k) != 0 for k in ["unresolved", "incomplete", "ambiguous", "invalid"]):
        blocked.append("mandatory-reference-closure")
    if audit.get("distinguishability") != 0:
        blocked.append("effective-namespace-ambiguity")
    if not derived["closure_complete"]:
        blocked.append("publication-critical-derived-and-producer-closure")
    # Missing evidence is not successful evidence. These are measured graph
    # obligations, independent of the number of implemented validator rules.
    for requirement in ["identity_and_provenance_verified", "complete_publication_overlay"]:
        if audit.get(requirement) is not True:
            blocked.append(requirement)
    return blocked


def curate(key, value):
    path = MILESTONE / "summary.json"
    summary = json.loads(path.read_text())
    summary[key] = value
    path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("mode", choices=["publication", "conformance"])
    parser.add_argument("--audit", required=True)
    args = parser.parse_args()
    audit_path = ROOT / args.audit
    audit = json.loads(audit_path.read_text())
    identity = {"audit": audit_path.relative_to(ROOT).as_posix(),
                "audit_sha256": hashlib.sha256(audit_path.read_bytes()).hexdigest(),
                "profile": audit["profile"], "library_set": audit["library_set"],
                "scope": audit["validator_scope"]}
    review = verify()
    inventory = build()
    classifications = {r["rule"]: r for r in inventory["constraints"]}
    GENERATED.mkdir(parents=True, exist_ok=True)
    if args.mode == "conformance":
        checked = set(audit["evaluated_rules"])
        missing = sorted(set(classifications) - checked)
        unknown = sorted(checked - set(classifications))
        authority_impact = {b["id"]: b["impact"] for b in review["blockers"]}
        diagnostics = []
        for diagnostic in audit["validation_findings"]:
            rule = classifications.get(diagnostic["code"])
            relevance = rule["relevance"] if rule else "PublicationCritical"
            critical = relevance == "PublicationCritical" or (
                relevance == "AuthorityBlocked" and any(
                    authority_impact[i] == "PublicationBlockingAuthorityConflict"
                    for i in rule["authority_conflicts"]))
            diagnostics.append({**diagnostic, "publication_relevance": relevance,
                                "publication_critical": critical})
        report = {"format": "agentique-kerml-conformance-report/1", **identity,
                  "coverage": "Incomplete" if missing or unknown or audit["deferred_by_phase"] else "Complete",
                  "evaluated_rules": audit["evaluated_rules"],
                  "unknown_evaluated_rules": unknown,
                  "deferred_by_phase": audit["deferred_by_phase"],
                  "unimplemented_or_not_evaluated": [{"rule": n, "relevance": classifications[n]["relevance"]} for n in missing],
                  "diagnostics": diagnostics,
                  "diagnostic_counts": dict(Counter(r["code"] for r in audit["validation_findings"])),
                  "authority_conflicts": review["blockers"],
                  "canonical_publication_accepted": False}
        (GENERATED / "conformance-report.json").write_text(json.dumps(report, indent=2) + "\n")
        curate("conformance", {**identity, "coverage": report["coverage"],
                               "evaluated_rule_count": len(checked),
                               "unimplemented_or_not_evaluated_by_relevance": dict(Counter(classifications[n]["relevance"] for n in missing)),
                               "deferred_by_phase": audit["deferred_by_phase"],
                               "diagnostic_counts": report["diagnostic_counts"]})
        print(f"Conformance report generated: coverage {report['coverage']}; {len(checked)} rules evaluated on {identity['scope']}")
        return 0
    derived = json.loads((MILESTONE / "publication-critical-derived.json").read_text())
    blocked = publication_blockers(audit, review, derived)
    result = {"format": "agentique-canonical-publication-assessment/1", **identity,
              "accepted": not blocked, "blockers": blocked,
              "sysml_ready": not blocked and audit.get("accepted_bindings") is True
                             and audit.get("authored_publication_integration") is True,
              "downstream_acceptance": {k: audit.get(k, False) for k in
                                        ["accepted_bindings", "authored_publication_integration"]},
              "source_records": audit["canonical_records"],
              "references": {"required": audit["required_references"],
                             **{k: audit[k] for k in ["unresolved", "incomplete", "ambiguous", "invalid_references"]}},
              "reference_findings": audit["reference_findings"],
              "effective_namespace_findings": audit["distinguishability"],
              "validator_coverage_is_publication_gate": False}
    (GENERATED / "publication-gate.json").write_text(json.dumps(result, indent=2) + "\n")
    curate("publication", result)
    print("Canonical publication INCOMPLETE: " + ", ".join(blocked))
    return int(bool(blocked))


if __name__ == "__main__":
    raise SystemExit(main())
