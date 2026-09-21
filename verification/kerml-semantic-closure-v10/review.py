"""Recount structural coverage and fail closed on publication prerequisites.

This evidence gate cannot construct an accepted runtime publication. The
production acceptance API remains unfinished until the coverage gaps close.
"""
import collections
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
def read(path):return json.loads(path.read_text(encoding='utf8'))
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def write(path,value):
    encoded=json.dumps(value,indent=2,ensure_ascii=False)+'\n'
    if '--check' in sys.argv:assert path.read_text(encoding='utf8')==encoded,path
    else:path.write_text(encoded,encoding='utf8',newline='\n')

audit_path=OUT/'full-v7-complete.json';audit=read(audit_path)
authority_path=OUT/'authority-applicability.json';authority=read(authority_path)
blocker_path=ROOT/'standards/kerml-1.0-operational-authority-blockers.json';blockers=read(blocker_path)
old=read(ROOT/'verification/kerml-semantic-closure-v7/validation-coverage.json')
profile_path=ROOT/'standards/kerml-1.0-operational-profile-v7.json';profile=read(profile_path)
correction_path=ROOT/'standards/kerml-1.0-operational-owned-cross-feature-errata-v7.json'
assert audit['profile']=='agentique-kerml-1.0-operational/7'
assert sha(correction_path)=='57676586b516124929fad739ff06498980ef9bbeefbfcc2f4e1ce8bb4a0eb3fe'
assert len(authority['constraints'])==len(old['constraints'])==258
assert len(authority['derived_members'])==218

reviewed={
    'checkFeatureValuationSpecialization','checkExpressionResultBindingConnector',
    'checkFunctionResultBindingConnector','checkIndexExpressionResultSpecialization',
    'checkSelectExpressionResultSpecialization','checkFeatureReferenceExpressionBindingConnector',
    'validateRedefinitionEndConformance',
}
rows=[]
for formal in old['constraints']:
    name=formal['name'];count=audit['evaluated_rules'].get(name,0)
    conflicts=[b['id'] for b in blockers['blockers'] if any(f['name']==name for f in b['formal'])]
    status=('BlockedByRegisteredAuthorityConflict' if conflicts else
        'ImplementedAndChecked' if count else
        'ReviewedOperationalErratum' if name in reviewed else
        'SourceAuthoringOnly' if formal['implementation_status']=='SourceAuthoringOnly' else
        'NotYetImplemented')
    rows.append(dict(name=name,external_id=formal['external_id'],owning_metaclass=formal['owning_metaclass'],
        formal_ocl=formal['formal_ocl'],status=status,actual_executions=count,blockers=conflicts,
        note='Execution counts do not certify successful outcomes. An implemented producer is not an independent validator. Structural gaps are not deferred as execution.'))
counts=dict(collections.Counter(r['status'] for r in rows))
coverage_complete=not any(r['status']=='NotYetImplemented' for r in rows)
coverage=dict(format='agentique-v10-structural-coverage/1',complete=coverage_complete,
    corpus_audit_sha256=sha(audit_path),constraints=rows,status_counts=counts,
    structural_rules_deferred_as_execution=0,accepted=False)
write(OUT/'structural-coverage.json',coverage)

queries={('Feature','ownedCrossFeature'):'owned_cross_feature',('Feature','isOwnedCrossFeature'):'is_owned_cross_feature',
    ('Feature','ownedCrossSubsetting'):'owned_cross_subsetting',('Feature','crossFeature'):'cross_feature',
    ('Feature','featureTarget'):'feature_target',('FeatureValue','featureWithValue'):'feature_with_value',
    ('Expression','result'):'structural_result',('Function','result'):'structural_result',
    ('Feature','typingFeatures'):'typing_features (identity projection; complete ordered formal projection is not certified)',
    ('Feature','type'):'feature_types (identity closure; complete implied/ordered type production is not certified)',
    ('Feature','featuringType'):'featuring_types (implemented domain cases; all implied domains are not certified)',
    ('Feature','chainingFeature'):'chaining_features',('Feature','allRedefinedFeatures'):'all_redefined_features',
    ('FeatureReferenceExpression','referent'):'reference_referent',
    ('Connector','relatedFeature'):'connector_endpoints (requires established end reference-subsettings)',
    ('Connector','defaultFeaturingType'):'common_connector_context (endpoint-domain computation)',
    ('Namespace','memberships'):'namespace_members',('Namespace','members'):'namespace_members (member identity projection)',
    ('Type','inheritedMemberships'):'inherited_memberships',('Type','supertypes'):'supertypes',
    ('Feature','supertypes'):'supertypes',('Type','allSupertypes'):'all_supertypes',
    ('Element','owningRelationship'):'owning_relationship',('Element','owner'):'owner',
    ('Feature','owningType'):'owning_type',('Namespace','ownedMembership'):'memberships',
    ('Membership','memberElement'):'member',('Type','ownedFeature'):'direct_features'}
