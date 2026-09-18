"""Offline, exhaustive source inventory; deliberately not a rule interpreter."""
from pathlib import Path
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "standards/sysml-semantic-coverage.json"
X = "{http://www.omg.org/spec/XMI/20161101}"
SLICE = set("Definition Usage OccurrenceDefinition OccurrenceUsage ItemDefinition ItemUsage PartDefinition PartUsage AttributeDefinition AttributeUsage PortDefinition PortUsage ConjugatedPortDefinition PortConjugation ConnectionDefinition ConnectionUsage ConnectorAsUsage InterfaceDefinition InterfaceUsage".split())


def digest(data):
    return hashlib.sha256(data).hexdigest()


def authority(language, version):
    directory = ROOT / f"standards/normative/{language.lower()}-{version}"
    lock = json.loads((directory / "lock.json").read_text())
    item = next(a for a in lock["artifacts"] if a["path"].endswith(f"{language}.xmi"))
    data = (ROOT / item["path"]).read_bytes()
    assert digest(data) == item["sha256"], "Normative XMI changed"
    tree = ET.fromstring(data)
    nodes = {e.get(X + "id"): e for e in tree.iter() if e.get(X + "id")}
    classes = {i: e for i, e in nodes.items() if e.get(X + "type") == "uml:Class"}
    return item, nodes, classes


def target(e):
    return e.get(X + "idref") or e.get("href").split("#", 1)[1]


def rules(c):
    return [e for e in c if e.tag in ("ownedRule", "ownedOperation")]


def bodies(e):
    return [n.get("body") for n in e.iter() if n.get("body") is not None and n.tag != "ownedComment"]


def dependencies(e):
    # This extracts *authority references*, never model declarations or semantics.
    return sorted(set(re.findall(r"'([^']+::[^']+)'", "\n".join(bodies(e)))))


