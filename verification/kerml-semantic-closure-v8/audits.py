"""Fresh v8 observations and fail-closed coverage; never infer acceptance."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(name,value):
    text=json.dumps(value,indent=2)+'\n'
    path=OUT/name
    if '--check' in sys.argv: assert path.read_text(encoding='utf8')==text,name
    else: path.write_text(text,encoding='utf8',newline='\n')


quality_path=OUT/'baseline-quality.json'
quality=json.loads(quality_path.read_text())
assert not quality['semantic_quality_gate_passed'] and not quality['published_snapshot']
documents=quality['documents']
totals={key:sum(d[key] for d in documents) for key in [
    'canonical_element_count','canonical_relationship_count','reference_count',
    'unresolved_count','ambiguous_count','incomplete_reference_count',
    'mismatched_endpoint_count','unevaluated_expression_count','query_count',
    'incomplete_query_count','invalid_query_count']}
findings=[]
executions=Counter()
for d in documents:
    executions.update(d['full_KerML_constraint_validation']['evaluated_constraints'])
    for family,rows in [('resolution',d['diagnostics_by_category']['resolution']),
                       ('semantic',d['diagnostics_by_category']['KerML_semantic']['query_diagnostics'])]:
        for row in rows:
            findings.append(dict(document=d['file'],family=family,**row,
                disposition='Ordinary structural implementation obligation; no execution deferral or additional authority conflict is inferred.'))
codes=Counter(r['code'] for r in findings if r['family']=='semantic')
totals.update(end_membership_findings=codes['validateEndFeatureMembershipIsEnd'],
    distinguishability_findings=codes['validateNamespaceDistinguishibility'],
    expression_result_cardinality_findings=codes['validateExpressionResultParameterMembership']+codes['validateFunctionResultParameterMembership'])
prior=ROOT/'verification/kerml-semantic-closure-v7/quality-comparison.json'
old=json.loads(prior.read_text())['v5']['totals']
write('quality-comparison.json',dict(format='agentique-v8-quality-comparison/1',
    current=dict(profile=quality['baseline_profile'],totals=totals,quality_sha256=sha(quality_path),
        query_cache_batch_size=quality['query_cache_batch_size'],library_graph_digest=quality['library_graph_digest']),
    historical=dict(path=prior.relative_to(ROOT).as_posix(),sha256=sha(prior),totals=old),
    changes={key:dict(previous=old.get(key),current=value) for key,value in totals.items() if old.get(key)!=value},
    counts_are_observations_not_expected_results=True,
    semantic_publication_accepted=False,authority_stop='KLCV8-F-001'))
write('grouped-diagnostics.json',dict(format='agentique-v8-grouped-diagnostics/1',
    groups=[dict(family=family,code=code,count=sum(r['family']==family and r['code']==code for r in findings),
        findings=[r for r in findings if r['family']==family and r['code']==code])
        for family,code in sorted({(r['family'],r['code']) for r in findings})],
    independently_reproduced_authority_conflict='feature-reference-authority-conflict.json'))
write('reference-obligation-audit.json',dict(
    reference_count=totals['reference_count'],unresolved=totals['unresolved_count'],incomplete=totals['incomplete_reference_count'],
    ambiguous=totals['ambiguous_count'],stored_endpoint_mismatches=totals['mismatched_endpoint_count'],
    all_required_references_complete=False,
    documents=[dict(file=d['file'],unresolved=d['unresolved_count'],incomplete=d['incomplete_reference_count']) for d in documents],
    unique_target_does_not_establish_complete_support=True,diagnostics='grouped-diagnostics.json'))
write('distinguishability-audit.json',dict(findings=[r for r in findings if r['code']=='validateNamespaceDistinguishibility'],
    count=totals['distinguishability_findings'],closed=totals['distinguishability_findings']==0,
    classification='Ordinary inference work; KERML11-72 risk metadata does not establish an additional conflict.'))
write('feature-chain-structural-audit.json',dict(complete=False,
    local_authority_witness='feature-reference-authority-conflict.json',
    proven_scope='ControlFunctions dot source.target chain, its first-feature domain and terminal specialization are included in the independent conflict proof.',
    remaining='Full corpus feature-chain expansion, relationships and proof dependencies are still mandatory after authority resolution.',
    execution_deferred_instead=False,authority_stop='KLCV8-F-001'))
write('positional-redefinition-audit.json',dict(complete=False,
    local_authority_witness='feature-reference-authority-conflict.json',
    proven_scope='Reference result and both inner connector end positional redefinitions are present in the independent proof.',
    remaining='Full corpus positional closure and independent canonical target comparison remain mandatory.',
    identity_order_used=False,execution_deferred_instead=False))

inventory_path=ROOT/'verification/kerml-semantic-closure-v7/validation-coverage.json'
inventory=json.loads(inventory_path.read_text())
family={r['rule'] for r in json.loads((OUT/'authority-matrix.json').read_text())['constraints']}
rows=[]
for previous in inventory['constraints']:
    row=dict(previous)
    name=row['name']
    row['historical_v7_classification']=dict(category=previous['category'],status=previous['implementation_status'])
    if name=='checkFeatureReferenceExpressionBindingConnector':
        row.update(category='F',status='NewAuthorityConflict',implementation_status='NewAuthorityConflict',
            reason='KLCV8-F-001 / KERML11-8; independently reproduced on pinned corpus after relevant result, chain, position and base inference.')
    elif name in family:
        row.update(category='E',status='ReviewedOperationalErratum',implementation_status='NotYetImplemented',
            reason='KERML11-145 operational correction authorized for v6; implementation stopped at the independent Gate 1 conflict. This is no longer an unauthorized authority conflict.',
            authorization='KERML11-145 / v6 task authorization',correction_implemented=False)
    elif row['category']=='B':
        row['reason']='Full structural validation implementation remains required. This v8 preflight stops for independently proven KLCV8-F-001, not for this workload; KERML11-145 is authorized.'
    row['actual_executions']=executions[name]
    rows.append(row)
assert len(rows)==258 and len({r['external_id'] for r in rows})==258
write('validation-coverage.json',dict(format='agentique-v8-formal-coverage/1',
    named_constraints=len(rows),explicit_checks=len(executions),
    category_counts=dict(sorted(Counter(r['category'] for r in rows).items())),
    reviewed_errata_awaiting_implementation=sum(r.get('correction_implemented') is False for r in rows),
    structural_implementation_obligations=sum(r['implementation_status']=='NotYetImplemented' for r in rows),
    inputs=dict(inventory_sha256=sha(inventory_path),quality_sha256=sha(quality_path)),
    complete=False,publication_gate_passed=False,authority_stop='KLCV8-F-001',
    execution_deferral='No structural constraint has been moved into DeferredExecution.',constraints=rows))
write('strict-publication-status.json',dict(format='agentique-v8-publication-status/1',
    accepted=False,authority_stop='KLCV8-F-001',v6_implemented=False,
    accepted_snapshot=None,accepted_bindings=None,loaded_kerml_standard_libraries=None,
    authored_accepted_library_consumption=False,
    blocked_gates=['v6 canonical result corrections','all complete references','full structural validation',
        'feature-chain and positional closure','formal coverage','strict language-semantic acceptance',
        'accepted binding generation','accepted immutable facade','authored accepted-library integration'],
    construction_is_not_acceptance=True,runtime_evaluation_required=False))
print(json.dumps(dict(totals=totals,coverage=Counter(r['category'] for r in rows),accepted=False)))
if '--gate' in sys.argv: sys.exit(1)
