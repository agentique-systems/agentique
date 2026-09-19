"""Actual corpus findings and remaining work; no acceptance or execution waiver."""
import collections
import json
from pathlib import Path
import re
from evidence_io import read_json

OUT=Path(__file__).resolve().parent
records=read_json(OUT/'operational-declarations/model.json')['records']
profiles={name:json.loads((OUT/folder/'quality.json').read_bytes()) for name,folder in [('v2','baseline-quality'),('v3','operational-quality')]}
summaries={}
for name,quality in profiles.items():
    counts={k:sum(d[k] for d in quality['documents']) for k in ['canonical_element_count','reference_count','unresolved_count','ambiguous_count','incomplete_reference_count','mismatched_endpoint_count','unevaluated_expression_count','recovery_count']}
    diagnostics=[x for d in quality['documents'] for x in d['diagnostics_by_category']['KerML_semantic']['query_diagnostics']]
    counts.update(profile=quality['baseline_profile'],rule_version=quality['rule_version'],documents=len(quality['documents']),
        mandatory_lower_bounds=quality['structural_obligation_count'],diagnostic_rows_by_code=dict(collections.Counter(x['code'] for x in diagnostics)),
        unique_diagnostics_by_code=dict(collections.Counter(code for code,subject in {(x['code'],x['subject']) for x in diagnostics})),
        semantic_quality_gate_passed=quality['semantic_quality_gate_passed'],publication_accepted=quality['published_snapshot'])
    summaries[name]=counts

def owned_result_count(identifier):
    relationships=next((s['references'] for s in records[identifier]['slots'].values() if s['name']=='ownedRelationship'),[])
    return sum(records[r]['metaclass']=='ReturnParameterMembership' for r in relationships)

expressions=[]
distinguishability=[]
for document in profiles['v3']['documents']:
    for diag in document['diagnostics_by_category']['KerML_semantic']['query_diagnostics']:
        if diag['code']=='validateExpressionResultParameterMembership':
            record=records[diag['subject']]
            owned=owned_result_count(diag['subject'])
            cause='inherited result / implied common redefinition remains incomplete' if owned==1 else 'inherited result or implied-result construction requires validation' if owned==0 else 'multiple owned result memberships'
            expressions.append(dict(element=diag['subject'],metaclass=record['metaclass'],document=document['file'],
                source=record['source'],origin=record['origin'],owned_result_memberships=owned,
                rule=diag['code'],classification=cause,execution_exemption=False))
        if diag['code']=='validateNamespaceDistinguishibility':
            is_result='Name "result"' in diag['message']
            is_end='Name "target"' in diag['message']
            cause=('Implied result redefinition currently examines owned general results; the required general result can be inherited.' if is_result else
                   'Positional end population omits the terminal feature of a FeatureChaining general.' if is_end else
                   'Feature-chain parameter population / implied parameter redefinition remains incomplete; no independently established source conflict.')
            ids=re.findall(r'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}',diag['message'])
            distinguishability.append(dict(document=document['file'],actual_diagnostic=diag,category='B',cause=cause,
                correction_scope=('KERML11-76 corrected source facts are present; ordinary inference is still incomplete' if document['file'].endswith(('FeatureReferencingPerformances.kerml','Observation.kerml')) else 'Outside KERML11-76 source-correction scope'),
                conflicting_facts=[dict(element=i,metaclass=records[i]['metaclass'],source=records[i]['source'],origin=records[i]['origin']) for i in ids],
                waived=False))

report=dict(format='agentique-library-quality-comparison/1',profiles=summaries,
    source_assertions=dict(original=4003,active_v3=4000,superseded_and_preserved=3,evidence='source-assertion-diff.json'),
    distinguishability=distinguishability,
    expression_structure=dict(findings=len(expressions),by_metaclass=dict(collections.Counter(e['metaclass'] for e in expressions)),
        by_owned_result_count=dict(collections.Counter(e['owned_result_memberships'] for e in expressions)),
        by_cause=dict(collections.Counter(e['classification'] for e in expressions)),execution_waivers=0,findings_detail=expressions),
    separate_authority_stop='KLCV5-F-001 / KERML11-68',
    ordinary_work_remains=True,accepted_bindings=False,accepted_facade=False,accepted_authored_library_integration=False)
with (OUT/'quality-comparison.json').open('x',encoding='utf8') as stream:json.dump(report,stream,indent=2);stream.write('\n')
print(json.dumps(dict(profiles=summaries,expression_groups=report['expression_structure']['by_owned_result_count'],remaining_distinguishability=len(distinguishability))))
