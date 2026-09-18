"""Independent full-manifest verifier. Standard-library XML, no importer/IR helpers.

Run from the repository root. --check is read-only; without it write the evidence
summary. Source ranges come from Expat byte offsets, identities from Python uuid5.
"""
import hashlib
import json
from pathlib import Path
import sys
import uuid
import xml.etree.ElementTree as ET
from xml.parsers import expat

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
XMI = "{http://www.omg.org/spec/XMI/20161101}"
DOMAIN = uuid.UUID("a63be8e5-2305-4377-88be-764db0bd5da6")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_source(baseline):
    lock = json.loads((ROOT / f"standards/normative/{baseline}/lock.json").read_bytes())
    artifact = next(a for a in lock["artifacts"] if a["filename"].endswith(".xmi") and a["role"] == "primary")
    raw = (ROOT / artifact["path"]).read_bytes()
    assert digest(raw) == artifact["sha256"] and len(raw) == artifact["bytes"]
    tree = ET.fromstring(raw)
    parents = {c: p for p in tree.iter() for c in p}
    ids = {e.get(XMI + "id"): e for e in tree.iter() if e.get(XMI + "id")}
    assert len(ids) == sum(1 for e in tree.iter() if e.get(XMI + "id"))
    ranges = {}
    stack = []
    parser = expat.ParserCreate(namespace_separator="}")

    def start(_name, attrs):
        begin = parser.CurrentByteIndex
        end = raw.index(b">", begin) + 1
        stack.append((attrs.get(XMI[1:] + "id"), begin, end))

    def end(_name):
        key, begin, tag_end = stack.pop()
        stop = tag_end if raw[tag_end - 2:tag_end] == b"/>" else raw.index(b">", parser.CurrentByteIndex) + 1
        if key:
            ranges[key] = [begin, stop]

    parser.StartElementHandler = start
    parser.EndElementHandler = end
    parser.Parse(raw, True)
    source = dict(specification=lock["specification"], version=lock["version"],
                  metamodel_uri=lock["metamodel_uri"], artifact_uri=artifact["source"], sha256=artifact["sha256"])
    return dict(source=source, raw=raw, tree=tree, ids=ids, parents=parents, ranges=ranges)


def kind(element):
    return element.get(XMI + "type", "").split(":")[-1]


def ref(model, element):
    if element is None:
        return None
    return element.get("href") or model["source"]["artifact_uri"] + "#" + element.attrib[XMI + "idref"]


def refs(model, element, tag):
    return [ref(model, c) for c in element.findall(tag)]


def identity(model, element, identity_kind):
    chain = []
    parent = model["parents"].get(element)
    while parent is not None:
        if kind(parent) == "Package" or parent.tag.endswith("}Package"):
            chain.append(parent.attrib["name"])
        parent = model["parents"].get(parent)
    chain.reverse()
    source = model["source"]
    key = dict(source=source, package_path=chain, external_id=element.attrib[XMI + "id"], kind=identity_kind)
    encoded = json.dumps(["agentique-descriptor-key/1", source["specification"], source["version"],
                          source["metamodel_uri"], source["artifact_uri"], source["sha256"],
                          chain, key["external_id"], identity_kind], ensure_ascii=False, separators=(",", ":"))
    return key, str(uuid.uuid5(DOMAIN, encoded))


def check_entity(model, element, entity, descriptor, identity_kind):
    key, uid = identity(model, element, identity_kind)
    assert entity["key"] == key, key
    assert descriptor == uid, key
    assert entity["name"] == element.get("name", ""), key
    assert entity["source_range"] == model["ranges"][key["external_id"]], key
    assert entity["source_attributes"] == element.attrib, key


def boolean(element, name, default=False):
    value = element.get(name)
    assert value is None or value in ("true", "false")
    return default if value is None else value == "true"


def bound(element, tag):
    node = element.find(tag)
    return 1 if node is None else int(node.get("value", "0"))


