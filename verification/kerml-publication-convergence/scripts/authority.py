"""Check retained exact witnesses locally; never acquire issue-tracker data."""
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
MILESTONE = ROOT / "verification/kerml-publication-convergence"


def verify():
    review = json.loads((MILESTONE / "authority-blockers.json").read_text())
    register = json.loads((ROOT / review["register"]).read_text())
    assert review["profile"] == register["profile"]
    assert not review["new_corrections_authorized"]
    rows = {b["id"]: b for b in register["blockers"]}
    assert set(rows) == {b["id"] for b in review["blockers"]}
    source_files = {hashlib.sha256(p.read_bytes()).hexdigest(): p
                    for p in (ROOT / "standards/libraries").rglob("*.kerml")}
    witnesses = 0
    for b in rows.values():
        for witness in b["pinned_witnesses"]:
            source = witness["source"]
            raw = source_files[source["sha256"]].read_bytes()
            start, end = source["range"]
            assert raw[start:end].decode() == source["text"], witness["id"]
            witnesses += 1
    xmi = ROOT / "standards/normative/kerml-1.0/KerML.xmi"
    assert hashlib.sha256(xmi.read_bytes()).hexdigest() == register["normative_xmi_sha256"]
    imports = [e for e in ET.parse(xmi).iter()
               if e.tag == "ownedOperation" and e.get("name") == "importedMemberships"]
    namespace = next(e for e in imports if any(
        c.get("body") == "ownedImport.importedMemberships(excluded->including(self))" for c in e.iter()))
    assert any("distinguisibility collisions" in c.get("body", "") for c in namespace.iter())
    collisions = rows["KLCV10-F-004"]["complete_local_structural_facts"]["collisions"]
    assert collisions and all(c["imported_membership"] != c["local_membership"] for c in collisions)
    print(f"Verified {witnesses} exact retained source witnesses; {len(collisions)} distinct imported/local membership collisions")
    return review


if __name__ == "__main__":
    verify()
