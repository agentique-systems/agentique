"""Check existing Studio inputs without acquiring or reconstructing publications."""
import json
import os
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
INPUTS = {
    "AGENTIQUE_KERML_CACHE": "verification/generated/kerml-v9-publication/canonical.publication.zip",
    "AGENTIQUE_SYSTEMS_CACHE": "verification/generated/final-audit-semantic-closure/accepted/canonical.publication.zip",
    "AGENTIQUE_SELF_MODEL_DB": "verification/generated/modeling-platform-phase2/agentique-dogfood.sqlite",
}


def main():
    observations = []
    for variable, default in INPUTS.items():
        path = Path(os.environ.get(variable, ROOT / default))
        observations.append({
            "environment_variable": variable,
            "path": str(path),
            "explicit_override": variable in os.environ,
            "exists": path.is_file(),
            "bytes": path.stat().st_size if path.is_file() else None,
        })
    ready = all(item["exists"] for item in observations[:2])
    print(json.dumps({
        "publication_inputs_present": ready,
        "retained_self_model_present": observations[2]["exists"],
        "inputs": observations,
        "note": "Presence is a precondition, not authentication or acceptance. Runtime restoration authenticates exact publication identities.",
    }, indent=2))
    raise SystemExit(0 if ready else 2)


if __name__ == "__main__":
    main()
