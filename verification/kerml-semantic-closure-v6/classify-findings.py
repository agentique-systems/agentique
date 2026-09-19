"""Individual remaining findings, with source/canonical facts and reference evidence."""
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import sys
sys.dont_write_bytecode=True
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
OLD=ROOT/'verification/kerml-library-content-errata-publication-v5'
sys.path.insert(0,str(OLD))
from xmi import Model
archive=next(r for r in json.loads((OLD/'archived-exports.json').read_text())
             if r['logical_path']=='operational-declarations/model.json')
data=(OLD/archive['archive']).read_bytes()
assert hashlib.sha256(data).hexdigest()==archive['archive_sha256']
raw=gzip.decompress(data)
assert hashlib.sha256(raw).hexdigest()==archive['sha256']
records=json.loads(raw)['records']
parents={}
def slot(element,name):
    return next((s for s in records[element]['slots'].values() if s['name']==name),None)
for element,record in records.items():
    for s in record['slots'].values():
        if s['name'] in ['ownedRelationship','ownedRelatedElement']:
            for target in s['references']:parents[target]=element
def path(element):
    result=[]
    while element is not None:
        value=slot(element,'declaredName')
        if value:
            encoded=value['value']
            assert encoded.startswith('Scalar(String(') and encoded.endswith('))')
            name=json.loads(encoded[len('Scalar(String('):-2])
            if name:result.append(name)
        element=parents.get(element)
    return '::'.join(reversed(result))
reference=Model(OUT/'release/sysml.library.xmi.implied')
quality=json.loads((OUT/'quality-comparison.json').read_text())
rows=[]
for finding in quality['current']['findings']:
    row=dict(finding)
    element=finding.get('subject',finding.get('element'))
    row['source']=records[element].get('source')
    row['canonical_path']=path(element)
    code=row['code']
    row['classification']='B'
    row['disposition']='Required structural inference/resolution work; not the independent authority stop'
    if code=='validateEndFeatureMembershipIsEnd':
        member=slot(element,'ownedRelatedElement')['references']
        assert len(member)==1
        flag=slot(member[0],'isEnd')
        assert flag['value']=='Scalar(Boolean(false))'
        row.update(classification='A',member=member[0],stored_is_end=False,
            disposition='Lowering leaves the member of an EndFeatureMembership with the default false end flag. The newly implemented check exposes this construction defect; no source correction or end-flag workaround is applied.')
    elif code=='KLS_UNRESOLVED':
        source=reference.find(row['canonical_path'])
        row['reference_redefinitions']=[dict(id=r.get('{http://www.omg.org/XMI}id'),
            implied=r.get('isImplied','false'),targets=[reference.path(t) for t in reference.targets(r,'redefinedFeature')])
            for r in source if reference.kind(r)=='Redefinition']
        row['disposition']='Required redefinition search across renamed inherited members after redundant library specializations are removed. Current reference supplies explicit targets. Not attributed to the separate Clocks/VectorFunctions name ambiguity.'
    elif code=='KQ_AMBIGUOUS_IMPLIED_NAME':
        row['disposition']='Implied specialization/redefinition closure still needs review, including transitive redundancy through implied parents. Distinct possible names are retained as typed incompleteness; no genuinely unordered source case or new authority conflict is asserted from this diagnostic alone.'
    rows.append(row)
report=dict(format='agentique-kerml-v6-individual-findings/1',findings=rows,
    category_counts=dict(Counter(r['classification'] for r in rows)),
    source_records=dict(path=str((OLD/archive['archive']).relative_to(ROOT)),sha256=archive['sha256'],
        scope='Historical canonical declarations corroborated by unchanged lowering and the fresh v3/v4 full-record equality test; not a replacement for final refined endpoints'),
    execution=dict(classification='C',count=quality['current']['totals']['unevaluated_expression_count']),
    authority=dict(classification='E',id='KLCV6-F-001',packet='subobject-authority-conflict.json'),
    ordinary_implementation_complete=False,accepted_publication=False)
(OUT/'individual-findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(category_counts=report['category_counts'],accepted_publication=False)))