def main():
    si, sn, sc = authority("SysML", "2.0")
    ki, kn, kc = authority("KerML", "1.0")
    assert len(sc) == 93 and len(kc) == 82
    classes = kc | sc
    parents = {i: [target(g.find("general")) for g in e.findall("generalization")] for i, e in classes.items()}
    libraries = json.loads((ROOT / "standards/normative/sysml-2.0/library-set.json").read_text())
    documents = {}
    metadata = []
    for artifact in libraries["artifacts"]:
        raw = (ROOT / artifact["path"]).read_bytes()
        assert digest(raw) == artifact["sha256"]
        with zipfile.ZipFile(ROOT / artifact["path"]) as archive:
            assert set(archive.namelist()) == {e["path"] for e in artifact["entries"]}
            for entry in artifact["entries"]:
                data = archive.read(entry["path"])
                assert digest(data) == entry["sha256"] and len(data) == entry["bytes"]
                if entry["path"].endswith((".kerml", ".sysml")):
                    documents[entry["path"]] = data.decode("utf-8")
                if entry["path"].endswith((".project.json", ".meta.json")):
                    metadata.append({"archive": artifact["source"], "entry": entry["path"], "content": json.loads(data)})
    # Read the actual supplied publication, not an unversioned internet grammar.
    import pypdf
    pdf_bytes = (ROOT / "SysML.pdf").read_bytes()
    pages = [page.extract_text() for page in pypdf.PdfReader(ROOT / "SysML.pdf").pages]
    clauses = {}
    for page_number, page in enumerate(pages, 1):
        for clause, name in re.findall(r"(?m)^(8\.3\.\d+(?:\.\d+)+)\s+([A-Z][A-Za-z]+)\s*$", page):
            clauses[name] = {"clause": clause, "pdf_page": page_number}
    rows = []
    total_rules = 0
    for identity, c in sorted(sc.items(), key=lambda pair: pair[1].get("name")):
        name = c.get("name")
        ancestors = set()
        pending = list(parents[identity])
        while pending:
            p = pending.pop()
            if p not in ancestors:
                ancestors.add(p)
                pending.extend(parents[p])
        retained = []
        for rule in rules(c):
            rule_name = rule.get("name")
            refs = dependencies(rule)
            category = ("standard-library-specialization" if refs else
                        "sysml-derived-behavior" if rule_name.startswith("derive") else
                        "validation-constraint" if rule_name.startswith(("validate", "check")) else
                        "not-yet-interpreted")
            retained.append({
                "source_id": rule.get(X + "id"), "name": rule_name,
                "kind": rule.tag, "category": category,
                "bodies": bodies(rule), "library_dependencies": refs,
                "implementation_status": "not-implemented", "query_id": None,
                "deferred_reason": "Awaiting project/library semantic prerequisites and normative rule implementation" if name in SLICE else "Outside initial systems structural semantic family; body retained for future interpretation",
            })
        total_rules += len(retained)
        inherited = [{"metaclass": classes[a].get("name"), "source_id": a,
                      "rules": [r.get(X + "id") for r in rules(classes[a])]}
                     for a in sorted(ancestors) if a in kc]
        all_refs = sorted(set(ref for a in ancestors | {identity} for r in rules(classes[a]) for ref in dependencies(r)))
        rows.append({
            "metaclass": name, "source_id": identity,
            "authority": {"artifact": si["source"], "sha256": si["sha256"], "specification": clauses.get(name)},
            "initial_slice": name in SLICE,
            "structural_behavior": {"status": "implemented", "owner": "agq-sysml", "scope": "Exact descriptors, borrowed views and generic kernel structural validation only"},
            "direct_supertypes": parents[identity],
            "inherited_kerml_behavior": inherited,
            "kerml_query_dependencies": ["owner", "memberships", "member"] + (["direct_specializations", "all_specializations", "direct_features", "effective_features"] if "Core-Types-Type" in ancestors else []) + (["direct_feature_types", "subsetted_features", "redefined_features"] if "Core-Features-Feature" in ancestors else []),
            "kerml_query_scope": "Existing bounded agq-kerml-query/5 contracts; inherited retained bodies are not all implemented",
            "rules": retained,
            "standard_library_dependencies": all_refs,
            "execution_semantics": {"status": "deferred", "reason": "No generation-2 execution runtime in this milestone; structural queries do not execute instances"},
            "uninterpreted_semantics": {"status": "explicit", "reason": "Unimplemented retained rules and additional specification prose are obligations; empty own-rule lists do not certify inherited meaning"},
        })
    missing = [row["metaclass"] for row in rows if row["authority"]["specification"] is None]
    assert not missing, f"Missing specification anchors: {missing}"
    out = {
        "format": "agentique-sysml-semantic-coverage/1", "generation": 2,
        "normative_targets": {"KerML": "1.0", "SysML": "2.0"},
        "status": "inventory-established-rules-not-implemented",
        "generator": "verification/sysml-semantic-foundation-v1/authority.py",
        "specification": {"path": "SysML.pdf", "sha256": digest(pdf_bytes)},
        "library_set": libraries["id"], "class_count": len(rows), "own_rule_operation_count": total_rules,
        "authority_scope": "Complete class and directly owned XMI rule/operation inventory with class-specific PDF anchors; not an exhaustive interpretation of specification prose",
        "metadata_identity_finding": "All four KPARs contain textual documents and name-to-file indexes; project/meta metadata supplies no semantic element UUIDs. A private scheme is required for immutable library elements.",
        "metadata": metadata,
        "source_discrepancies": [
            {"rule": "checkConnectionDefinitionBinarySpecialization", "source": "Connections::BinaryConnections", "library": "Systems Library/Connections.sysml", "finding": "Archive declares BinaryConnection (singular); unresolved source spelling must not create a replacement", "status": "requires-explicit-interpretation"},
            {"rule": "checkItemUsageSubitemSpecialization", "source": "Items::Item::subitem", "library": "Systems Library/Items.sysml", "finding": "Archive declares subitems (plural)", "status": "requires-explicit-interpretation"},
            {"rule": "deriveDefinitionOwnedInterface / deriveUsageNestedInterface", "finding": "Retained bodies select ReferenceUsage; preserve exact body and compare normative property types and prose before implementation", "status": "requires-explicit-interpretation"},
            {"source": "Systems Library/.meta.json", "finding": "AnalysisCases index points to absent AnalysisCase.sysml; archive contains AnalysisCases.sysml", "status": "retained-archive-diagnostic"},
        ],
        "metaclasses": rows,
    }
    encoded = (json.dumps(out, ensure_ascii=False, indent=2) + "\n").encode()
    if "--check" in sys.argv:
        assert OUT.read_bytes() == encoded, "Semantic authority inventory is stale"
    else:
        OUT.write_bytes(encoded)
    print(f"Verified {len(rows)} SysML classes, {total_rules} own rules/operations, {len(documents)} pinned library documents; no semantic implementation claim")


if __name__ == "__main__":
    main()
