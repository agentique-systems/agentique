"""Exhaustive inventory traversal and independent local authority reproductions.

No Rust query is invoked. A completed traversal is deliberately distinguished
from proof that every structural applicability question has been closed.
"""
import collections
import json
import re
import sys
from bs4 import BeautifulSoup
from corpus import *

def write(path,value):
    text=json.dumps(value,indent=2,ensure_ascii=False)+'\n'
    if '--check' in sys.argv:assert path.read_text(encoding='utf8')==text,path
    else:path.write_text(text,encoding='utf8',newline='\n')

p=Pinned();m=reference()
soup=BeautifulSoup((OUT/'issues/KerML-all.html').read_bytes(),'html.parser')
issues={}
for article in soup.select('article.issue-description'):
    text=article.get_text(' ',strip=True);key=re.search(r'Key:\s*(\S+)',text).group(1)
    issues[key]=dict(key=key,title=article.h2.get_text(' ',strip=True),status=re.search(r'Status:\s*(open|closed)',text).group(1),
        url=f'https://issues.omg.org/issues/{key}',anchor=article['id'],text=text,
        updated=re.search(r'Updated:\s*(.*?GMT)',text).group(1))
assert len(issues)==410
formal={r['name']:r for r in inventory['members'] if r['name']}
def rules(*names):return [r for n in names for r in inventory['members'] if r['name']==n]
def supers(x):
    result=[]
    for r in x:
        if kind(m,r,'Specialization'):
            prop=next((prop for cls,prop in [('FeatureTyping','type'),('Redefinition','redefinedFeature'),('CrossSubsetting','crossedFeature'),('ReferenceSubsetting','referencedFeature'),('Subsetting','subsettedFeature'),('Subclassification','superclassifier')] if kind(m,r,cls)),'general')
            result.extend(m.targets(r,prop))
    return result
def closure(x):
    seen=set();pending=[x]
    while pending:
        x=pending.pop()
        if x in seen:continue
        seen.add(x);pending.extend(supers(x))
    return seen
def rows(xs):return sorted((brief(m,x) for x in xs),key=lambda r:(r['file'],r['id']))
def search(key):
    path=OUT/f'resolution/search-{key}.json'
    if not path.exists():return dict(searched=False)
    j=json.loads(path.read_text());assert not j['incomplete_results'];assert j['total_count']==len(j['items'])
    return dict(searched=True,source=path.relative_to(ROOT).as_posix(),sha256=digest(path),
        commits=[dict(commit=r['sha'],title=r['commit']['message'].splitlines()[0],url=r['html_url']) for r in j['items']],
        interpretation='Issue-associated commit search, not proof of OMG adoption; no result is not proof that no unlabelled commit exists.')
blockers=[]
def block(key,issue,rule_names,witnesses,facts,published,alternatives,reference,reason):
    blockers.append(dict(id=key,issue=issues.get(issue,dict(key=issue,status='No matching issue found in retained tracker')),
        formal=rules(*rule_names),pinned_witnesses=witnesses,complete_local_structural_facts=facts,
        published_result=published,alternative_results=alternatives,reference_behavior=reference,
        issue_associated_resolution=search(issue),covered_by_profile=False,authorized_correction=False,
        independent_local_reproduction=True,execution_required=False,acceptance_block=reason))

