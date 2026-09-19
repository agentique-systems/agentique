"""Independent exported-fact diff. Does not import the Rust transform or recipe.

Build expected facts from the review in a second language, then compare every
record, property and provenance assertion, including all untouched input facts.
"""
import copy
import hashlib
import json
from pathlib import Path
import re
import uuid
from evidence_io import read_json

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
v2 = read_json(OUT/'declarations-3/model.json')
before = read_json(OUT/'published-declarations/model.json')
assert before['profile']=='omg-kerml-1.0-published/1'
assert before['records']==v2['records']
assert before['occurrences']==v2['occurrences']
after = read_json(OUT/'operational-declarations/model.json')
manifest_path = ROOT/'standards/kerml-1.0-operational-library-errata-v3.json'
review = json.loads(manifest_path.read_text())
assert before['library_set'] == after['library_set'] == review['library_set']
assert after['profile'] == review['profile_id']
assert before['occurrences'] == after['occurrences']
expected = copy.deepcopy(before['records'])
outputs, libraries, provenance = {}, {}, {}
element_id_property = next(p for r in expected.values() for p,s in r['slots'].items() if s['name']=='elementId')

def origin(entry, key):
    return 'Declared(ReviewedCorrection { profile: '+json.dumps(review['profile_id'])+', entry: '+json.dumps(entry['id'])

def encode(value):
    kind, v = next(iter(value.items()))
    if kind == 'reference':
        identifier = resolve(v)
        return 'Scalar(Reference(ElementId('+str(uuid.UUID(identifier).int)+')))', [identifier]
    if kind == 'boolean': return 'Scalar(Boolean('+str(v).lower()+'))', []
    if kind == 'string': return 'Scalar(String('+json.dumps(v,ensure_ascii=False)+'))', []
    if kind == 'integer': return 'Scalar(Integer(Integer('+v+')))', []
    if kind == 'enumeration': return 'Scalar(Enumeration(EnumerationLiteralId('+str(uuid.UUID(v).int)+')))', []
    raise AssertionError(kind)

for key,s in review['selectors'].items():
    r = expected[s['id']]
    assert r['metaclass_id']==s['metaclass']
    assert {k:r['source'][k] for k in ['document','sha256','range']} == {k:s[k] for k in ['document','sha256','range']}

for entry in review['entries']:
    sample = next(r for r in expected.values() if r['source']['document']==entry['document'])
    library_int = int(re.search(r'LibraryId\((\d+)\)',sample['origin'])[1])
    libraries[entry['id']] = str(uuid.UUID(int=library_int))
    for op in entry['operations']:
        if op['operation']!='create':continue
        encoded=json.dumps(['agentique-operational-library-output/1',review['profile_id'],review['library_set'],libraries[entry['id']],entry['id'],op['key']],separators=(',',':'))
        identifier=str(uuid.uuid5(uuid.UUID('802a9394-27b1-54e9-b496-e1d88bd98722'),encoded))
        assert identifier not in expected
        outputs[entry['id'],op['key']]=identifier
        expected[identifier]=dict(metaclass_id=op['metaclass'],slots={},source=None,origin=origin(entry,op['key']))
        expected[identifier]['slots'][element_id_property]=dict(value='Scalar(String('+json.dumps(identifier)+'))',references=[],origin=origin(entry,op['key']))
        provenance[identifier,None]=(entry,op['key'])
        provenance[identifier,element_id_property]=(entry,op['key'])

for entry in review['entries']:
    def resolve(target):
        if 'pinned' in target:return review['selectors'][target['pinned']]['id']
        return outputs[entry['id'],target['output']]
    for op in entry['operations']:
        operation=op['operation']
        if operation=='create':continue
        identifier=resolve(op['element'])
        r=expected[identifier]
        if operation=='reclassify':
            r['metaclass_id']=op['metaclass'];r['origin']=origin(entry,op['key'])
            provenance[identifier,None]=(entry,op['key']);continue
        prop=op['property']
        if operation=='set':value,refs=encode(op['value'])
        elif operation=='append':
            refs=r['slots'].get(prop,{}).get('references',[])+[resolve(op['target'])]
            value='Ordered(['+', '.join('Reference(ElementId('+str(uuid.UUID(i).int)+'))' for i in refs)+'])'
        else:raise AssertionError(operation)
        r['slots'][prop]=dict(value=value,references=refs,origin=origin(entry,op['key']))
        provenance[identifier,prop]=(entry,op['key'])

assert expected.keys()==after['records'].keys(), 'Unexpected added/removed identities'
diff=[]
for identifier,wanted in expected.items():
    actual=after['records'][identifier]
    assert wanted['metaclass_id']==actual['metaclass_id'],identifier
    assert wanted['source']==actual['source'],identifier
    assert wanted['slots'].keys()==actual['slots'].keys(),identifier
    for prop in [None,*wanted['slots']]:
        a=actual if prop is None else actual['slots'][prop]
        w=wanted if prop is None else wanted['slots'][prop]
        if prop is not None:
            assert a['value']==w['value'],(identifier,prop,a['value'],w['value'])
            assert a['references']==w['references']
        if (identifier,prop) in provenance:
            entry,key=provenance[identifier,prop]
            assert a['origin'].startswith(w['origin']), (identifier,prop)
            assert 'output_key: '+json.dumps(key)+' }' in a['origin']
            assert 'library: LibraryId('+str(uuid.UUID(libraries[entry['id']]).int)+')' in a['origin']
            assert all(uri in a['origin'] for uri in entry['authority'])
            assert 'source_key: "'+entry['document']+'#sha256:' in a['origin']
            old=before['records'].get(identifier,{})
            old_fact=old if prop is None else old.get('slots',{}).get(prop)
            diff.append(dict(element=identifier,property=prop,metaclass=actual['metaclass'],
                before=old_fact if prop is not None else {k:old.get(k) for k in ['metaclass_id','origin']},
                after=a if prop is not None else {k:a[k] for k in ['metaclass_id','origin']},
                correction=entry['id'],operation=key))
        else:assert a['origin']==w['origin'],(identifier,prop)

report=dict(format='agentique-independent-operational-model-diff/1',review_sha256=hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
    before_profile=before['profile'],after_profile=after['profile'],published_and_v2_parsed_source_facts_identical=True,
    before_records=len(before['records']),after_records=len(after['records']),new_records=len(outputs),
    removed_records=0,unreviewed_changes=0,changes=diff,
    dimensions=['element','membership','specialization','redefinition','typing','ownership','provenance'])
(OUT/'operational-patch-diff.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='changes'}))
