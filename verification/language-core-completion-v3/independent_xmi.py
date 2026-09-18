"""Second implementation: direct XML + Python UUIDv5, then emitted Rust records.

Reuses the unchanged v1 independent source checker, never generator/IR helpers.
--check is read-only. The Rust package tests additionally exercise compiled records.
"""
import importlib.util
import json
import re
import sys
import uuid
from pathlib import Path
import xml.etree.ElementTree as ET
from xml.parsers import expat

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("independent_v1", HERE.parent / "language-core-completion-v1/direct_xmi.py")
old = importlib.util.module_from_spec(spec)
spec.loader.exec_module(old)


def primitives():
    lock = json.loads((ROOT / "standards/normative/kerml-1.0/lock.json").read_bytes())
    artifact = next(a for a in lock["artifacts"] if a["filename"] == "PrimitiveTypes.xmi")
    raw = (ROOT / artifact["path"]).read_bytes()
    assert old.digest(raw) == artifact["sha256"]
    tree = ET.fromstring(raw)
    for e in tree.iter():
        e.attrib = {k.replace("http://www.omg.org/spec/XMI/20131001", "http://www.omg.org/spec/XMI/20161101"):v for k,v in e.attrib.items()}
    parents = {c:p for p in tree.iter() for c in p}
    ids = {e.get(old.XMI+"id"):e for e in tree.iter() if e.get(old.XMI+"id")}
    ranges, stack = {}, []
    parser = expat.ParserCreate(namespace_separator="}")
    def start(_name, attrs):
        begin=parser.CurrentByteIndex
        stack.append((attrs.get("http://www.omg.org/spec/XMI/20131001}id"),begin,raw.index(b">",begin)+1))
    def end(_name):
        key,begin,tag_end=stack.pop()
        if key:
            ranges[key]=[begin,tag_end if raw[tag_end-2:tag_end]==b"/>" else raw.index(b">",parser.CurrentByteIndex)+1]
    parser.StartElementHandler=start
    parser.EndElementHandler=end
    parser.Parse(raw,True)
    source=dict(specification="UML",version="2.5.1",metamodel_uri="http://www.omg.org/spec/PrimitiveTypes/20161101",artifact_uri=artifact["source"],sha256=artifact["sha256"])
    return dict(source=source,tree=tree,parents=parents,ids=ids,ranges=ranges)


def ids(text):
    return [str(uuid.UUID(hex=x)) for x in re.findall(r"from_u128\(0x([0-9a-f]{32})\)",text)]


def field(line,name):
    # Descriptor fields have no nested braces except multiplicity. End on the
    # next top-level comma, accounting for brackets, strings and parentheses.
    begin=line.index(name+": ")+len(name)+2
    stack=[]
    quote=False
    escape=False
    for i in range(begin,len(line)):
        c=line[i]
        if quote:
            if escape: escape=False
            elif c=="\\": escape=True
            elif c=='"': quote=False
        elif c=='"': quote=True
        elif c in "([{": stack.append(c)
        elif c in ")]}":
            if not stack: return line[begin:i].strip()
            stack.pop()
        elif c=="," and not stack: return line[begin:i].strip()
    raise AssertionError((name,line))


