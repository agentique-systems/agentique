"""Independent readers for pinned declarations, normative metadata and reference XMI.

No runtime query/producer implementation is imported. Historical declaration
exports are content-checked against pinned source bytes before reuse.
"""
import gzip
import hashlib
import json
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET
from functools import cache

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT/'verification/kerml-library-content-errata-publication-v5'))
from xmi import Model, XID

def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
inventory = json.loads((ROOT/'verification/kerml-semantic-closure-v6/metamodel-inventory.json').read_text())
normative_path=ROOT/inventory['file'].replace('\\','/')
assert digest(normative_path) == inventory['sha256']
normative = ET.parse(normative_path).getroot()
UID = '{http://www.omg.org/spec/XMI/20161101}id'
UTYPE = '{http://www.omg.org/spec/XMI/20161101}type'
classes = {e.get(UID):e for e in normative.iter() if e.get(UTYPE)=='uml:Class'}
names = {e.get('name'):e for e in classes.values()}
@cache
def conforms(name):
    seen, pending = set(), [names[name]] if name in names else []
    while pending:
        c = pending.pop()
        if c.get('name') in seen: continue
        seen.add(c.get('name'))
        pending.extend(classes[g.find('general').get('{http://www.omg.org/spec/XMI/20161101}idref')] for g in c if g.tag=='generalization')
    return seen

class Pinned:
    def __init__(self):
        self.archive=ROOT/'verification/kerml-semantic-closure-v8/declarations-v5.json.gz'
        self.export=json.loads(gzip.decompress(self.archive.read_bytes()))
        self.records=self.export['records']
        self.parents={target:identity for identity in self.records for prop in ['ownedRelationship','ownedRelatedElement'] for target in self.refs(identity,prop)}
        for path,sha in {(r['source']['document'],r['source']['sha256']) for r in self.records.values() if r.get('source')}:
            candidates=list((ROOT/'standards/libraries').rglob(Path(path).name))
            assert any(digest(p)==sha for p in candidates),(path,sha)
    def is_kind(self,e,c):return c in conforms(self.records[e]['metaclass'])
    def refs(self,e,p): return [v for s in self.records[e]['slots'].values() if s['name']==p for v in s['references']]
    def scalar(self,e,p):
        slots=[s for s in self.records[e]['slots'].values() if s['name']==p]
        if not slots:return None
        value=slots[0]['value']
        if value in ['Scalar(Boolean(true))','Scalar(Boolean(false))']:return value=='Scalar(Boolean(true))'
        if value.startswith('Scalar(String('):return json.loads(value[len('Scalar(String('):-2])
        return value
    def members(self,e):return [(m,v) for m in self.refs(e,'ownedRelationship') if self.is_kind(m,'OwningMembership') for v in self.refs(m,'ownedRelatedElement')]
    def owner(self,e):return self.parents.get(self.parents.get(e))
    def path(self,e):
        parts=[];seen=set()
        while e in self.records and e not in seen:
            seen.add(e)
            name=self.scalar(e,'declaredName')
            if name:parts.append(name)
            e=self.parents.get(e)
        return '::'.join(reversed(parts))
    def find(self,path):
        matches=[e for e in self.records if self.scalar(e,'declaredName') and self.path(e)==path]
        assert len(matches)==1,(path,len(matches))
        return matches[0]
    def brief(self,e):
        r=self.records[e]
        return dict(id=e,path=self.path(e),kind=r['metaclass'],source=r.get('source'))
    def select_cross(self,e,operational=True):
        membership=self.parents.get(e)
        if not self.scalar(e,'isEnd') or not membership or not self.is_kind(membership,'FeatureMembership'):return None
        owner=self.owner(e)
        if not owner or not self.is_kind(owner,'Type'):return None
        for m,v in self.members(e):
            if self.is_kind(v,'Feature') and not any(self.is_kind(v,c) for c in ['Multiplicity','MetadataFeature']+(['BindingConnector'] if operational else ['FeatureValue'])) and not self.is_kind(m,'FeatureMembership') and not (operational and self.is_kind(m,'FeatureValue')):return v
        return None

def reference():return Model(ROOT/'verification/kerml-semantic-closure-v6/release/sysml.library.xmi.implied')
def kind(model,e,c):return c in conforms(model.kind(e))
def members(model,e):return [(r,v) for r in e if kind(model,r,'OwningMembership') for v in r if v.tag=='ownedRelatedElement']
def related(model,e,cls,prop):return [v for r in e if kind(model,r,cls) for v in model.targets(r,prop)]
def brief(model,e):return dict(id=e.get(XID),path=model.path(e),kind=model.kind(e),file=model.files[e])
