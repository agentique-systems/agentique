"""Independent authority audit: XML/ZIP facts, not Agentique inference output."""
import hashlib
import json
from pathlib import Path
import re
import sys
import zipfile
sys.dont_write_bytecode = True
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
sys.path.insert(0,str(ROOT/'verification/kerml-library-content-errata-publication-v5'))
from xmi import Model
sha=lambda b:hashlib.sha256(b).hexdigest()
normalize=lambda s:re.sub(r'\s+',' ',s)
inventory=json.loads((OUT/'metamodel-inventory.json').read_text())
assert sha((ROOT/inventory['file']).read_bytes())==inventory['sha256']
rule=next(r for r in inventory['members'] if r['name']=='checkFeatureSubobjectSpecialization')
ocl=next(b['body'] for b in rule['bodies'] if b.get('language')=='OCL2.0')
prose=next(b['body'] for b in rule['bodies'] if not b.get('language'))
literal='Occurrence::Occurrence::suboccurrences'
intended='Objects::Object::subobjects'
assert literal in ocl and intended in prose
operations={}
for name in ['specializes','specializesFromLibrary','allSupertypes']:
    operations[name]=next(m for m in inventory['members'] if m['owner']=='Type' and m['name']==name and m['kind']=='ownedOperation')
assert any('allSupertypes()->includes(supertype)' in normalize(b.get('body','')) for b in operations['specializes']['bodies'])
assert any('OrderedSet{self}' in b.get('body','') for b in operations['allSupertypes']['bodies'])
assert any('Return this <code>Type</code>' in b.get('body','') for b in operations['allSupertypes']['bodies'])
assert any('mem <> null' in b.get('body','') for b in operations['specializesFromLibrary']['bodies'])
clauses=json.loads((OUT/'subobject-formal-clauses-v2.json').read_text())
for artifact in clauses:
    assert sha((ROOT/artifact['file']).read_bytes())==artifact['sha256']
    texts=normalize(' '.join(p['text'] for p in artifact['pages']))
    assert literal in texts and intended in texts
manifest=json.loads((ROOT/'standards/normative/sysml-2.0/library-set.json').read_text())
packages=[]
for artifact in manifest['artifacts']:
    if artifact['specification'] != 'KerML':
        continue
    with zipfile.ZipFile(ROOT/artifact['path']) as z:
        for ent in artifact['entries']:
            if not ent['path'].endswith('.kerml'):
                continue
            body=z.read(ent['path'])
            assert sha(body)==ent['sha256']
            names=re.findall(r'^\s*(?:standard\s+)?library\s+package\s+(\w+)',body.decode('utf8'),re.M)
            assert len(names)==1,(ent['path'],names)
            packages.append(dict(path=ent['path'],sha256=ent['sha256'],name=names[0]))
assert len(packages)==36 and all(p['name']!='Occurrence' for p in packages)
archive=next(a for a in manifest['artifacts'] if a['source'].endswith('Semantic-Library.kpar'))
data=(ROOT/archive['path']).read_bytes()
assert sha(data)==archive['sha256']
with zipfile.ZipFile(ROOT/archive['path']) as z:
    path=next(n for n in z.namelist() if n.endswith('/Objects.kerml'))
    source=z.read(path)
entry=next(e for e in archive['entries'] if e['path']==path)
assert sha(source)==entry['sha256']
text=source.decode('utf8')
witness='composite feature subobjects: Object[0..*] subsets objects, suboccurrences'
assert text.count(witness)==1
start=source.index(witness.encode())
model=Model(OUT/'release/sysml.library.xmi.implied')
feature=model.find(intended)
membership=model.parents[feature]
owner=model.parents[membership]
typing=[r for r in feature if model.kind(r)=='FeatureTyping']
assert feature.get('isComposite')=='true'
assert model.kind(owner)=='Structure' and model.path(owner)=='Objects::Object'
assert len(typing)==1 and model.targets(typing[0],'type')==[owner]
assert not any(model.kind(r)=='Conjugation' for r in feature)
assert model.find('Occurrences::Occurrence::suboccurrences') is not feature
# Inspect every captured library, so the missing top-level namespace is not a
# failed partial search or a dependence on unimplemented semantic ordering.
literal_root=[e for e in model.files if e.get('declaredName')=='Occurrence' and
              model.kind(e) in ['Package','LibraryPackage','Namespace']]
