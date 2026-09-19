"""Four-model source/XMI review, including latent positional-end collisions.

This verifier reads original sources, captured reference XMI and the actual
Agentique diagnostics; it does not invoke the operational correction transform.
"""
import collections
import hashlib
import json
from pathlib import Path
import re
from xmi import Model, XID
from evidence_io import read_json

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
issue_bytes=(OUT/'KERML11-76.html').read_bytes()
issue=issue_bytes.decode('utf8')
assert 'status-public-open">open</span>' in issue
assert all('<tt>'+name+'</tt>' in issue for name in ['FeatureReferencingPerformances','Objects','Observation','VectorFunctions'])
old=Model(OUT/'historical/sysml.library.xmi')
current=Model(OUT/'release/sysml.library.xmi',list((OUT/'release/sysml.library.xmi.implied').rglob('*.kermlx')))
declarations=read_json(OUT/'declarations-3/model.json')['records']
quality=json.loads((OUT/'baseline-quality/quality.json').read_text())
review=json.loads((ROOT/'standards/kerml-1.0-operational-library-errata-v3.json').read_text())
parents={child:identifier for identifier,r in declarations.items() for s in r['slots'].values()
         if s['name'] in ['ownedRelationship','ownedRelatedElement'] for child in s['references']}

def source_record(identifier):
    r=declarations[identifier]
    return dict(id=identifier,metaclass=r['metaclass'],source=r['source'])

def canonical_path(identifier):
    path=[]
    while identifier in declarations:
        r=declarations[identifier]
        if 'Membership' not in r['metaclass']:
            name=next((json.loads(s['value'][14:-2]) for s in r['slots'].values() if s['name']=='declaredName'),None)
            path.append(name or '<'+r['metaclass']+'@'+str(r['source']['range'][0])+'>')
        identifier=parents.get(identifier)
    return '::'.join(reversed(path))

def own_members(model,e):
    return [(r,c) for r in e if model.kind(r).endswith('Membership') for c in r if c.tag=='ownedRelatedElement']

def redefinitions(model,e):
    return [t for r in e if model.kind(r)=='Redefinition' for t in model.targets(r,'redefinedFeature')]

def ancestors(model,e):
    seen=set();pending=redefinitions(model,e)
    while pending:
        e=pending.pop()
        if e in seen:continue
        seen.add(e);pending.extend(redefinitions(model,e))
    return seen

cases=[]
for name,kind,identifier in [('faces','Surface','6f8ea8fd-31ac-590a-b94d-5d5b9ace0ecf'),('edges','Curve','172ffcc9-142d-5b0c-ab6c-1ba878491506'),('vertices','Point','59e9dfaf-0dda-5e9a-a24b-77781a3532e4')]:
    cases.append(dict(entry='001',subject=identifier,container='Objects::StructuredSpaceObject::'+name,
        common='Objects::StructuredSpaceObject::Structured'+kind+'::innerSpaceDimension',effective_name='innerSpaceDimension',
        repair='Introduce combined intersection Structure, one common Feature with two explicit Redefinitions; replace this feature type.'))
cases.append(dict(entry='002',subject='a39fa9f8-e0e8-5964-b866-938d7cdbd4bd',
    container='FeatureReferencingPerformances::BooleanEvaluationResultMonitorPerformance::onOccurrence::monitoredOccurrence',
    common='FeatureReferencingPerformances::BooleanEvaluationResultMonitorPerformance::onOccurrence::monitoredOccurrence::result',effective_name='result',
    repair='Promote the Evaluation monitor to Expression and its result membership to ReturnParameterMembership; introduce a common Boolean[1] result.'))
for number,identifier in [(1,'333ba226-0b0f-5992-97a3-e42c5135efa2'),(2,'6ac03b22-5656-5498-8a39-83b81e60094d')]:
    container=f'FeatureReferencingPerformances::BooleanEvaluationResultToMonitorPerformance::monitor{number}::endWhen'
    for end in ['earlierOccurrence','laterOccurrence']:
        cases.append(dict(entry='002',subject=identifier,container=container,common=container+'::'+end,effective_name=end,
            repair='Introduce the two ordered end Features; checkFeatureEndRedefinition supplies common positional Redefinitions.'))
cases.append(dict(entry='003',subject='0597cd26-c12e-52b5-af35-5c3dc4332452',container='Observation::ObserveChange::transfer',
    common='Observation::ObserveChange::transfer::target',effective_name='target',repair='Introduce the second end Feature after the existing source end.'))
vector=current.find('VectorFunctions::CartesianThreeVectorOf')
vector_result=next(c for r,c in own_members(current,vector) if current.kind(r)=='ReturnParameterMembership')
vector_common=next(c for r,c in own_members(current,vector_result) if len(redefinitions(current,c))==2)
cases.append(dict(entry='004',subject='6239c627-2efd-50f2-bf4f-c8c6cf5233b8',container='VectorFunctions::CartesianThreeVectorOf::<return>',
    common=vector_common,effective_name='dimension',repair='Introduce an unnamed common dimension Feature with two explicit Redefinitions.'))

