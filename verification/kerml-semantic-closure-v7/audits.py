"""Summarize the fresh full audit without converting observations to acceptance."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()
quality_path=OUT/'quality-v5-final-2.json'
quality=json.loads(quality_path.read_text())
documents=quality['documents']
totals={key:sum(d[key] for d in documents) for key in [
    'canonical_element_count','canonical_relationship_count','reference_count',
    'unresolved_count','ambiguous_count','incomplete_reference_count',
    'mismatched_endpoint_count','unevaluated_expression_count','query_count',
    'incomplete_query_count','invalid_query_count']}
findings=[]
for d in documents:
    categories=d['diagnostics_by_category']
    for family,rows in [('resolution',categories['resolution']),
                        ('semantic',categories['KerML_semantic']['query_diagnostics'])]:
        for row in rows:
            findings.append(dict(document=d['file'],family=family,**row,
                classification='B',disposition='Ordinary structural inference/validation obligation. Not execution or authority conflict.'))
codes=Counter(r['code'] for r in findings if r['family']=='semantic')
totals.update(end_membership_findings=codes['validateEndFeatureMembershipIsEnd'],
    distinguishability_findings=codes['validateNamespaceDistinguishibility'],
    expression_result_cardinality_findings=codes['validateExpressionResultParameterMembership']+codes['validateFunctionResultParameterMembership'])
baseline_log=(OUT/'baseline-v4-command/output.txt').read_text(encoding='utf8')
baseline=re.findall(r'^(.*\.kerml): (\d+) candidate elements, (\d+) unresolved, (\d+) incomplete reference answers$',baseline_log,re.M)
assert len(baseline)==36
baseline_totals=dict(canonical_element_count=sum(int(r[1]) for r in baseline),
    unresolved_count=sum(int(r[2]) for r in baseline),incomplete_reference_count=sum(int(r[3]) for r in baseline))
baseline_documents={r[0]:dict(unresolved=int(r[2]),incomplete=int(r[3])) for r in baseline}
changes=[]
for d in documents:
    prior=baseline_documents[d['file']]
    current=dict(unresolved=d['unresolved_count'],incomplete=d['incomplete_reference_count'])
    if current!=prior:changes.append(dict(document=d['file'],v4=prior,v5=current))
comparison=dict(format='agentique-v7-quality-comparison/1',
    v4_baseline=dict(totals=baseline_totals,scope='Fresh pre-change executable. Its command did not request JSON output; only printed count fields are claimed.',
        evidence='baseline-v4-command/output.txt',sha256=sha((OUT/'baseline-v4-command/output.txt').read_bytes())),
    v5=dict(totals=totals,quality_sha256=sha(quality_path.read_bytes())),
    reference_count_changes=changes,
    semantic_publication_accepted=False,authority_stop='KLCV7-F-001')
reports={'quality-comparison.json':comparison,
    'individual-findings.json':dict(format='agentique-v7-individual-findings/1',findings=findings,
        independent_authority_conflict='result-binding-authority-conflict.json'),
    'unresolved-reference-audit.json':dict(count=totals['unresolved_count'],
        rows=[r for r in findings if r['code']=='KLS_UNRESOLVED'],closed=totals['unresolved_count']==0),
    'incomplete-answer-audit.json':dict(count=totals['incomplete_reference_count'],closed=False,
        documents=[dict(file=d['file'],incomplete=d['incomplete_reference_count']) for d in documents if d['incomplete_reference_count']],
        diagnostic_causes=dict(sorted(Counter(r['code'] for r in findings if r['family']=='resolution').items())),
        caveat='Diagnostic causes and answer counts are distinct populations. Per-answer supporting inference closure is not established. A unique candidate never counts as Complete.'),
    'distinguishability-audit.json':dict(count=totals['distinguishability_findings'],
        findings=[r for r in findings if r['code']=='validateNamespaceDistinguishibility']),
    'expression-structural-coverage.json':dict(result_cardinality_findings=totals['expression_result_cardinality_findings'],
        separately_unevaluated=totals['unevaluated_expression_count'],structural_coverage_complete=False,
        required_families=['FeatureReferenceExpression','InvocationExpression','OperatorExpression','classification','cast','index','select','conditional','feature-chain'],
        authority_conflict='KLCV7-F-001',remaining='Full family structural expansion and rule coverage remain mandatory; zero result-cardinality findings is insufficient.'),
    'feature-chain-structural-audit.json':dict(complete=False,canonical_inherited_features_copied=False,
        remaining='Complete source/target/nested Feature, specialization, result, featuring, ownership and proof expansion remains required. No execution deferral is substituted.',
        stop='KLCV7-F-001'),
    'positional-target-comparison.json':dict(complete=False,
        existing_reference_oracle='verification/kerml-semantic-closure-v6/reference-positional-oracle.json',
        sha256=sha((ROOT/'verification/kerml-semantic-closure-v6/reference-positional-oracle.json').read_bytes()),
        disposition='Retained prior reference inventory. Full canonical target differential remains required after independent authority resolution; no ID/insertion order is promoted to normative order.')}
for name,report in reports.items():
    encoded=json.dumps(report,indent=2)+'\n'
    path=OUT/name
    if '--check' in sys.argv: assert path.read_text(encoding='utf8')==encoded,name
    else:path.write_text(encoded,encoding='utf8',newline='\n')
print(json.dumps(comparison))
