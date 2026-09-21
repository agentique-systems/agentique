"""Separate publication acceptance from the scoped KerML conformance report."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from authority import verify

ROOT = Path(__file__).resolve().parents[3]
MILESTONE = ROOT / "verification/kerml-canonical-publication"
GENERATED = ROOT / "verification/generated/kerml-canonical-publication"


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("mode", choices=["publication", "conformance"])
    parser.add_argument("--focused", default="verification/generated/kerml-canonical-publication/focused-closure.json")
    parser.add_argument("--expanded", help="A complete v8 three-library publication audit; never substitute focused evidence")
    args = parser.parse_args()
    authority = verify()
    focused_path = ROOT / args.focused
    focused = load(focused_path)
    identity = {"focused_evidence": args.focused,
                "focused_sha256": hashlib.sha256(focused_path.read_bytes()).hexdigest(),
                "profile": focused["profile"], "scope": focused["scope"],
                "measurements": {
                    "source_records": focused["source_records"],
                    "focused_derived_records": focused["derived_records"],
                    "library_artifacts": len(focused["library_set"]),
                    "source_structural_references": focused["total_mandatory_references"],
                    "references_evaluated": len(focused["reference_findings"]),
                    "scoped_result_producer_complete": focused["producer_complete"],
                    "producer_diagnostic_counts": dict(Counter(d["code"] for d in focused["producer_diagnostics"])),
                }}
    assert focused["profile"] == authority["profile"]
    if args.mode == "conformance":
        report = {**focused["conformance"], **identity,
                  "validator_coverage_is_publication_gate": False}
        assert report["coverage"] == "Incomplete", "Focused partial evidence cannot establish full conformance coverage"
        assert {r["issue"] for r in report["authority_conflicts"]} == {"KERML11-2", "KERML11-4"}
        save(GENERATED / "conformance-report.json", report)
        result = {**identity, "coverage": report["coverage"], "checked_rules": len(report["checked"]),
                  "diagnostic_counts": dict(Counter(d["code"] for d in report["diagnostics"])),
                  "authority_conflicts": report["authority_conflicts"],
                  "validator_coverage_is_publication_gate": False}
        exit_code = 0  # A generated report, never conformance success.
    else:
        matrix = load(MILESTONE / "publication-capabilities.json")
        gaps = [r["family"] for r in matrix["capabilities"] if r["status"] != "Complete"]
        blockers = []
        if gaps:
            blockers.append("publication capabilities remain incomplete")
        if focused["focused_failures"]:
            blockers.append("focused reference/namespace regressions failed")
        if not focused["producer_complete"]:
            blockers.append("focused producer closure remains incomplete")
        expanded = load(ROOT / args.expanded) if args.expanded else None
        if expanded is None:
            blockers.append("complete three-library expanded publication gate not run")
        else:
            assert expanded["profile"] == authority["profile"] and expanded["full_expansion"]
            refs = expanded["mandatory_references"]
            for key in ["unresolved", "incomplete", "ambiguous", "invalid"]:
                if refs[key] != 0:
                    blockers.append(f"mandatory reference {key}")
            for key in ["publication_blocking_authority_conflicts", "publication_critical_graph_contradictions"]:
                if expanded[key] != 0:
                    blockers.append(key)
        # No accepted facade/integration constructor exists in this milestone.
        # Caller-supplied success booleans are not an acceptance proof.
        for field in ["canonical_facade", "accepted_bindings", "authored_publication_integration"]:
            blockers.append(field + " not accepted")
        result = {**identity, "accepted": not blockers, "blockers": blockers,
                  "publication_blocking_authority_conflicts": authority["publication_blocking_authority_conflicts"],
                  "capabilities_incomplete": gaps,
                  "focused_references": {"count": len(focused["reference_findings"]),
                      "incomplete": sum(r["completeness"] != "Complete" for r in focused["reference_findings"]),
                      "unresolved": sum(len(r["targets"]) == 0 for r in focused["reference_findings"]),
                      "ambiguous": sum(len(r["targets"]) > 1 for r in focused["reference_findings"]),
                      "invalid": sum(not r["canonical_endpoint_valid"] for r in focused["reference_findings"])},
                  "effective_namespaces": focused["effective_namespaces"],
                  "full_expansion_run": expanded is not None,
                  "validator_coverage_is_publication_gate": False}
        save(GENERATED / "publication-gate.json", result)
        exit_code = int(bool(blockers))
    # Curated results are small. Raw queries, diagnostics and command logs stay ignored.
    summary_path = MILESTONE / "summary.json"
    summary = load(summary_path)
    summary[args.mode] = result
    save(summary_path, summary)
    print(json.dumps(result, indent=2))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