def verify_runtime(models, primitive):
    all_models=models+[primitive]
    entities={}
    for m in all_models:
        for local,e in m["ids"].items():
            k=old.kind(e)
            if k in ("Class","Association","Enumeration","Property","EnumerationLiteral","PrimitiveType") or e.tag.endswith("}Package"):
                kind="package" if e.tag.endswith("}Package") else re.sub(r"(?<!^)(?=[A-Z])","_",k).lower()
                key,uid=old.identity(m,e,kind)
                if kind=="package":
                    key["package_path"].append(e.get("name"))
                    source=m["source"]
                    encoded=json.dumps(["agentique-descriptor-key/1",source["specification"],source["version"],source["metamodel_uri"],source["artifact_uri"],source["sha256"],key["package_path"],key["external_id"],kind],separators=(",",":"))
                    uid=str(uuid.uuid5(old.DOMAIN,encoded))
                entities[m["source"]["artifact_uri"]+"#"+local]=(m,e,key,uid)
    def resolve(m,e):
        q=old.ref(m,e)
        return entities[q][3] if q else None
    def targets(m,e,tag): return sorted(resolve(m,x) for x in e.findall(tag))
    seen=set()
    summaries={}
    for language,m in zip(("kerml","sysml"),models):
        path=ROOT/f"crates/{language}/src/generated/complete.rs"
        raw=path.read_bytes()
        lines=raw.decode().splitlines()
        records={ids(line)[0]:line for line in lines if re.match(r"\s*output\.(classes|properties|associations|enumerations|primitives|models)\.push\(",line)}
        sources={ids(line)[0]:line for line in lines if "output.sources.insert(" in line}
        expected={uid for mm,e,key,uid in entities.values() if mm is m and old.kind(e) in ("Class","Property","Association","Enumeration") or mm is m and e.tag.endswith("}Package")}
        if language=="kerml":
            expected.update(uid for mm,e,key,uid in entities.values() if mm is primitive and e.get("name") in ("Boolean","String","Integer","Real"))
        assert set(records)==expected,(language,len(records),len(expected),set(records)^expected)
        assert not seen.intersection(records),"dependency descriptors cloned"
        seen.update(records)
        for mm,e,key,uid in entities.values():
            if uid not in records: continue
            line=records[uid]
            assert json.loads(field(line,"name").removesuffix(".into()"))==e.get("name","")
            src=sources[uid]
            for name in ("specification","version","artifact_uri","sha256"):
                assert json.loads(field(src,name).removesuffix(".into()"))==mm["source"][name]
            assert json.loads(field(src,"external_id").removesuffix(".into()"))==e.get(old.XMI+"id")
            assert json.loads(field(src,"byte_range"))==mm["ranges"][e.get(old.XMI+"id")]
            k=old.kind(e)
            if k in ("Class","Association"):
                assert ids(field(line,"direct_supertypes"))==sorted(resolve(mm,g.find("general")) for g in e.findall("generalization"))
                assert field(line,"is_abstract")==str(old.boolean(e,"isAbstract")).lower()
            if k=="Association":
                assert ids(field(line,"member_ends"))==[resolve(mm,x) for x in e.findall("memberEnd")]
                assert ids(field(line,"navigable_owned_ends"))==targets(mm,e,"navigableOwnedEnd")
            if k=="Property":
                parent=mm["parents"][e]
                parent_uid=entities[mm["source"]["artifact_uri"]+"#"+parent.get(old.XMI+"id")][3]
                owner_kind="Class" if old.kind(parent)=="Class" else "Association"
                assert field(line,"owner").startswith("PropertyOwner::"+owner_kind)
                assert ids(field(line,"owner"))==[parent_uid]
                target=old.ref(mm,e.find("type")); tm,te,_,tid=entities[target]
                variant={"Class":"Reference","Enumeration":"Enumeration","PrimitiveType":"Primitive"}[old.kind(te)]
                assert field(line,"value_kind").startswith("ValueKind::"+variant)
                assert ids(field(line,"value_kind"))==[tid]
                assert field(field(line,"multiplicity"),"lower")==str(old.bound(e,"lowerValue"))
                upper=old.bound(e,"upperValue")
                assert field(field(line,"multiplicity"),"upper")==("None" if upper==-1 else f"Some({upper})")
                for name,attr,default in [("ordered","isOrdered",False),("unique","isUnique",True),("derived","isDerived",False),("derived_union","isDerivedUnion",False)]:
                    assert field(line,name)==str(old.boolean(e,attr,default)).lower()
                assert field(line,"composite")==str(e.get("aggregation")=="composite").lower()
                for name,tag in [("redefines","redefinedProperty"),("subsets","subsettedProperty")]:
                    assert ids(field(line,name))==targets(mm,e,tag)
                association=old.ref(mm,e.find("association"))
                assert ids(field(line,"association"))==([entities[association][3]] if association else [])
                opposites=[]
                if association:
                    am,ae,_,_=entities[association]
                    opposites=sorted(resolve(am,x) for x in ae.findall("memberEnd") if resolve(am,x)!=uid)
                assert ids(field(line,"opposite_ends"))==opposites
            if k=="PrimitiveType": assert field(line,"representation")=="PrimitiveRepresentation::"+e.get("name")
            if k=="Enumeration":
                literals=[old.identity(mm,x,"enumeration_literal")[1] for x in e.findall("ownedLiteral")]
                assert ids(field(line,"literals"))==sorted(literals)
                for x in e.findall("ownedLiteral"):
                    lid=old.identity(mm,x,"enumeration_literal")[1]
                    assert lid in sources
        views=ROOT/f"crates/{language}/src/generated/typed_views.rs"
        names=set(re.findall(r"define_view!\((\w+),",views.read_text()))
        assert names=={e.get("name") for e in m["ids"].values() if old.kind(e)=="Class"}
        summaries[language]=dict(descriptors=len(records),sources=len(sources),views=len(names),descriptor_sha256=old.digest(raw),views_sha256=old.digest(views.read_bytes()))
    return summaries


def main():
    models=[old.read_source("kerml-1.0"),old.read_source("sysml-2.0")]
    result={}
    for name,selected in [("kerml-1.0",models[:1]),("sysml-2.0",models)]:
        result[name]=old.verify(json.loads((ROOT/f"standards/generated/{name}/full.golden.json").read_bytes()),selected)
    result["emitted_runtime"]=verify_runtime(models,primitives())
    rendered=(json.dumps(result,sort_keys=True,indent=2)+"\n").encode()
    target=HERE/"independent-verification.json"
    if "--check" in sys.argv: assert target.read_bytes()==rendered,"stale independent evidence"
    else: target.write_bytes(rendered)
    print(rendered.decode())

if __name__=="__main__": main()