# A real owned cross Feature remains after the authorized selector correction.
path='Occurrences::Occurrence::surroundedByOccurrences::surroundingSpace'
end=p.find(path);owner=p.owner(end);cross=p.select_cross(end)
assert cross and p.scalar(end,'isEnd') is True
owner_ref=m.find(p.path(owner));ancestors=closure(owner_ref)
inherited_ends=[v for a in ancestors if a is not owner_ref for r,v in members(m,a) if kind(m,r,'FeatureMembership') and v.get('isEnd')=='true']
assert not inherited_ends
own_ends=[v for r,v in p.members(owner) if p.is_kind(r,'FeatureMembership') and p.scalar(v,'isEnd')]
assert own_ends==[end]
ref=m.find(path);assert ref.get('isEnd','false')=='false'
block('KLCV10-F-001','KERML11-2',['checkFeatureCrossingSpecialization','validateCrossSubsettingCrossingFeature','ownedCrossFeature'],
    [p.brief(end),p.brief(cross)],dict(owning_type=p.brief(owner),own_end_sequence=own_ends,
        reference_supertype_closure=rows(ancestors),inherited_ends=[],selected_cross=cross,end_flag=True),
    'v7 selects the authored cross Feature. Crossing specialization requires an owned CrossSubsetting, whose crossing Feature has only one end in its owning Type; the >1 validation predicate is false.',
    ['Remove the source end/cross-multiplicity as suggested by the issue.','Change the applicability or validity of unary crossings. Neither change is authorized.'],
    dict(end=m.record(ref),is_end=False,source_diff='Reference treats surroundingSpace as a regular feature with an ordinary multiplicity.'),
    'Required structural crossing and its validity cannot both hold for the exact pinned end. KERML11-1 changes selection only and cannot repair this witness.')

# checkMultiplicityTypeFeaturing and validateFeatureMultiplicityDomain impose
# contradictory values without evaluating the multiplicity or its expression.
path='TransitionPerformances::TransitionPerformance::accept'
feature=p.find(path);mults=[(r,v) for r,v in p.members(feature) if p.is_kind(v,'Multiplicity')]
assert len(mults)==1
membership,mult=mults[0]
assert not p.is_kind(membership,'FeatureMembership')
feature_owner=p.owner(feature);assert p.is_kind(p.parents[feature],'FeatureMembership')
assert not p.scalar(feature,'isVariable')
ref=m.find(path);ref_mult=[v for _,v in members(m,ref) if kind(m,v,'Multiplicity')][0]
domain=related(m,ref,'TypeFeaturing','featuringType');mult_domain=related(m,ref_mult,'TypeFeaturing','featuringType')
assert len(domain)==1 and domain==mult_domain
block('KLCV10-F-002','KERML11-4',['checkMultiplicityTypeFeaturing','validateFeatureMultiplicityDomain','checkFeatureFeatureMembershipTypeFeaturing','isFeaturingType'],
    [p.brief(feature),p.brief(mult)],dict(feature_owning_type=p.brief(feature_owner),feature_is_variable=False,
        multiplicity_owning_membership=p.brief(membership),multiplicity_owning_type=None,
        required_feature_featuring_types=[feature_owner],required_multiplicity_featuring_types_literal=[]),
    'checkMultiplicityTypeFeaturing requires an empty domain because owningType is null; validateFeatureMultiplicityDomain requires the nonempty featuring domain of the owning Feature.',
    ['Use owningNamespace in checkMultiplicityTypeFeaturing, as proposed by KERML11-4.','Change multiplicity-domain validation. Neither is authorized.'],
    dict(multiplicity=m.record(ref_mult),feature_featuring_types=rows(domain),multiplicity_featuring_types=rows(mult_domain)),
    'The same Multiplicity is required to have both an empty and a nonempty featuringType. No runtime value or unresolved reference is needed for this contradiction.')