assert not literal_root
issue=(OUT/'KERML11-205.html').read_text(encoding='utf8')
assert 'status-public-open">open</span>' in issue and literal in issue and intended in issue
pilot=OUT/'pilot/org.omg.sysml.logic/src/main/java/org/omg/sysml'
mapping=(pilot/'util/ImplicitGeneralizationMap.java').read_text()
assert 'put(FeatureImpl.class, "subobject", "Objects::Object::subobjects")' in mapping
adapter=(pilot/'adapter/FeatureAdapter.java').read_text()
assert 'hasStructureType()? isSubobject()? "subobject": "object"' in adapter
synthetic=json.loads((OUT/'subobject-authority-matrix-1/result.json').read_text())
assert synthetic['exit_code']==0
output=(OUT/'subobject-authority-matrix-1/output.txt').read_text()
assert output.count('targets-distinct=true')==5
report=dict(format='agentique-independent-authority-conflict/1',id='KLCV6-F-001',
    issue=dict(id='KERML11-205',status='open',url='https://issues.omg.org/issues/KERML11-205'),
    classification='E: inconsistent published semantic target; not an inference implementation gap',
    rule=rule,operations=operations,pdf_artifacts=clauses,
    authority_scope='Pinned KerML 1.0. Preliminary revision and pilot output are corroboration only.',
    formal_expression_evaluation='Exact OCL is retained, not executed. The witness evaluates the conflicting target requirement under the separately documented semantic antecedent.',
    pinned_library_packages=packages,
    source=dict(archive=archive['path'],archive_sha256=archive['sha256'],path=path,
                sha256=entry['sha256'],range=[start,start+len(witness.encode())],text=witness),
    reference=dict(release=json.loads((OUT/'release-head.json').read_text())['sha'],
                   feature=model.record(feature),owner=model.record(owner),typing=model.record(typing[0]),
                   antecedent=dict(isComposite=True,owningType='Structure',ownedTyping='Structure'),
                   literal_namespace_candidates=0,files=len(set(model.files.values()))),
    independent_dispositions=dict(
        formal_literal=dict(target=literal,target_exists=False,satisfied=False,
            reason='specializesFromLibrary requires a resolved membership; none exists'),
        prose=dict(target=intended,target_exists=True,satisfied=True,
            reason='Witness is exactly the target; Type::specializes is reflexive'),
        spelling_only=dict(target='Occurrences::Occurrence::suboccurrences',target_exists=True,
            distinct_from_prose_target=True,reason='Correcting the package spelling alone selects a different canonical feature')),
    pilot=dict(commit='d9231d21e621aeabeafa92aef026b3929c859116',
               adapter_sha256=sha((pilot/'adapter/FeatureAdapter.java').read_bytes()),
               mapping_sha256=sha((pilot/'util/ImplicitGeneralizationMap.java').read_bytes()),
               selected_target=intended),
    synthetic=dict(profiles=5,command=synthetic['command'],exit_code=0,
                   output_sha256=sha((OUT/'subobject-authority-matrix-1/output.txt').read_bytes())),
    independent_of_implementation=['Antecedent is established by explicit source and XMI facts',
        'Prose interpretation is reflexively satisfied; inherited or implied edges cannot change this',
        'Literal target has no namespace in all 36 library models',
        'Arbitrary-name kernel witnesses resolve both competing existing targets completely'],
    not_covered_by=['KERML11-81 descriptor correction','KERML11-140 redefinition lookup',
                   'KERML11-76 four-document library patch','KERML11-68 end validation'],
    required_decision='A separately reviewed semantic erratum must choose the required target; v4 does not authorize this change',
    changes_applied=False,accepted_publication=False)
path=OUT/'subobject-authority-conflict.json'
encoded=json.dumps(report,indent=2)+'\n'
if '--check' in sys.argv:
    assert path.read_text()==encoded
else:
    path.write_text(encoded)
print('KLCV6-F-001 independently reproduced: explicit antecedent true; literal rule false; prose rule reflexively true; five synthetic profiles complete')