def verify(manifest, models):
    source_classifiers = {}
    source_properties = {}
    for m in models:
        for local, e in m["ids"].items():
            qid = m["source"]["artifact_uri"] + "#" + local
            if kind(e) in ("Class", "Association", "Enumeration"):
                source_classifiers[qid] = (m, e)
            elif kind(e) == "Property":
                source_properties[qid] = (m, e)
    assert set(manifest["classifiers"]) == set(source_classifiers)
    assert set(manifest["properties"]) == set(source_properties)
    references = set()
    literal_count = 0
    for qid, (m, e) in source_classifiers.items():
        c = manifest["classifiers"][qid]
        check_entity(m, e, c["entity"], c["descriptor_id"], kind(e).lower())
        assert c["abstract"] == boolean(e, "isAbstract"), qid
        assert c["direct_supertypes"] == [ref(m, g.find("general")) for g in e.findall("generalization")], qid
        assert c["member_ends"] == refs(m, e, "memberEnd"), qid
        assert c["navigable_owned_ends"] == refs(m, e, "navigableOwnedEnd"), qid
        assert c["declared_properties"] == [m["source"]["artifact_uri"] + "#" + p.attrib[XMI + "id"] for p in e if p.tag in ("ownedAttribute", "ownedEnd")], qid
        references.update(c["direct_supertypes"] + c["member_ends"] + c["navigable_owned_ends"])
        literals = e.findall("ownedLiteral")
        assert len(literals) == len(c["literals"])
        for raw, generated in zip(literals, c["literals"]):
            check_entity(m, raw, generated["entity"], generated["descriptor_id"], "enumeration_literal")
            literal_count += 1
    domains = {}
    for qid, (m, e) in source_properties.items():
        entry = manifest["properties"][qid]
        p = entry["source"]
        check_entity(m, e, p["entity"], entry["descriptor_id"], "property")
        assert p["owner"] == m["source"]["artifact_uri"] + "#" + m["parents"][e].attrib[XMI + "id"], qid
        target = ref(m, e.find("type"))
        assert p["type_ref"] == dict(kind="local" if target in source_classifiers else "external", target=target), qid
        domains[target] = domains.get(target, 0) + 1
        assert p["lower"] == bound(e, "lowerValue"), qid
        upper = bound(e, "upperValue")
        assert p["upper"] == (dict(kind="unlimited") if upper == -1 else dict(kind="finite", value=upper)), qid
        for field, xml_name, default in [("is_ordered", "isOrdered", False), ("is_unique", "isUnique", True),
                ("is_derived", "isDerived", False), ("is_derived_union", "isDerivedUnion", False),
                ("is_read_only", "isReadOnly", False), ("is_id", "isID", False)]:
            assert p[field] == boolean(e, xml_name, default), (qid, field)
        assert p["aggregation"] == e.get("aggregation", "none"), qid
        for field, tag in [("redefines", "redefinedProperty"), ("subsets", "subsettedProperty")]:
            assert p[field] == refs(m, e, tag), (qid, field)
            references.update(p[field])
        association = ref(m, e.find("association"))
        assert p["association"] == association, qid
        ends = manifest["classifiers"][association]["member_ends"] if association else []
        assert p["opposite_ends"] == sorted(x for x in ends if x != qid), qid
        references.add(target)
    assert references <= set(source_classifiers) | set(source_properties) | set(manifest["external_types"])
    return dict(classifiers=len(source_classifiers), properties=len(source_properties), literals=literal_count,
                distinct_references=len(references), primitive_uses={k: v for k, v in domains.items() if k not in source_classifiers},
                checked="All direct structural facts, complete identity sets, UUID v1, hashes, attributes and byte ranges. Effective inheritance/rules/storage not certified.")


def main():
    kerml, sysml = read_source("kerml-1.0"), read_source("sysml-2.0")
    results = {}
    for baseline, models in [("kerml-1.0", [kerml]), ("sysml-2.0", [kerml, sysml])]:
        path = ROOT / f"standards/generated/{baseline}/full.golden.json"
        raw = path.read_bytes()
        results[baseline] = verify(json.loads(raw), models) | dict(manifest_sha256=digest(raw))
    rendered = (json.dumps(results, indent=2, sort_keys=True) + "\n").encode()
    path = OUT / "independent-verification.json"
    if "--check" in sys.argv:
        assert path.read_bytes() == rendered, "Independent verification evidence is stale"
    else:
        path.write_bytes(rendered)
    print(rendered.decode())


if __name__ == "__main__":
    main()