# The existing v9 packet already retained this literal/prose discrepancy. Now
# prove it on a legitimate cross Feature rather than FeatureValue infrastructure.
path='Links::SelfLink::sameThing';end=p.find(path);cross=p.select_cross(end);owner=p.owner(end)
ends=[v for r,v in p.members(owner) if p.is_kind(r,'FeatureMembership') and p.scalar(v,'isEnd')]
assert len(ends)==2 and cross not in ends
ref_end=m.find(path);ref_cross=[v for r,v in members(m,ref_end) if kind(m,v,'Feature') and m.name(v)=='self2'][0]
domains=related(m,ref_cross,'TypeFeaturing','featuringType');assert len(domains)==1 and m.kind(domains[0])=='Classifier'
body=' '.join(b.get('body','') for b in formal['checkFeatureOwnedCrossFeatureTypeFeaturing']['bodies'])
assert 'endFeature->excluding(self)' in body
block('KLCV10-F-003','UNREPORTED-owned-cross-featuring-otherEnds',['checkFeatureOwnedCrossFeatureTypeFeaturing','isOwnedCrossFeature','isCartesianProduct'],
    [p.brief(end),p.brief(cross)],dict(owning_type=p.brief(owner),end_sequence=ends,context_self=cross,
        literal_other_ends=[e for e in ends if e!=cross],opposite_end_alternative=[e for e in ends if e!=end]),
    'The constraint is evaluated on the cross Feature; excluding(self) removes neither association end. Even this binary association therefore takes the Cartesian-product branch.',
    ['Exclude the owning end instead of the cross Feature, yielding the opposite-end domain. This is what current pilot code implements.'],
    dict(cross=m.record(ref_cross),featuring_types=rows(domains),pilot_source='pilot/FeatureAdapter.java',
        pilot_sha256=digest(OUT/'pilot/FeatureAdapter.java'),pilot_method='addOwnedCrossFeatureTypeFeaturing',
        pilot_binary_domain_is_cartesian_product=False),
    'The literal formal rule rejects the corroborated binary domain. The exact authorized KERML11-1 diff does not change this different constraint; selecting the alternative needs a separate authority decision.')

# A later independent sweep step reproduced the imported-membership prose/OCL
# disagreement. Keep collecting conflicts; do not change import semantics.
imports_path=OUT/'import-authority.json'
imports=json.loads(imports_path.read_text(encoding='utf8'))
assert imports['source_archive_sha256']==digest(p.archive)
assert imports['local_contradiction_independently_proven'] and len(imports['witnesses'])==5
assert not imports['correction_applied']
block('KLCV10-F-004','KERML11-75',['importedMemberships','deriveNamespaceImportedMembership','deriveNamespaceMembers'],
    [imports['import_']]+[w['facts'][key] for w in imports['witnesses'] for key in ['imported_membership','imported_member','local_membership','local_member']],
    dict(namespace=imports['namespace'],imported_namespace=imports['imported_namespace'],
        independent_packet=imports_path.relative_to(ROOT).as_posix(),independent_packet_sha256=digest(imports_path),
        collisions=imports['witnesses']),
    'The literal Namespace::importedMemberships OCL includes five direct public NumericalFunctions memberships imported into VectorFunctions. Each collides with a local same-metaclass membership.',
    ['The published prose excludes imported memberships that collide with owned memberships, removing these five identities. The two published descriptions disagree; v7 does not authorize choosing a correction.'],
    imports['reference'],
    'The same five imported Membership identities must be included by the OCL and excluded by its prose. The contradiction is proved from direct ownership, explicit names, visibility and metaclass conformance without incomplete inherited lookup or execution. Strict publication requires a separate authority decision.')

write(ROOT/'standards/kerml-1.0-operational-authority-blockers.json',dict(
    format='agentique-operational-authority-blockers/1',profile='agentique-kerml-1.0-operational/7',
    issue_inventory_sha256=digest(OUT/'issues/KerML-all.html'),normative_xmi_sha256=inventory['sha256'],
    reference_release=json.loads((OUT/'release-head.json').read_text())['sha'],
    source_archive_sha256=digest(p.archive),blockers=blockers,
    register_scope='All independently proven conflicts from this traversal. This is not a claim that incomplete structural applicability analyses are closed.',
    unauthorized_corrections_applied=False))

