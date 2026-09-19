"""Independent Git objects, formal XMI, reference XMI and Rust witness checks."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile

sys.dont_write_bytecode = True
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
sys.path.insert(0,str(ROOT/'verification/kerml-library-content-errata-publication-v5'))
from xmi import Model, XID
COMMIT='d9231d21e621aeabeafa92aef026b3929c859116'
normalize=lambda s: re.sub(r'\s+',' ',s)
digest=lambda b: hashlib.sha256(b).hexdigest()
git=lambda *args: subprocess.check_output(['git','-C',str(ROOT/'.workspaces/kerml-v6-reference.git'),*args])
if '--git' in sys.argv:
    raw=git('cat-file','commit',COMMIT)
    assert hashlib.sha1(b'commit '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==COMMIT
    (OUT/'reference-commit-object.txt').write_bytes(raw)
    diff=git('diff',COMMIT+'^',COMMIT,'--no-ext-diff','--binary')
    (OUT/'reference-git.diff').write_bytes(diff)
    commit=json.loads((OUT/'reference-commit.json').read_text())
    for f in commit['files']:
        for label, rev in [('before',COMMIT+'^'),('after',COMMIT)]:
            data=(OUT/label/f['filename']).read_bytes()
            assert data==git('show',rev+':'+f['filename'])
            if label=='after':
                assert hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()==f['sha']
    print('Verified actual Git commit object, parent-to-commit diff and all six before/after blobs')

for r in json.loads((OUT/'acquisition.json').read_text()):
    assert digest((OUT/r['path']).read_bytes())==r['sha256'],r['path']
raw=(OUT/'reference-commit-object.txt').read_bytes()
assert hashlib.sha1(b'commit '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==COMMIT
issue=(OUT/'KERML11-68.html').read_text(encoding='utf8')
assert 'status-public-open">open</span>' in issue and 'transitionLink.laterOccurrence' in issue
inventory=json.loads((OUT/'metamodel-inventory.json').read_text())
assert digest((ROOT/inventory['file']).read_bytes())==inventory['sha256']
constraint=next(r for r in inventory['members'] if r['name']=='validateRedefinitionEndConformance')
assert any(b['body']=='redefinedFeature.isEnd implies redefiningFeature.isEnd' for b in constraint['bodies'])
clauses={label:json.loads((OUT/(label+'-clauses.json')).read_text()) for label in ['formal','preliminary']}
for d in clauses.values():
    assert digest((ROOT/d['file']).read_bytes())==d['sha256']
assert 'redefinedFeature.isEnd implies redefiningFeature.isEnd' in normalize(next(p['text'] for p in clauses['formal']['pages'] if p['page']==201))
preliminary=next(p for p in clauses['preliminary']['pages'] if 'RedefinitionEndConformance' in p['matches'])
assert 'redefiningFeature.owningType.oclIsKindOf(Association)' in normalize(preliminary['text'])
assert 'redefiningFeature.owningType.oclIsKindOf(Connector)' in normalize(preliminary['text'])
validator='org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend'
before=(OUT/'before'/validator).read_text()
after=(OUT/'after'/validator).read_text()
start=after.index('// validateRedefinitionEndConformance')
new=after[start:after.index('\n\t\t}\t',start)]
assert 'val redefiningOwner = redefiningFeature.owningType' in new
assert 'redefiningOwner instanceof Association || redefiningOwner instanceof Connector' in new
assert new.count('error(')==1 and 'redefinedFeature.isEnd && !redefiningFeature.isEnd' in new
old_start=before.index('// validatRedefinitionEndConformance')
old=before[old_start:before.index('\n\t\t}\t',old_start)]
assert 'owningType' not in old

model=Model(OUT/'release/sysml.library.xmi.implied')
feature=model.find('StatePerformances::StateTransitionPerformance::transitionLinkTarget')
chain=next(e for e in feature.iter() if model.kind(e)=='FeatureChainExpression')
witnesses=[]
for rel in chain.iter():
    if model.kind(rel)!='Redefinition':continue
    source=model.parents[rel]
    for target in model.targets(rel,'redefinedFeature'):
        if target.get('isEnd','false')=='true' and source.get('isEnd','false')=='false':
            membership=model.parents[source]
            assert model.kind(membership)=='FeatureMembership'
            owner=model.parents[membership]
            assert model.kind(owner)=='Feature'
            witnesses.append(dict(relationship=model.record(rel),redefining=model.record(source),
                redefined=model.record(target),owning_type_id=owner.get(XID),owner_metaclass=model.kind(owner),
                redefining_is_end=False,redefined_is_end=True,published_through_v3_valid=False,v4_valid=True))
assert len(witnesses)==1
assert witnesses[0]['redefined']['path']=='Occurrences::HappensBefore::laterOccurrence'
matrix=(OUT/'end-matrix-2/output.txt').read_text()
assert json.loads((OUT/'end-matrix-2/result.json').read_text())['exit_code']==0
rows=[line for line in matrix.splitlines() if line.startswith('profile=')]
assert len(rows)==220
source=ROOT/'standards/libraries/Semantic-Library/Kernel Semantic Library/StatePerformances.kerml'
source_bytes=source.read_bytes()
libraries=json.loads((ROOT/'standards/normative/sysml-2.0/library-set.json').read_text())
artifact=next(a for a in libraries['artifacts'] if a['source'].endswith('Semantic-Library.kpar'))
assert digest((ROOT/artifact['path']).read_bytes())==artifact['sha256']
with zipfile.ZipFile(ROOT/artifact['path']) as archive:
    path=next(p for p in archive.namelist() if p.endswith('/StatePerformances.kerml'))
    assert archive.read(path)==source_bytes
offset=source_bytes.index(b'transitionLink.laterOccurrence')
report=dict(format='agentique-validation-erratum-authority/1',issue='KERML11-68',issue_status='open',
    disposition='Project-authorized operational KerML 1.0/v4 validation correction; not an adopted final KerML 1.1 correction',
    formal_constraint=constraint,formal_pdf_sha256=clauses['formal']['sha256'],formal_xmi_sha256=inventory['sha256'],
    preliminary=dict(status='KerML 1.1 Beta 2; corroboration only',pdf_sha256=clauses['preliminary']['sha256'],constraint_page=preliminary),
    reference_commit=COMMIT,git_object_sha1_verified=True,git_diff_sha256=digest((OUT/'reference-git.diff').read_bytes()),
    exact_validator_before=old,exact_validator_after=new,
    exact_file_diffs=json.loads((OUT/'reference-commit.json').read_text())['files'],
    reference_release_commit=json.loads((OUT/'release-head.json').read_text())['sha'],
    source_witness=dict(path=str(source.relative_to(ROOT)),sha256=digest(source_bytes),range=[offset,offset+len(b'transitionLink.laterOccurrence')]),
    reference_implied_xmi_witness=witnesses,synthetic_canonical_matrix=rows,
    synthetic_implementation='crates/kerml-semantics/tests/redefinition_end_v4.rs',
    subtype_treatment='Xtend instanceof / OCL oclIsKindOf: semantic metaclass conformance, including subtypes; no modeled-type substitution for owningType',
    mutation_scope='Validation disposition only; source, XMI, KPAR, isEnd values and feature-chain structure unchanged')
packet=OUT/'authority-packet.json'
encoded=json.dumps(report,indent=2)+'\n'
if '--check' in sys.argv: assert packet.read_text()==encoded
else: packet.write_text(encoded)
print('KERML11-68 verified: Git object and diff, formal/preliminary rule, current implied XMI and 220 canonical profile/flag/owner cases')
