"""Independent reference-side positional-redefinition inventory; not normative order."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET
sys.dont_write_bytecode=True
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
sys.path.insert(0,str(ROOT/'verification/kerml-library-content-errata-publication-v5'))
from xmi import Model, XID
directory=OUT/'release/sysml.library.xmi.implied'
model=Model(directory)
def brief(e):
    return dict(file=model.files[e],id=e.get(XID),metaclass=model.kind(e),path=model.path(e))
rows=[]
for rel in model.files:
    if model.kind(rel)!='Redefinition' or rel.get('isImplied')!='true':continue
    source=model.parents[rel]
    membership=model.parents.get(source)
    owner=model.parents.get(membership)
    if membership is None or owner is None:continue
    family='result' if model.kind(membership)=='ReturnParameterMembership' else (
        'end' if source.get('isEnd')=='true' else (
            'parameter-candidate' if source.get('direction') in ['in','out','inout'] else None))
    if family is None:continue
    targets=model.targets(rel,'redefinedFeature')
    sequence=[r for r in source if r.tag=='ownedRelationship']
    rows.append(dict(source=brief(source),owner=brief(owner),membership=brief(membership),
        family=family,relationship_id=rel.get(XID),reference_owned_relationship_position=sequence.index(rel)+1,
        targets=[brief(t) for t in targets],reference_xml=ET.tostring(rel,encoding='unicode'),
        match=None,agentique_targets=None,
        authority_status='Implementation corroboration only; no normative insertion order inferred',
        comparison_status='Reference side captured; Agentique comparison unfinished at independent authority stop'))
rows.sort(key=lambda r:(r['source']['file'],r['source']['id'],r['relationship_id']))
artifacts=[dict(path=p.relative_to(OUT).as_posix(),sha256=hashlib.sha256(p.read_bytes()).hexdigest())
           for p in sorted(directory.rglob('*.kermlx'))]
report=dict(format='agentique-reference-positional-oracle/1',
    reference_commit=json.loads((OUT/'release-head.json').read_text())['sha'],
    artifacts=artifacts,rows=rows,counts=dict(Counter(r['family'] for r in rows)),
    complete_comparison=False,accepted_publication=False,
    caveat='Parameter candidates are selected from explicit direction/ownership facts, not asserted to exhaust normative rule applicability. XML IDs are reference identities only.')
(OUT/'reference-positional-oracle.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(files=len(artifacts),counts=report['counts'],complete_comparison=False)))