# Traverse EVERY retained issue and EVERY formal member. Preserve all pending
# proof work instead of turning missing implementation into false applicability.
old=json.loads((ROOT/'standards/kerml-1.0-constraint-authority-map.json').read_text())
authorized={'KERML11-81':1,'KERML11-140':2,'KERML11-76':3,'KERML11-68':4,'KERML11-205':5,'KERML11-206':5,'KERML11-207':5,'KERML11-145':6,'KERML11-8':6,'KERML11-1':7}
editorial={f'KERML11-{n}' for n in [6,7,9,10,11,13,33,70,74,77,83,85,89,91,92,93,94,95,107,108,109,110,111,112,113,114,115,138,146,180,194,195,196,198,200,203,204]}
not_exercised={'KERML11-12':'No ElementFilterMembership or filter expression in any pinned declaration or reference model.',
    'KERML11-183':'No ElementFilterMembership or filter expression in any pinned declaration or reference model.',
    'KERML11-82':'All five source Flows use longhand bodies; no FlowEnd exists in source or reference. No shorthand FlowEnd is generated for these declarations.',
    'KERML11-96':'No variable Connector in pinned construction; the disputed case requires a variable Connector.',
    'KERML11-191':'The only Type-owned import is the private SequenceFunctions import on Occurrence. It cannot be inherited through non-private membership projection.',
    'KERML11-182':'All 657 pinned FeatureReferenceExpressions have an explicit named or owned referent. Supplemental sequence evidence verifies every non-parameter Membership; v7 producers create no FeatureReferenceExpression.',
    'KERML11-32':'Same exhaustive non-exercise proof as KERML11-182; no implicit Anything::self is required by the pinned source population.'}
for c in ['ElementFilterMembership','FlowEnd']:
    assert not [e for e in p.records if p.is_kind(e,c)]
    assert not [e for e in m.ids.values() if kind(m,e,c)]
assert not [e for e in p.records if p.is_kind(e,'Connector') and p.scalar(e,'isVariable')]
imports=[e for e in p.records if p.is_kind(e,'Import') and p.is_kind(p.parents[e],'Type')]
assert len(imports)==1 and p.records[imports[0]]['source']['text']=='private import SequenceFunctions::*;'
referent_review=json.loads((OUT/'supplemental-authority-review.json').read_text(encoding='utf8'))
assert referent_review['pinned_archive_sha256']==digest(p.archive)
referent_review=referent_review['operandless_reference_expressions']
assert referent_review['missing_referents']==0 and referent_review['source_population']==657
assert {r['expression'] for r in referent_review['sequence_evidence']}=={e for e in p.records if p.is_kind(e,'FeatureReferenceExpression')}
pending={'KERML11-3':'Cross multiplicity bound domain requires independent analysis of the KERML11-4 alternative and the retained KERML11-3 resolution commits.',
    'KERML11-69':'All 23 IndexExpression operand-result type closures must be established; the Collection/Array guard difference is not silently adopted.',
    'KERML11-72':'Complete diamond and renamed-member suppression analysis is still required after all ordinary producer implications.',
    'KERML11-75':'Five direct VectorFunctions imported/owned collisions are independently registered as KLCV10-F-004. Exhaustive import closure still requires review; no alternative is adopted.',
    'KERML11-90':'Both constructors inherit default FeatureValues. The prose rule and missing OCL require a complete default-binding graph analysis.',
    'KERML11-79':'Exact accept::receiver witness retained; explicit and positional redefinition interactions require complete parameter closure.'}
