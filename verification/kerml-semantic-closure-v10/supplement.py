"""Independent population review for additional open issues; no corrections.

Reference behavior is explicitly distinguished from pinned-corpus authority.
"""
import json
import sys
from corpus import *
p=Pinned();m=reference()
def generals(x):
    out=[]
    for r in x:
        if kind(m,r,'Specialization'):
            prop=next((prop for cls,prop in [('FeatureTyping','type'),('Redefinition','redefinedFeature'),('CrossSubsetting','crossedFeature'),('ReferenceSubsetting','referencedFeature'),('Subsetting','subsettedFeature'),('Subclassification','superclassifier')] if kind(m,r,cls)),'general')
            out.extend(m.targets(r,prop))
    chain=related(m,x,'FeatureChaining','chainingFeature')
    if chain:out.append(chain[-1])
    return out
def closure(x):
    seen=set();pending=[x]
    while pending:
        x=pending.pop()
        if x in seen:continue
        seen.add(x);pending.extend(generals(x))
    return seen
def result(x):
    own=[v for r,v in members(m,x) if kind(m,r,'ReturnParameterMembership')]
    if own:return own
    return list({v for t in closure(x) for r,v in members(m,t) if kind(m,r,'ReturnParameterMembership')})
indexes=[]
for x in m.ids.values():
    if m.kind(x)!='IndexExpression':continue
    parameters=[v for r,v in members(m,x) if kind(m,r,'ParameterMembership') and not kind(m,r,'ReturnParameterMembership')]
    argument=[v for r,v in members(m,parameters[0]) if kind(m,r,'FeatureValue')][0]
    results=result(argument);assert len(results)==1
    supers=closure(results[0]);array=any(m.path(t)=='Collections::Array' for t in supers);collection=any(m.path(t)=='Collections::Collection' for t in supers)
    indexes.append(dict(expression=brief(m,x),argument=brief(m,argument),result=brief(m,results[0]),
        supertype_closure=sorted((brief(m,t) for t in supers),key=lambda r:(r['file'],r['id'])),
        array=array,collection=collection,guard_differs=array!=collection))
assert len(indexes)==23 and not any(r['guard_differs'] for r in indexes)
constructors=[]
constructor_rule=next(r for r in inventory['members'] if r['name']=='checkConstructorExpressionResultDefaultValueBindingConnector')
constructor_prose=[b['body'] for b in constructor_rule['bodies'] if b.get(UTYPE)=='uml:Comment']
constructor_ocl=[b['body'] for b in constructor_rule['bodies'] if b.get('language')=='OCL2.0']
assert len(constructor_prose)==1 and constructor_ocl==['TBD']
for x in m.ids.values():
    if m.kind(x)!='ConstructorExpression':continue
    instantiated=related(m,x,'Membership','memberElement');assert len(instantiated)==1
    defaults=[dict(feature=brief(m,v),value=brief(m,vv),membership=brief(m,rr)) for t in closure(instantiated[0])
        for r,v in members(m,t) for rr,vv in members(m,v) if m.kind(rr)=='FeatureValue' and rr.get('isDefault')=='true']
    results=[v for r,v in members(m,x) if kind(m,r,'ReturnParameterMembership')]
    assert len(results)==1
    bindings=[v for r,v in members(m,results[0]) if kind(m,v,'BindingConnector')]
    assert not bindings
    constructors.append(dict(expression=brief(m,x),pinned_expression=p.brief(p.find(m.path(x))),
        instantiated_type=brief(m,instantiated[0]),result=brief(m,results[0]),
        reference_result_owned_binding_connectors=[brief(m,v) for v in bindings],
        inherited_default_candidates=sorted(defaults,key=lambda r:r['feature']['id']),
        effective_override_and_binding_proof_complete=False))
