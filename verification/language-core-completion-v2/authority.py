"""Offline Gate 0 evidence, independent of the Rust importer and kernel.

This checks source facts and distinguishes candidate context predicates. It is
not an implementation or acceptance test of a resolved Property override.
Run with --check to compare without writing. Acquiring evidence is never a build
step. UML.xmi is a research input, not a runtime metamodel dependency.
"""
import hashlib
import importlib.util
import json
from html.parser import HTMLParser
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "direct_xmi", ROOT / "verification/language-core-completion-v1/direct_xmi.py")
direct = importlib.util.module_from_spec(spec)
spec.loader.exec_module(direct)


class PageText(HTMLParser):
    def __init__(self):
        super().__init__()
        self.parts = []

    def handle_data(self, data):
        if data.strip():
            self.parts.append(data.strip())


def strict_ancestors(edges, start):
    """Identity-based traversal, with no recursion and no inferred edges."""
    visited = {start}
    pending = list(edges[start])
    while pending:
        node = pending.pop()
        if node not in visited:
            visited.add(node)
            pending.extend(edges[node])
    return sorted(visited - {start})


def build():
    acquisitions = json.loads((OUT / "evidence/acquisition.json").read_bytes())
    issues = []
    for key, legacy in [("UML24-85", 15525), ("UML24-86", 15526), ("UML24-96", 15567)]:
        raw = (OUT / f"evidence/{key}.html").read_bytes()
        acquisition = next(a for a in acquisitions if a["id"] == key)
        assert hashlib.sha256(raw).hexdigest() == acquisition["sha256"]
        assert len(raw) == acquisition["bytes"]
        page = PageText()
        page.feed(raw.decode("utf8"))
        text = " ".join(page.parts)
        assert f"Legacy Issue Number: {legacy}" in text
        assert "Status: closed" in text
        assert "Disposition: Resolved" in text
        resolution = text.split("Disposition Summary:", 1)[1].split("Updated:", 1)[0].strip()
        issues.append(dict(key=key, legacy=legacy, status="closed", disposition="Resolved",
                           source=acquisition, resolution=resolution))

    models = [direct.read_source(b) for b in ["kerml-1.0", "sysml-2.0"]]
    entities = {m["source"]["artifact_uri"] + "#" + key: (m, e)
                for m in models for key, e in m["ids"].items()}
    edges = {qid: [direct.ref(m, g.find("general")) for g in e.findall("generalization")]
             for qid, (m, e) in entities.items() if direct.kind(e) in ("Class", "Association")}

    def fact(qid):
        m, e = entities[qid]
        local = e.attrib[direct.XMI + "id"]
        begin, end = m["ranges"][local]
        key, uid = direct.identity(m, e, direct.kind(e).lower())
        result = dict(source_qualified_id=qid, descriptor_id=uid, key=key,
                      byte_range=[begin, end], lines=[m["raw"][:begin].count(b"\n") + 1,
                                                    m["raw"][:end].count(b"\n") + 1],
                      raw_xmi=m["raw"][begin:end].decode())
        if direct.kind(e) == "Property":
            association = direct.ref(m, e.find("association"))
            am, ae = entities[association]
            opposite = [p for p in direct.refs(am, ae, "memberEnd") if p != qid]
            assert len(opposite) == 1
            om, oe = entities[opposite[0]]
            result.update(owner=m["source"]["artifact_uri"] + "#" + m["parents"][e].attrib[direct.XMI + "id"],
                          owner_kind=direct.kind(m["parents"][e]), association=association,
                          opposite=opposite[0], opposite_type=direct.ref(om, oe.find("type")),
                          value_type=direct.ref(m, e.find("type")),
                          redefines=direct.refs(m, e, "redefinedProperty"),
                          subsets=direct.refs(m, e, "subsettedProperty"),
                          lower=direct.bound(e, "lowerValue"), upper=direct.bound(e, "upperValue"))
        else:
            result.update(direct_supertypes=edges[qid], ancestors=strict_ancestors(edges, qid))
        return result

    s = models[1]["source"]["artifact_uri"] + "#"
    k = models[0]["source"]["artifact_uri"] + "#"
    p = fact(s + "Systems-Flows-A_flowDefinition_definedFlow-definedFlow")
    b = fact(s + "Systems-DefinitionAndUsage-Definition-ownedAction")
    assert p["redefines"] == [b["source_qualified_id"]]
    assert p["owner_kind"] == "Association" and b["owner_kind"] == "Class"
    checks = dict(
        owning_association_has_parents=bool(edges[p["owner"]]),
        base_association_is_ancestor=b["association"] in strict_ancestors(edges, p["association"]),
        opposite_context_is_strict_subclass=p["opposite_type"] != b["owner"] and
            b["owner"] in strict_ancestors(edges, p["opposite_type"]),
        value_type_conforms=p["value_type"] == b["value_type"] or
            b["value_type"] in strict_ancestors(edges, p["value_type"]),
        flow_definition_inherits_both_contexts=all(
            c in strict_ancestors(edges, s + "Systems-Flows-FlowDefinition")
            for c in [p["opposite_type"], b["owner"]]),
    )
    assert checks == dict(owning_association_has_parents=False,
                         base_association_is_ancestor=False,
                         opposite_context_is_strict_subclass=False,
                         value_type_conforms=True,
                         flow_definition_inherits_both_contexts=True)
    # Evaluate the final inherited-property condition directly: an empty parent
    # set supplies no inherited features. No general evaluator is claimed.
    assert strict_ancestors(edges, p["owner"]) == []
    checks["literal_final_redefined_property_inherited"] = False
    checks["ownership_independent_opposite_context_candidate"] = False
    checks["association_ancestry_candidate"] = False
    checks["either_ancestry_candidate"] = False

    uml_path = ROOT / ".cache/sysml-resume/UML.xmi"
    uml_raw = uml_path.read_bytes()
    assert direct.digest(uml_raw) == "e8166c91f51b8a0c015a90101b83d2249d03252e9f9a500a6511654347804f21"
    ux = "{http://www.omg.org/spec/XMI/20131001}"
    uml = ET.fromstring(uml_raw)
    ids = {e.get(ux + "id"): e for e in uml.iter() if e.get(ux + "id")}
    operations = [e.get("name") for e in ids["Property"].findall("ownedOperation")]
    assert "isRedefinitionContextValid" not in operations
    subsets = [e.get(ux + "idref") for e in ids["Property-owningAssociation"].findall("subsettedProperty")]
    assert "RedefinableElement-redefinitionContext" in subsets
    rules = {}
    for key in ["Property-redefined_property_inherited", "Property-subsettingContext",
                "Property-subsetting_context_conforms", "Property-isConsistentWith",
                "Property-type_of_opposite_end", "RedefinableElement-isRedefinitionContextValid",
                "RedefinableElement-redefinition_context_valid"]:
        at = uml_raw.index(('xmi:id="' + key + '"').encode())
        rules[key] = dict(line=uml_raw[:at].count(b"\n") + 1,
                          bodies=[e.text for e in ids[key].iter("body")])

    # UML24-96 gives 21 concrete cases. These two samples corroborate that its
    # association generalizations survived into the final normative XMI.
    samples = {}
    for a, parent in [("A_specification_timeConstraint", "A_specification_intervalConstraint"),
                      ("A_min_timeInterval", "A_min_interval")]:
        parents = [g.get("general") for g in ids[a].findall("generalization")]
        assert parent in parents
        samples[a] = parents

    from pypdf import PdfReader
    pdfs = []
    for name, pages in [(".cache/sysml-resume/UML.pdf", [153, 154, 155, 191, 192, 193, 194, 195, 196]),
                        ("SysML.pdf", [336, 337, 338])]:
        path = ROOT / name
        reader = PdfReader(path)
        page_hashes = {str(n): direct.digest(reader.pages[n - 1].extract_text().encode()) for n in pages}
        pdfs.append(dict(path=name, sha256=direct.digest(path.read_bytes()), pages=pages,
                         extracted_page_sha256=page_hashes))
    assert pdfs[0]["sha256"] == "416b57e1933780eb48bd60fe513e031da220c28a521bdd334a366bebc78a463e"

    # Every ownership/inheritance arrangement is exposed, not promoted to a
    # guessed universal validity rule. C and A are independent graph facts.
    matrix = []
    for redefining in ["class", "association"]:
        for redefined in ["class", "association"]:
            for classifier_inheritance in [False, True]:
                for association_inheritance in [False, True]:
                    matrix.append(dict(redefining_owner=redefining, redefined_owner=redefined,
                                       strict_context_inheritance=classifier_inheritance,
                                       strict_association_inheritance=association_inheritance,
                                       endpoint_candidate=classifier_inheritance,
                                       association_candidate=association_inheritance,
                                       either_candidate=classifier_inheritance or association_inheritance,
                                       accepted_runtime_rule=False))
    inventory = {}
    for baseline in ["kerml-1.0", "sysml-2.0"]:
        raw = (ROOT / f"standards/generated/{baseline}/full-audit.json").read_bytes()
        audit = json.loads(raw)
        inventory[baseline] = dict(sha256=direct.digest(raw), counts=audit["counts"],
            association_arities=audit["association_arities"], findings=audit["findings"],
            translation_error=audit["translation_error"], registration_attempted=audit["registration_attempted"])

    facts = [p, b, fact(p["opposite"]), fact(b["opposite"]),
             fact(p["association"]), fact(b["association"])]
    classes = []
    for qid in [p["opposite_type"], b["owner"], p["value_type"], b["value_type"],
                s + "Systems-Flows-FlowDefinition", k + "Kernel-Interactions-Interaction"]:
        if qid not in [c["source_qualified_id"] for c in classes]:
            c = fact(qid)
            del c["raw_xmi"]
            classes.append(c)
    return dict(format="agentique-property-authority-review/2", classification="F",
                runtime_rule_accepted=False, issues=issues, properties_and_associations=facts,
                classes=classes, acceptance_case=checks, ownership_candidate_matrix=matrix,
                uml=dict(source="https://www.omg.org/spec/UML/20161101/UML.xmi",
                         sha256=direct.digest(uml_raw), property_declared_operations=operations,
                         owning_association_subsets=subsets, rules=rules,
                         final_association_generalization_samples=samples),
                pdfs=pdfs, complete_audits=inventory)


def main():
    result = build()
    rendered = (json.dumps(result, indent=2, ensure_ascii=False) + "\n").encode("utf8")
    path = OUT / "evidence/property-authority.json"
    if "--check" in sys.argv:
        assert path.read_bytes() == rendered, "Gate 0 authority evidence is stale"
    else:
        if path.exists():
            raise RuntimeError("Evidence exists; do not overwrite an earlier investigation")
        path.write_bytes(rendered)
    print(json.dumps(dict(classification=result["classification"],
                          runtime_rule_accepted=False, acquired_resolved_issues=3,
                          candidate_matrix_rows=len(result["ownership_candidate_matrix"]),
                          acceptance_case=result["acceptance_case"]), indent=2))


if __name__ == "__main__":
    main()
