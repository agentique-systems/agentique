"""Recount measured construction and authority inventory; never invent closure."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys
from bs4 import BeautifulSoup

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
quality=json.loads((OUT/'baseline-quality.json').read_text())
rows=quality['documents']
authority=json.loads((ROOT/'standards/kerml-1.0-constraint-authority-map.json').read_text())
tracker=BeautifulSoup((OUT/'issues/KerML-all.html').read_text(encoding='utf8'),'html.parser')
issues=tracker.select('article.issue-description')
counts={key:sum(r[key] for r in rows) for key in ['canonical_element_count','canonical_relationship_count',
    'reference_count','unresolved_count','incomplete_reference_count','ambiguous_count',
    'mismatched_endpoint_count','unevaluated_expression_count']}
diagnostics=Counter(d['code'] for row in rows for d in row['diagnostics_by_category']['KerML_semantic']['query_diagnostics'])
checks=Counter()
for row in rows:
    checks.update(row['full_KerML_constraint_validation']['evaluated_constraints'])
status=Counter(r['current_implementation_status'] for r in authority['constraints'])
measure=dict(format='agentique-v9-measurement/1',scope='Fresh unchanged v5 production audit, launched before v9 semantics edits. Not a completed v6 corpus expansion.',
    profile=quality['baseline_profile'],rule_version=quality['rule_version'],
    quality_sha256=hashlib.sha256((OUT/'baseline-quality.json').read_bytes()).hexdigest(),
    documents=len(rows),libraries=len({r['library_id'] for r in rows}),counts=counts,
    end_membership_findings=diagnostics['validateEndFeatureMembershipIsEnd'],
    namespace_distinguishability_findings=diagnostics['validateNamespaceDistinguishibility'],
    expression_result_cardinality_findings=diagnostics['validateExpressionResultParameterMembership'],
    all_diagnostics=dict(diagnostics),named_checks_executed=len(checks),check_counts=dict(sorted(checks.items())),
    named_formal_constraints=len(authority['constraints']),current_retained_issue_records=len(issues),
    historical_authority_map_status_counts=dict(status),accepted_publication=False)

# Applicability is asserted only where this turn established evidence. Other rows
# remain obligations at the independent authority stop, not execution deferrals.
covered={'KERML11-145','KERML11-8','KERML11-205','KERML11-206','KERML11-207','KERML11-68','KERML11-76','KERML11-140'}
records=[]
for constraint in authority['constraints']:
    review=[]
    for issue in constraint['known_omg_issues']:
        if issue['status']!='open':
            continue
        key=issue['key']
        if key=='KERML11-1':
            disposition='IndependentAuthorityConflict';evidence='cross-feature-authority-conflict.json'
        elif key in covered:
            disposition='CoveredByOperationalProfile';evidence=(
                'reference-binding-authority.json' if key=='KERML11-8' else
                '../kerml-semantic-closure-v8/authority-matrix.json' if key=='KERML11-145' else
                'Historical immutable operational manifests v1-v5')
        else:
            disposition=None;evidence='Applicability review stopped at independently reproduced KLCV9-F-001; no negative applicability or implementation claim.'
        review.append(dict(issue=key,classification=disposition,evidence=evidence))
    records.append(dict(rule=constraint['rule'],owning_metaclass=constraint['owning_metaclass'],
        fresh_baseline_check_count=checks[constraint['rule']],historical_status=constraint['current_implementation_status'],
        issue_applicability=review))
decisions=Counter(r['classification'] or 'UnreviewedAtAuthorityStop' for c in records for r in c['issue_applicability'])
applicability=dict(format='agentique-v9-authority-applicability/1',complete=False,authority_stop='KLCV9-F-001',
    scope='All inherited formal-rule/issue associations recounted. Positive authority dispositions are distinct from full implementation/acceptance.',
    constraints=records,classification_counts=dict(decisions),
    independent_conflict='cross-feature-authority-conflict.json',deferred_execution_structural_rules=0,
    strict_structural_coverage_gate_passed=False,publication_gate_passed=False)
for name,value in [('measurement.json',measure),('authority-applicability.json',applicability)]:
    encoded=json.dumps(value,indent=2)+'\n';path=OUT/name
    if '--check' in sys.argv:
        assert path.read_text(encoding='utf8')==encoded
    else:
        path.write_text(encoded,encoding='utf8',newline='\n')
print(json.dumps(dict(counts=counts,named_formal_constraints=len(records),issue_records=len(issues),
    existing_authority_map_status_counts=dict(status),current_applicability_counts=dict(decisions))))