extra_links={
    'KERML11-2':['checkFeatureCrossingSpecialization','validateCrossSubsettingCrossingFeature'],
    'KERML11-4':['validateFeatureMultiplicityDomain'],
    'KERML11-79':['checkFeatureParameterRedefinition'],
    'KERML11-32':['deriveFeatureReferenceExpressionReferent'],
}
constraint_rows=[]
for row in old['constraints']:
    linked={i['key'] for i in row['known_omg_issues']}
    linked.update(k for k,ns in extra_links.items() if row['rule'] in ns)
    blocker_ids=[b['id'] for b in blockers if row['rule'] in [f['name'] for f in b['formal']]]
    open_links={k for k in linked if issues[k]['status']=='open'}
    reviewed_issues=set(authorized)|editorial|set(not_exercised)|{'KERML11-2','KERML11-4'}
    unresolved_review=sorted(open_links-reviewed_issues)
    if blocker_ids:status='IndependentAuthorityConflict'
    elif open_links.intersection(authorized):status='CoveredByOperationalProfile'
    elif open_links and open_links<=editorial:status='EditorialIssueOnly'
    elif open_links and open_links<=set(not_exercised):status='KnownIssueNotExercised'
    elif open_links:status='KnownIssueExercisedNoContradiction'
    elif linked:status='CoveredByPublishedSemantics'
    else:status='NoKnownRelevantIssue'
    population=[e for e in p.records if p.is_kind(e,row['owning_metaclass'])]
    constraint_rows.append(dict(rule=row['rule'],owning_metaclass=row['owning_metaclass'],
        source_population=len(population),source_metaclass_exercised=bool(population),
        authority_status=status,linked_issues=sorted(linked),blockers=blocker_ids,
        authority_status_is_final=not unresolved_review,
        linked_issue_review_closed=not unresolved_review,
        applicability_proof_closed=False,
        pending_proofs=[dict(issue=k,reason=pending.get(k,'The retained issue is linked to this structural rule; exhaustive disputed-antecedent population proof has not been completed.')) for k in unresolved_review],
        non_exercise_proofs=[dict(issue=k,proof=not_exercised[k]) for k in sorted(open_links.intersection(not_exercised))],
        note='Status is provisional where authority_status_is_final is false. NoContradiction means no independently established contradiction in this review, not a final applicability or validation claim. Source metaclass population is not proof of each conditional antecedent.'))
assert len(constraint_rows)==258
members=[]
for r in inventory['members']:
    if r['kind'] not in ['ownedOperation','ownedAttribute']:continue
    if r['kind']=='ownedAttribute' and r.get('attributes',{}).get('isDerived')!='true':continue
    members.append(dict(owner=r['owner'],name=r['name'],kind=r['kind'],
        source_population=sum(p.is_kind(e,r['owner']) for e in p.records),
        linked_issues=[key for key,i in issues.items() if r['name'] and r['name'].lower() in i['text'].lower()],
        formal=r,applicability_proof_closed=False))
issue_rows=[]
for key,issue in issues.items():
    linked=[r['rule'] for r in constraint_rows if key in r['linked_issues']]
    disposition=('CoveredByOperationalProfile' if key in authorized else 'CoveredByPublishedSemantics' if issue['status']=='closed' else
        'EditorialIssueOnly' if key in editorial else 'KnownIssueNotExercised' if key in not_exercised else
        'IndependentAuthorityConflict' if any(b['issue']['key']==key for b in blockers) else 'RetainedForStructuralOrExecutionScopeReview')
    issue_rows.append(dict(**issue,linked_constraints=linked,disposition=disposition,
        earliest_authorized_profile=authorized.get(key),pending_proof=pending.get(key)))
write(OUT/'issue-inventory.json',dict(format='agentique-v10-issue-inventory/1',source_sha256=digest(OUT/'issues/KerML-all.html'),issues=issue_rows))
write(OUT/'authority-applicability.json',dict(format='agentique-v10-authority-applicability/1',
    traversal_complete=True,early_termination_on_conflict=False,complete=False,
    constraints_visited=len(constraint_rows),issues_visited=len(issue_rows),derived_members_visited=len(members),
    constraints=constraint_rows,derived_members=members,
    status_counts=dict(collections.Counter(r['authority_status'] for r in constraint_rows)),
    independently_proven_blockers=len(blockers),remaining_review_work=pending,
    scope='Full inventory traversed, with no early authority stop. Per-antecedent corpus and derived-operation applicability proofs remain open; this report must not be used to claim Gate 8 or structural coverage closure.',
    structural_rules_deferred_as_execution=0,accepted=False))
print(f'Visited all {len(constraint_rows)} constraints, {len(members)} derived/operation members and {len(issues)} issue records; registered {len(blockers)} independently reproduced conflicts. Applicability proof closure remains incomplete.')
if '--require-closure' in sys.argv:
    print('Authority gate FAILED: inventory traversal does not close every structural applicability proof.')
    sys.exit(1)
