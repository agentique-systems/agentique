"""Compare the Rust selector to an independent normative-XMI class closure.

The corpus sequence comes from canonical source lowering, not a Rust selection
query. The synthetic sequence is exported before independent filtering here.
"""
import json
import sys
from corpus import OUT, Pinned, conforms, digest

p=Pinned()
report=json.loads((OUT/'cross-corpus-v7.json').read_text())
assert report['profile']=='agentique-kerml-1.0-operational/7'
features={e for e in p.records if p.is_kind(e,'Feature')}
assert {r['feature'] for r in report['rows']}==features
selected=[]
for row in report['rows']:
    e=row['feature']
    expected_members=[m for m in p.refs(e,'ownedRelationship') if p.is_kind(m,'Membership')]
    assert row['owned_memberships']==expected_members,(e,'ordered memberships')
    assert row['selected']==p.select_cross(e),(e,row['selected'],p.select_cross(e))
    assert row['completeness']=='Complete'
    if row['selected']:selected.append(dict(feature=e,cross_feature=row['selected']))
matrix=[]
for line in (OUT/'selector-independent-matrix-2/output.txt').read_text().splitlines():
    if not line.startswith('MATRIX '):continue
    row=json.loads(line[7:]);expected=None
    v7=row['profile']=='agentique-kerml-1.0-operational/7'
    if row['is_end'] and row['owning_type']:
        for candidate in row['sequence']:
            m=conforms(candidate['membership_metaclass']);v=conforms(candidate['member_metaclass'])
            if 'OwningMembership' not in m or 'FeatureMembership' in m:continue
            if v7 and 'FeatureValue' in m:continue
            if 'Feature' not in v or v.intersection({'Multiplicity','MetadataFeature','BindingConnector' if v7 else 'FeatureValue'}):continue
            expected=candidate['member'];break
    assert row['selected']==expected,row
    matrix.append(row)
assert len(matrix)==120,len(matrix)
witnesses=[]
for path in ['Occurrences::Occurrence::incomingTransfersToSelf::target','Occurrences::Occurrence::outgoingTransfersFromSelf::source']:
    owner,name=path.rsplit('::',1)
    candidates=[v for _,v in p.members(p.find(owner)) if p.is_kind(v,'Feature') and p.records[v].get('source',{}).get('text')==f'end feature redefines {name} = that;']
    assert len(candidates)==1,(path,candidates)
    e=candidates[0]
    assert p.select_cross(e) is None
    assert p.select_cross(e,False) is not None
    witnesses.append(dict(path=path,feature=e,published_through_v6=p.select_cross(e,False),v7=None))
result=dict(format='agentique-independent-owned-cross-feature-verification/1',
    corpus_archive_sha256=digest(p.archive),rust_report_sha256=digest(OUT/'cross-corpus-v7.json'),
    feature_count=len(features),selected_count=len(selected),synthetic_cases=len(matrix),
    profiles=sorted({r['profile'] for r in matrix}),selected=selected,witnesses=witnesses,
    all_exact_order_and_identity_comparisons_passed=True,
    scope='Local selection over full pinned corpus and synthetic matrix; full implied structural expansion is a separate gate.')
encoded=json.dumps(result,indent=2)+'\n';path=OUT/'selector-verification.json'
if '--check' in sys.argv:assert path.read_text()==encoded
else:path.write_text(encoded,encoding='utf8',newline='\n')
print(f'{len(features)} corpus Features, {len(selected)} selected cross Features, {len(matrix)} synthetic cases: exact ordered selection agrees')