derived=[]
for row in authority['derived_members']:
    query=queries.get((row['owner'],row['name']))
    derived.append(dict(owner=row['owner'],name=row['name'],kind=row['kind'],formal=row['formal'],
        implemented_query=query,structural_publication_proof_complete=False,
        status='PartialImplementation' if query else 'NotYetEstablishedByCompleteCoverageAudit'))
write(OUT/'derived-coverage.json',dict(format='agentique-v10-derived-coverage/1',complete=False,
    operations=derived,inventory_count=len(derived),accepted=False,
    scope='The inventory includes kernel-backed properties and existing queries. Missing a mapping here does not assert absent code; it records missing complete structural publication proof.'))

criteria=dict(
    valid_kernel_snapshot=audit['kernel_storage_valid'],
    exact_v7_profile=audit['profile']=='agentique-kerml-1.0-operational/7',
    v7_correction_manifest_valid=sha(correction_path)=='57676586b516124929fad739ff06498980ef9bbeefbfcc2f4e1ce8bb4a0eb3fe',
    materialized_overlay=audit['overlay_error'] is None and audit['derived_overlay_count']>0,
    complete_required_references=all(audit[k]==0 for k in ['unresolved','incomplete','ambiguous','invalid_references']) and
        audit['expanded_references']['count']==audit['required_references'] and all(audit['expanded_references'][k]==0 for k in ['unresolved','incomplete','ambiguous','invalid']),
    complete_required_derived_facts=False,
    implemented_producers_complete=audit['producer_complete'],
    namespace_distinguishability=audit['query_count']>0 and audit['distinguishability']==0,
    applicable_constraints_checked_without_findings=audit['query_count']>0 and not audit['validation_findings'] and audit['incomplete_queries']==audit['invalid_queries']==0,
    structural_coverage_closed=coverage_complete,
    authority_applicability_sweep_closed=authority['complete'],
    zero_unresolved_authority_blockers=not blockers['blockers'],
    production_semantic_acceptance_api_finalized=False,
)
accepted=all(criteria.values())
assert not accepted
report=dict(format='agentique-v10-publication-review/1',criteria=criteria,
    semantic_publication_accepted=accepted,ordinary_structural_implementation_complete=False,
    accepted_snapshot_ids=[],accepted_bindings_current=False,loaded_library_facade_available=False,
    authority_conflicts=len(blockers['blockers']),authority_inventory_traversal_complete=authority['traversal_complete'],
    authority_applicability_complete=authority['complete'],
    corpus={k:audit[k] for k in ['canonical_records','derived_overlay_count','planned_derived_records','required_references',
        'unresolved','incomplete','ambiguous','invalid_references','incomplete_queries','invalid_queries','distinguishability','query_count']},
    reference_measurement_phase=audit['reference_measurement_phase'],expanded_references=audit['expanded_references'],
    expanded_occurrences_witnesses=audit['expanded_occurrences_witnesses'],
    structural_validation_findings=len(audit['validation_findings']),structural_coverage_status_counts=counts,
    completion_phrase='KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED',
    inputs={str(path.relative_to(ROOT)).replace('\\','/'):sha(path) for path in [audit_path,authority_path,blocker_path,profile_path,correction_path]},
    execution_required_for_acceptance=False,
    limitation='This review deliberately retains incomplete ordinary work. It is not the authority-only completion state and does not expose a production accepted publication.')
write(OUT/'publication-review.json',report)
print(json.dumps(dict(criteria=criteria,coverage=counts,authority_conflicts=report['authority_conflicts']),ensure_ascii=False))
if '--require-publication' in sys.argv:sys.exit(0 if accepted else 1)
