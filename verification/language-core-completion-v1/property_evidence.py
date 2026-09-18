"""Reproduce Gate 0 facts from pinned XMI and locally acquired UML evidence.

Research only. No network, runtime output, source normalization or kernel waiver.
"""
import json
from pathlib import Path
import xml.etree.ElementTree as ET
from direct_xmi import ROOT, OUT, XMI, read_source, identity, kind, ref, refs, digest, bound, boolean

models = [read_source("kerml-1.0"), read_source("sysml-2.0")]
entities = {m["source"]["artifact_uri"] + "#" + key: (m, e) for m in models for key, e in m["ids"].items()}


def ancestry(qid):
    seen, pending = set(), [qid]
    while pending:
        item = pending.pop()
        if item not in seen:
            seen.add(item)
            m, e = entities[item]
            pending.extend(ref(m, g.find("general")) for g in e.findall("generalization"))
    return sorted(seen - {qid})


def facts(qid):
    m, e = entities[qid]
    local = e.attrib[XMI + "id"]
    span = m["ranges"][local]
    parent = m["parents"][e]
    key, uid = identity(m, e, kind(e).lower())
    record = dict(source_qualified_id=qid, key=key, descriptor_id=uid, byte_range=span,
                  lines=[m["raw"][:span[0]].count(b"\n") + 1, m["raw"][:span[1]].count(b"\n") + 1],
                  attributes=e.attrib, owner=m["source"]["artifact_uri"] + "#" + parent.attrib[XMI + "id"],
                  owner_kind=kind(parent), raw_xmi=m["raw"][span[0]:span[1]].decode())
    if kind(e) == "Property":
        assoc = ref(m, e.find("association"))
        am, ae = entities[assoc]
        opposite = [x for x in refs(am, ae, "memberEnd") if x != qid]
        context = [ref(entities[x][0], entities[x][1].find("type")) for x in opposite]
        record.update(type=ref(m, e.find("type")), lower=bound(e, "lowerValue"), upper=bound(e, "upperValue"),
                      derived=boolean(e, "isDerived"), ordered=boolean(e, "isOrdered"), unique=boolean(e, "isUnique", True),
                      aggregation=e.get("aggregation", "none"), association=assoc, opposite_ends=opposite,
                      redefines=refs(m, e, "redefinedProperty"), subsets=refs(m, e, "subsettedProperty"),
                      property_contexts=context, context_ancestors={c: ancestry(c) for c in context},
                      is_leaf_explicit=e.get("isLeaf"), is_leaf_uml_default=False)
    else:
        record.update(direct_supertypes=[ref(m, g.find("general")) for g in e.findall("generalization")], ancestors=ancestry(qid))
    return record


sysml = models[1]["source"]["artifact_uri"] + "#"
kerml = models[0]["source"]["artifact_uri"] + "#"
names = [
    "Systems-Flows-A_flowDefinition_definedFlow-definedFlow",
    "Systems-DefinitionAndUsage-Definition-ownedAction",
    "Systems-Flows-FlowUsage-flowDefinition",
    "Systems-DefinitionAndUsage-A_ownedAction_actionOwningDefinition-actionOwningDefinition",
    "Systems-Flows-A_flowDefinition_definedFlow",
    "Systems-DefinitionAndUsage-A_ownedAction_actionOwningDefinition",
]
class_names = [sysml + x for x in ["Systems-Flows-FlowUsage", "Systems-Flows-FlowDefinition",
               "Systems-Actions-ActionUsage", "Systems-DefinitionAndUsage-Definition"]] + [kerml + "Kernel-Interactions-Interaction"]
classes = []
rules = []
for qid in class_names:
    record = facts(qid)
    # Whole classes contain unrelated rules. Retain their source locations and
    # inheritance; only rules mentioning the investigated properties are copied.
    del record["raw_xmi"]
    classes.append(record)
    m, e = entities[qid]
    for node in e:
        text = ET.tostring(node, encoding="unicode")
        if node.tag in ("ownedRule", "ownedOperation") and any(s in text for s in ["flowDefinition", "definedFlow", "ownedAction"]):
            local = node.attrib[XMI + "id"]
            start, end = m["ranges"][local]
            rules.append(dict(source=m["source"], external_id=local, byte_range=[start, end], raw_xmi=m["raw"][start:end].decode()))

uml_path = ROOT / ".cache/sysml-resume/UML.xmi"
uml_raw = uml_path.read_bytes()
assert digest(uml_raw) == "e8166c91f51b8a0c015a90101b83d2249d03252e9f9a500a6511654347804f21"
uml_tree = ET.fromstring(uml_raw)
ux = "{http://www.omg.org/spec/XMI/20131001}"
uml_ids = {e.get(ux + "id"): e for e in uml_tree.iter() if e.get(ux + "id")}
operation_names = [e.get("name") for e in uml_ids["Property"].findall("ownedOperation")]
assert "isRedefinitionContextValid" not in operation_names
rules_and_operations = {}
for owner in ["Property", "RedefinableElement", "MultiplicityElement"]:
    for e in uml_ids[owner]:
        if e.tag in ("ownedRule", "ownedOperation"):
            local = e.attrib[ux + "id"]
            at = uml_raw.index(('xmi:id="' + local + '"').encode())
            rules_and_operations[local] = dict(line=uml_raw[:at].count(b"\n") + 1,
                                               bodies=[n.text for n in e.iter("body")],
                                               body_ids=[n.get(ux + "id") for n in e.iter() if n.tag in ("specification", "bodyCondition", "precondition")])
pdfs = []
for path, pages in [(ROOT / "SysML.pdf", [336, 337]), (ROOT / ".cache/sysml-resume/UML.pdf", [154, 191, 192, 193, 194, 195, 196])]:
    from pypdf import PdfReader
    reader = PdfReader(path)
    pdfs.append(dict(path=str(path.relative_to(ROOT)).replace("\\", "/"), sha256=digest(path.read_bytes()),
                     bytes=path.stat().st_size, inspected_pdf_pages=pages,
                     extracted_page_hashes={str(p): digest(reader.pages[p - 1].extract_text().encode()) for p in pages}))
issue_path = ROOT / ".cache/language-core-completion-v1/UML24-85.html"
result = dict(format="agentique-property-context-investigation/1", classification="F",
              properties_and_associations=[facts(sysml + x) for x in names], classes=classes, retained_rules=rules,
              uml=dict(source="https://www.omg.org/spec/UML/20161101/UML.xmi", sha256=digest(uml_raw),
                       property_declared_operations=operation_names, property_context_override_present=False,
                       rules_and_operations=rules_and_operations), pdfs=pdfs,
              issue=dict(source="https://issues.omg.org/issues/UML24-85", sha256=digest(issue_path.read_bytes()),
                         bytes=issue_path.stat().st_size, status="resolved", legacy_id=15525,
                         interpretation="Resolution identifies mixed ownership and inheritance. The final Property operation list has no proposed context override.",
                         unavailable_revised_text=dict(url="https://www.omg.org/issues/issue15525.txt", http_status=403)),
              decision="ADR 0009. No inspected rule establishes the exact replacement; do not infer an association ancestor or rewrite the Interaction type.")
(OUT / "stage-0/property-evidence.json").write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps(dict(properties_and_associations=len(names), classes=len(classes), retained_rules=len(rules),
                     property_declared_operations=operation_names, output="stage-0/property-evidence.json"), indent=2))
