"""Report measured quality and remaining coverage; never turn unevaluated into success."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()
def write(name,value):
    (OUT/name).write_text(json.dumps(value,indent=2)+'\n')
def read(path):return json.loads(path.read_text())
latest_path=OUT/'quality-order-results-v4-3/quality.json'
latest=read(latest_path)
baseline=read(OUT/'quality-baseline-v4-release/quality.json')
assert latest['baseline_profile']=='agentique-kerml-1.0-operational/4'
assert latest['rule_version']=='agq-kerml-query/14'
inputs=read(OUT/'quality-order-results-v4-3/input-hashes.json')
for row in inputs:
    if row['path'].startswith(('crates/kerml-semantics/src/','crates/kerml-text/src/','crates/kerml/src/','crates/kernel/src/')):
        assert sha((ROOT/row['path']).read_bytes())==row['sha256'],row['path']
def summarize(report):
    fields=['canonical_element_count','canonical_relationship_count','reference_count',
            'unresolved_count','ambiguous_count','incomplete_reference_count','mismatched_endpoint_count',
            'query_count','incomplete_query_count','invalid_query_count','unevaluated_expression_count',
            'recovery_count','unsupported_syntax']
    docs=report['documents']
    groups=Counter()
    findings=[]
    for doc in docs:
        cats=doc['diagnostics_by_category']
        for family,rows in [('resolution',cats['resolution']),
                            ('semantic',cats['KerML_semantic']['query_diagnostics']),
                            ('storage',cats['KerML_semantic']['structural_obligations'])]:
            for row in rows:
                groups[family+':'+row['code']]+=1
                ordinary=row['code'] in ['KQ_IMPLIED_NAMING_ORDER',
                    'validateExpressionResultParameterMembership',
                    'validateFunctionResultParameterMembership',
                    'validateNamespaceDistinguishibility']
                findings.append(dict(document=doc['file'],family=family,**row,
                    classification='B' if ordinary else 'review_pending',
                    disposition='Required structural implementation work; not waived and not the stop reason' if ordinary else
                    'Retained individually; full cause/authority classification not claimed after the independent authority stop'))
    return dict(totals={key:sum(d[key] for d in docs) for key in fields},
                structural_obligation_count=report['structural_obligation_count'],
                grouped_findings=dict(sorted(groups.items())),findings=findings,
                exact_source_preservation=all(d['exact_source_preservation'] for d in docs),
                publication_accepted=report['published_snapshot'])
comparison=dict(format='agentique-kerml-v6-quality-comparison/1',
    baseline=summarize(baseline),current=summarize(latest),
    quality_files=['quality-baseline-v4-release/quality.json','quality-order-results-v4-3/quality.json'],
    independent_authority_stop='KLCV6-F-001',
    classification_complete=False,
    execution=dict(classification='C',scope='Expression/function value execution only; no structural finding is waived'),
    authority=dict(classification='E',evidence='subobject-authority-conflict.json'),
    accepted_semantic_publication=False)
individual=read(OUT/'individual-findings.json')
key=lambda r:(r['document'],r['family'],r['code'],r.get('subject',r.get('element')))
assert Counter(map(key,individual['findings']))==Counter(map(key,comparison['current']['findings']))
comparison['current']['findings']=individual['findings']
comparison['current']['classification_counts']=individual['category_counts']
comparison['classification_complete']=True
comparison['classification_scope']='Every current diagnostic assigned A/B; detailed semantic cause closure remains unfinished. This does not classify every historical incomplete reference or establish publication readiness.'
comparison['individual_evidence']='individual-findings.json'
write('quality-comparison.json',comparison)
old=read(ROOT/'verification/kerml-library-content-errata-publication-v5/validation-coverage.json')
source=(ROOT/'crates/kerml-semantics/src/validation.rs').read_text()
names=set(re.findall(r'"(validate[A-Za-z0-9]+)"',source))
names.update(re.findall(r'"(validate[A-Za-z0-9]+)"',(ROOT/'crates/kerml-semantics/src/namespaces.rs').read_text()))
executions=Counter()
for doc in latest['documents']:
    executions.update(doc['full_KerML_constraint_validation']['evaluated_constraints'])
constraints=[]
for original in old['constraints']:
    row=dict(original)
    row['actual_executions']=executions[row['name']]
    row['implementation_status']='explicit_check' if row['name'] in names else 'coverage_not_established'
    row['reason']='Named check evaluated by validate_local_structure or distinguishability; corpus validity remains separate' if row['name'] in names else 'Mandatory structural coverage remains unfinished; no acceptance claimed'
    # Use this task's category numbering; no structural rule is silently deferred.
    if row['category']==5:row['category']=6
    elif row['category']==6:row['category']=5
    if row['name']=='checkFeatureSubobjectSpecialization':
        row['implementation_status']='independent_authority_conflict'
        row['reason']='KLCV6-F-001; conflicting published required target independently reproduced'
    constraints.append(row)
write('validation-coverage.json',dict(format='agentique-kerml-formal-validation-coverage/3',
    complete=False,authority_stop='KLCV6-F-001',authority_sha256=old['authority_sha256'],
    named_constraints=len(constraints),explicit_checks=sum(r['implementation_status']=='explicit_check' for r in constraints),
    categories={'1':'structural validation','2':'name/resolution','3':'implied/derived structure',
                '4':'execution deferred','5':'not applicable to current corpus','6':'metamodel/source authoring'},
    category_counts=dict(Counter(r['category'] for r in constraints)),constraints=constraints,
    caveat='Inventory and explicit execution accounting only. Not a passing structural coverage gate.'))
print(json.dumps(dict(current=comparison['current']['totals'],groups=comparison['current']['grouped_findings'],
                     publication_accepted=False,authority_stop='KLCV6-F-001')))
