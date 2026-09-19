"""Reproduce KERML11-68 from pinned authority and independent reference XMI.

No Agentique parser, model builder, correction transform or semantic query is used.
"""
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET
from xmi import Model, XID

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
issue=(OUT/'KERML11-68.html').read_text(encoding='utf8')
assert 'status-public-open">open</span>' in issue
assert 'transitionLink.laterOccurrence' in issue
pdf=json.loads((OUT/'formal-clauses.json').read_text())
pages={p['page']:p['text'] for p in pdf['pages']}
normalize=lambda s: re.sub(r'\s+',' ',s)
assert 'redefinedFeature.isEnd implies redefiningFeature.isEnd' in normalize(pages[201])
assert "feature redefines ControlFunctions::'.'::source::target redefines f;" in normalize(pages[296])
source=ROOT/'standards/libraries/Semantic-Library/Kernel Semantic Library/StatePerformances.kerml'
raw=source.read_bytes()
assert b'transitionLink.laterOccurrence' in raw
start=raw.index(b'feature transitionLinkTarget')
end=raw.index(b';',start)+1
model=Model(OUT/'release/sysml.library.xmi',list((OUT/'release/sysml.library.xmi.implied').rglob('*.kermlx')))
feature=model.find('StatePerformances::StateTransitionPerformance::transitionLinkTarget')
chain=next(e for e in feature.iter() if model.kind(e)=='FeatureChainExpression')
failures=[]
for rel in chain.iter():
    if model.kind(rel)!='Redefinition':continue
    redefining=model.parents[rel]
    for target in model.targets(rel,'redefinedFeature'):
        if target.get('isEnd','false')=='true' and redefining.get('isEnd','false')!='true':
            failures.append(dict(relationship=model.record(rel),redefining=dict(file=model.files[redefining],id=redefining.get(XID),path=model.path(redefining),isEnd=False),
                redefined=dict(file=model.files[target],id=target.get(XID),path=model.path(target),isEnd=True)))
assert len(failures)==1
assert failures[0]['redefined']['path']=='Occurrences::HappensBefore::laterOccurrence'
metamodel=next((ROOT/'standards').rglob('KerML.xmi'))
root=ET.parse(metamodel).getroot()
constraint=next(e for e in root.iter() if e.tag=='ownedRule' and e.get('name')=='validateRedefinitionEndConformance')
spec=next(e for e in constraint if e.tag=='specification')
assert normalize(spec.get('body'))=='redefinedFeature.isEnd implies redefiningFeature.isEnd'
end_property=next(e for e in root.iter() if e.tag=='ownedAttribute' and e.get('name')=='isEnd')
default=next(e for e in end_property if e.tag=='defaultValue')
assert default.get('value','false')=='false'
for number in ['76','81','140']:
    assert not any('KERML11-68' in e['authority'] for e in json.loads((ROOT/'standards/kerml-1.0-operational-library-errata-v3.json').read_text())['entries'])
report=dict(id='KLCV5-F-001',issue='KERML11-68',issue_status='open',
    authority='Pinned Published KerML 1.0, independently corroborated by current reference XMI',
    source=dict(path=str(source.relative_to(ROOT)),sha256=hashlib.sha256(raw).hexdigest(),range=[start,end],text=raw[start:end].decode()),
    specification=dict(pdf_sha256=pdf['sha256'],physical_pages=[201,296],printed_pages=[175,270],
        xmi_sha256=hashlib.sha256(metamodel.read_bytes()).hexdigest(),constraint_xml=ET.tostring(constraint,encoding='unicode'),isEnd_default_xml=ET.tostring(default,encoding='unicode')),
    reference_failures=failures,truth_table=[dict(redefined_isEnd=t,redefining_isEnd=s,valid=(not t or s)) for t in [False,True] for s in [False,True]],
    implementation_isolation='Independent XML endpoints and literal flags; arbitrary-name canonical Rust witness; no Agentique feature-chain lowering or operational transform used.',
    existing_corrections_do_not_cover=['KERML11-81 removes contradictory participant descriptors','KERML11-140 changes redefinition name lookup','KERML11-76 corrects inherited-member collisions in four other library documents'],
    alternatives_requiring_new_authority=['Change the prescribed feature-chain nested feature to an end','Relax validateRedefinitionEndConformance for this context','Change the pinned StatePerformances feature-value expression'],
    disposition='Strict semantic publication blocked. No relaxation, source rewrite or unreviewed semantic correction applied.')
(OUT/'authority-stop.json').write_text(json.dumps(report,indent=2)+'\n')
print('KLCV5-F-001 independently reproduced; 1 reference-XMI end-conformance violation')
