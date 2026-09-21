"""Verify the two authorized v8 interpretations against local pinned authority."""
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]


def read(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def check_pin(pin):
    assert hashlib.sha256((ROOT / pin["path"]).read_bytes()).hexdigest() == pin["sha256"]


def verify():
    profile = read("standards/kerml-1.0-operational-profile-v8.json")
    assert profile["profile_id"] == "agentique-kerml-1.0-operational/8"
    assert profile["extends"]["profile_id"] == "agentique-kerml-1.0-operational/7"
    check_pin(profile["extends"])
    assert len(profile["manifests"]) == 2
    for pin in profile["manifests"]:
        check_pin(pin)
        manifest = read(pin["path"])
        assert manifest["profile_id"] == profile["profile_id"]
        assert manifest["extends"] == profile["extends"]
        check_pin(manifest["normative_artifact"])
    xmi = ET.parse(ROOT / "standards/normative/kerml-1.0/KerML.xmi")
    cross = next(n for n in xmi.iter("ownedRule") if n.get("name") == "checkFeatureOwnedCrossFeatureTypeFeaturing")
    assert any("endFeature->excluding(self)" in n.get("body", "") for n in cross.iter())
    namespace = next(n for n in xmi.iter("ownedOperation") if n.get("name") == "importedMemberships"
                     and any(c.get("body") == "ownedImport.importedMemberships(excluded->including(self))" for c in n.iter()))
    assert any("distinguisibility collisions" in n.get("body", "") for n in namespace.iter())
    old = read("verification/kerml-publication-convergence/authority-blockers.json")
    new = read("verification/kerml-canonical-publication/authority-decisions.json")
    before = {r["id"]: r for r in old["blockers"]}
    after = {r["id"]: r for r in new["decisions"]}
    assert before.keys() == after.keys()
    for identifier, row in after.items():
        assert row["prior_impact"] == before[identifier]["impact"]
        expected = ("CoveredByOperationalV8" if identifier in ["KLCV10-F-003", "KLCV10-F-004"]
                    else "ValidationOnlyAuthorityConflict")
        assert row["impact"] == expected
    register = read("standards/kerml-1.0-operational-authority-blockers.json")
    sources = {hashlib.sha256(p.read_bytes()).hexdigest(): p for p in (ROOT / "standards/libraries").rglob("*.kerml")}
    witnesses = 0
    for row in register["blockers"]:
        for witness in row["pinned_witnesses"]:
            source = witness["source"]
            start, end = source["range"]
            assert sources[source["sha256"]].read_bytes()[start:end].decode() == source["text"]
            witnesses += 1
    print(f"Operational v8: two pinned interpretations; {witnesses} unchanged source witnesses; two retained validation-only conflicts")
    return new


if __name__ == "__main__":
    verify()