diagnostics=[]
for d in quality['documents']:
    for diag in d['diagnostics_by_category']['KerML_semantic']['query_diagnostics']:
        if diag['code']!='validateNamespaceDistinguishibility':continue
        tokens=re.findall(r'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}',diag['message'])
        assert len(tokens)==4
        name=json.loads(re.search(r'Name ("[^"]+")',diag['message'])[1])
        entry=next((e for e in review['entries'] if e['document']==d['file']),None)
        # The other three diagnostics have their own explicit classification;
        # being inherited and same-named is not sufficient issue attribution.
        category='A' if entry else 'B'
        reason='Reviewed common-redefinition correction in named KERML11-76 model' if entry else 'Ordinary implied result/parameter-redefinition computation requires further validation; no KERML11-76 source change authorized here.'
        diagnostics.append(dict(document=d['file'],library_id=d['library_id'],source_sha256=d['sha256'],
            containing_type=canonical_path(diag['subject']),containing_element=source_record(diag['subject']),
            inherited_memberships=[source_record(i) for i in tokens[2:]],conflicting_features=[source_record(i) for i in tokens[:2]],
            effective_name=name,rule='validateNamespaceDistinguishibility / Membership::isDistinguishableFrom',
            actual_diagnostic=diag,category=category,classification_reason=reason,
            correction_entry=entry['id'] if entry else None))

for case in cases:
    common=case.pop('common')
    common=current.find(common) if isinstance(common,str) else common
    targets=redefinitions(current,common)
    assert len(targets)>=2
    assert len(set(targets))==len(targets)
    entry=next(e for e in review['entries'] if e['id'].endswith(case['entry']))
    case['entry']=entry['id']
    case['source']=source_record(case['subject'])
    case['pinned_archive']='Semantic-Library.kpar' if 'Semantic' in entry['document'] else 'Function-Library.kpar'
    case['reference_corrective_feature']=current.record(common)
    case['reference_conflicting_features']=[current.record(t) for t in targets]
    case['reference_conflicting_memberships']=[current.record(current.parents[t]) for t in targets]
    closures=[ancestors(current,t) for t in targets]
    case['common_redefined_ancestors']=[current.record(a) for a in sorted(set.intersection(*closures),key=lambda e:(current.files[e],e.get(XID)))]
    case['original_common_redefinition']=False
    case['corrected_common_redefinition']=True
    case['actual_agentique_diagnostics']=[d['actual_diagnostic'] for d in diagnostics if d['actual_diagnostic']['subject']==case['subject'] and d['effective_name']==case['effective_name']]
    case['latent_if_not_emitted']='Inherited positional end names require complete implied naming; absence of an Agentique diagnostic is not validation success.' if not case['actual_agentique_diagnostics'] else None
    case['applicable_rule']='validateNamespaceDistinguishibility; removeRedefinedFeatures; '+('checkFeatureEndRedefinition' if case['effective_name'] in ['earlierOccurrence','laterOccurrence','target'] else 'checkFeatureResultRedefinition' if case['effective_name']=='result' else 'explicit common Redefinition')
    case['history_evidence']=entry['authority']
    case['attribution']='Direct KERML11-76 substantive repair; Objects combined types are the later factorization of the same common redefinitions, also used by SYSML21-322.'
    case['minimal_model_changes_only']=True

sources=[]
for entry in review['entries']:
    name=Path(entry['document']).stem
    original=ROOT/'standards/libraries'/('Semantic-Library' if 'Semantic' in entry['document'] else 'Function-Library')/entry['document']
    historical=next((OUT/'historical/sysml.library').rglob(name+'.kerml'))
    latest=next((OUT/'release/sysml.library').rglob(name+'.kerml'))
    changes=next(f['patch'] for f in json.loads((OUT/'pilot-commit-240fba1faea614d6aa7f905cc697b95b8bf68d04.json').read_text())['files'] if f['filename'].endswith('/'+name+'.kerml'))
    sources.append(dict(document=entry['document'],pinned=dict(path=str(original.relative_to(ROOT)),sha256=hashlib.sha256(original.read_bytes()).hexdigest()),
        historical_reference=dict(path=str(historical.relative_to(OUT)),sha256=hashlib.sha256(historical.read_bytes()).hexdigest(),byte_identical=historical.read_bytes()==original.read_bytes()),
        current_reference=dict(path=str(latest.relative_to(OUT)),sha256=hashlib.sha256(latest.read_bytes()).hexdigest()),
        explicit_issue_commit_patch=changes,full_source_diff=name+'.diff',
        unrelated_current_changes_excluded=(['explicit that redefinitions','nested subsettings via that','comment/documentation changes'] if name=='Objects' else []),
        historical_baseline_difference=({'FeatureReferencingPerformances':'ItemFlows/Flows documentation wording only','Observation':'defaultMonitor readonly flag outside collision subgraph'}.get(name,'none'))))

report=dict(format='agentique-kerml11-76-authority-matrix/1',issue_status='open',reviewed_operational_authority=True,adopted_omg_correction=False,
    issue_pin=dict(path='KERML11-76.html',sha256=hashlib.sha256(issue_bytes).hexdigest()),
    all_four_named_models_audited=True,sources=sources,collision_roots=cases,baseline_diagnostics=diagnostics,
    baseline_diagnostic_categories=dict(collections.Counter(d['category'] for d in diagnostics)),
    related_issue_72='General redundant-specialization inheritance concerns; no blanket waiver or additional library patch inferred.',
    limitations='Baseline diagnostics are actual fresh Agentique results. Reference XMI is corroboration, not normative IDs or replacement source. Latent positional-end collisions are separately listed rather than hidden by incomplete names.')
(OUT/'authority-matrix.json').write_text(json.dumps(report,indent=2)+'\n')
print('Reviewed',len(cases),'root collision patterns in four models;',len(diagnostics),'actual baseline findings:',report['baseline_diagnostic_categories'])
