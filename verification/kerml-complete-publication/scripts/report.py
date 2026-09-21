"""Curate gate results without copying raw corpus diagnostics (ADR 0021)."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
DEST = ROOT / "verification/kerml-complete-publication"


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--focused", type=Path)
    parser.add_argument("--full", type=Path)
    parser.add_argument("--conformance", type=Path)
    parser.add_argument("--resources", type=Path)
    args = parser.parse_args()
    baseline = read(ROOT / "verification/kerml-canonical-publication/publication-capabilities.json")
    artifacts = {}
    for label, path in [("focused", args.focused), ("full", args.full), ("conformance", args.conformance), ("resources", args.resources)]:
        if path is not None:
            data = (ROOT / path).read_bytes()
            artifacts[label] = {"path": path.as_posix(), "sha256": hashlib.sha256(data).hexdigest()}
    focused = read(ROOT / args.focused) if args.focused else None
    full = read(ROOT / args.full) if args.full else None
    conformance = read(ROOT / args.conformance) if args.conformance else None
    observations = [json.loads(line) for line in (ROOT / args.resources).read_text(encoding="utf-8-sig").splitlines() if line.strip()] if args.resources else []
    overlay_accepted = bool(full and full.get("accepted_overlay_and_references")
                            and len(full.get("capabilities", [])) == 13
                            and all(row["status"] == "Complete" for row in full["capabilities"]))
    commands = {row["name"]: row for row in read(DEST / "summary.json")["commands"]}
    bindings_accepted = commands.get("accepted-binding-manifest-stale", {}).get("exit_code") == 0
    facade_accepted = bool(overlay_accepted and full.get("canonical_facade_accepted"))
    authored_accepted = bool(facade_accepted and full.get("authored_consumption_verified"))
    capabilities = []
    for row in baseline["capabilities"]:
        capabilities.append({"family": row["family"], "status": "Complete" if overlay_accepted else row["status"],
                             "evidence": "full complete-overlay gate" if overlay_accepted else
                             ("retained prior acceptance" if row["status"] == "Complete" else
                              "Focused implementation evidence exists; complete corpus closure remains unproved")})
    matrix = {"format": "agentique-publication-capabilities/2", "profile": baseline["profile"],
              "validator_coverage_is_publication_gate": False, "capabilities": capabilities}
    result = {"format": "agentique-kerml-complete-publication-result/1", "profile": baseline["profile"],
              "status": "Complete" if authored_accepted and bindings_accepted else "Incomplete", "complete_overlay_gate_passed": overlay_accepted,
              "focused_failures": focused.get("focused_failures") if focused else None,
              "mandatory_references": full.get("mandatory_references") if full else None,
              "full_expansion_run": full is not None, "publication_blocking_authority_conflicts": 0,
              "authority_decisions": "verification/kerml-canonical-publication/authority-decisions.json",
              "canonical_facade_accepted": facade_accepted, "accepted_bindings_regenerated": bindings_accepted,
              "authored_consumption_verified": authored_accepted,
              "conformance_coverage": conformance.get("coverage", "Incomplete") if conformance else "Incomplete",
              "current_gate_failure": (full.get("failure", "")[:800] or None) if full else ("Focused publication audit has failures" if focused and focused["focused_failures"] else "Full publication gate remains unproved"),
              "resource_observations": {"samples": len(observations),
                "sampled_peak_working_set_bytes": max((r["peak_working_set_bytes"] for r in observations), default=None),
                "maximum_observed_private_bytes": max((r["private_bytes"] for r in observations), default=None),
                "timing_threshold_is_correctness_gate": False},
              "artifacts": artifacts}
    for name, data in [("capability-status.json", matrix), ("publication-result.json", result)]:
        (DEST / name).write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