assert len(constructors)==2 and all(c['inherited_default_candidates'] for c in constructors)
flows=[]
for path in ['Transfers::transfers','Transfers::flowTransfers','Transfers::messageTransfers']:
    e=p.find(path);x=m.find(path)
    ends=[v for r,v in p.members(e) if p.is_kind(r,'FeatureMembership') and p.scalar(v,'isEnd')]
    assert p.is_kind(e,'Flow') and len(ends)==2
    flows.append(dict(pinned=p.brief(e),owned_ends=[p.brief(v) for v in ends],reference=brief(m,x),
        published_required_general='Transfers::flowTransfers'))
initial=[p.brief(e) for e in p.records if p.is_kind(e,'FeatureValue') and p.scalar(e,'isInitial')]
assert not initial
assertions={r['relationship']:r for r in p.export['active_references']}
referents=[]
for e in p.records:
    if not p.is_kind(e,'FeatureReferenceExpression'):continue
    owned=p.refs(e,'ownedRelationship')
    operands=[r for r in owned if p.is_kind(r,'Membership') and not p.is_kind(r,'ParameterMembership')]
    assert len(operands)==1,(e,operands)
    membership=operands[0];targets=p.refs(membership,'ownedRelatedElement')+p.refs(membership,'memberElement')
    named=assertions.get(membership)
    assert len(targets)==1 or named and named['name'],e
    assert all(p.is_kind(v,'Feature') for v in targets)
    referents.append(dict(expression=e,membership=membership,explicit_owned_or_resolved_targets=targets,
        explicit_reference=named['name'] if named else None))
assert len(referents)==657
review=dict(format='agentique-v10-supplemental-authority-populations/1',
    pinned_archive_sha256=digest(p.archive),reference_release=json.loads((OUT/'release-head.json').read_text())['sha'],
    indexing=dict(issue='KERML11-69',reference_index_expressions=indexes,
        reference_guard_differences=0,pinned_source_index_expressions=sum(p.is_kind(e,'IndexExpression') for e in p.records),
        conclusion='No differing Array/Collection case in all 23 current reference expressions. Complete pinned derived-graph comparison remains required; the operational Array guard is unchanged.'),
    constructors=dict(issue='KERML11-90',witnesses=constructors,
        formal_prose=constructor_prose[0],formal_ocl=constructor_ocl[0],
        independently_proven_authority_contradiction=False,
        conclusion='The default-value rule is exercised. Absence of direct defaults on ChangeSignal/TimeSignal is not a non-applicability proof; inherited default candidates exist. The published rule has prose and a TBD OCL body. Neither current reference result owns a BindingConnector. Missing OCL or reference production alone is not an independent authority contradiction, and does not waive the structural default-binding obligation.'),
    flows=dict(issue='KERML11-78',witnesses=flows,
        conclusion='Pinned transfers is a Flow with ends, so the published rule implies a specialization cycle with flowTransfers. Current reference uses Step for transfers/messageTransfers. This source change is not authorized or applied; a cycle alone is not a structural contradiction proof.'),
    initial_feature_values=dict(count=0,witnesses=initial,
        conclusion='The v7 initial binding implementation has no pinned source antecedent; its that.startShot graph and batch invariance are tested synthetically.'),
    operandless_reference_expressions=dict(issues=['KERML11-182','KERML11-32'],source_population=len(referents),
        missing_referents=0,sequence_evidence=referents,
        conclusion='Every pinned FeatureReferenceExpression has one explicit non-parameter Membership with a named assertion or an owned Feature. No source operandless-reference antecedent exists. The v7 producer set does not create FeatureReferenceExpressions; no implicit Anything::self correction is adopted.'),
    applied_additional_corrections=[])
encoded=json.dumps(review,indent=2,ensure_ascii=False)+'\n';path=OUT/'supplemental-authority-review.json'
if '--check' in sys.argv:assert path.read_text(encoding='utf8')==encoded
else:path.write_text(encoded,encoding='utf8',newline='\n')
print('Reviewed all 23 reference index guards, both constructors with inherited defaults, three Flow witnesses, zero pinned initial FeatureValues, and all 657 explicit FeatureReferenceExpression referents; no additional correction applied.')
