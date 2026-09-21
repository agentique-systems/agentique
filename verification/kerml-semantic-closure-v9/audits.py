"""Explicit incomplete publication accounting at the new authority boundary."""
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
measurement=json.loads((OUT/'measurement.json').read_text())
conflict=json.loads((OUT/'cross-feature-authority-conflict.json').read_text())
assert conflict['disposition']=='IndependentAuthorityConflict'
assert all(conflict['stop_policy'][k] for k in ['exercised_by_pinned_corpus',
    'independent_complete_local_reproduction','not_covered_by_v1_v6','materially_different_canonical_choice'])
assert not conflict['stop_policy']['unfinished_agentique_semantics']
manifest=json.loads((ROOT/'standards/kerml-1.0-operational-profile-v6.json').read_text())
for row in manifest['manifests']:
    assert hashlib.sha256((ROOT/row['path']).read_bytes()).hexdigest()==row['sha256']
reference=json.loads((ROOT/manifest['manifests'][1]['path']).read_text())
assert hashlib.sha256((ROOT/reference['authority_record']).read_bytes()).hexdigest()==reference['authority_sha256']
assert not reference['feature_chain_expression_included']
matrices=json.loads((OUT/'reference-result-matrices-final/result.json').read_text())
assert matrices['exit_code']==0
gates=[dict(gate=n,status=status,evidence=evidence) for n,status,evidence in [
    (0,'Verified','reference-binding-authority.json'),
    (1,'VerifiedExcludedDirectFeatureChainExpression','reference-binding-authority.json#/feature_chain_scope'),
    (2,'Registered','../../standards/kerml-1.0-operational-profile-v6.json'),
    (3,'ImplementedFixtureVerified','reference-result-matrices-final/result.json'),
    (4,'ImplementedFixtureVerified','reference-result-matrices-final/result.json'),
    (5,'ImplementedFixtureVerified','reference-binding-authority.json#/default_featuring_type'),
    (6,'FixtureMatrixPassed','reference-result-matrices-final/result.json'),
    (7,'ImplementedFixtureVerified','reference-result-matrices-final/result.json'),
    (8,'ImplementedFixtureVerified','reference-result-matrices-final/result.json'),
    (9,'ImplementedFixtureVerified','reference-result-matrices-final/result.json'),
    (10,'ImplementedFixtureVerified','reference-result-matrices-final/result.json'),
    (11,'ImplementedFixtureVerifiedPublishedArrayGuardRetained','reference-result-matrices-final/result.json'),
    (12,'ImplementedFixtureVerifiedNoExecution','reference-result-matrices-final/result.json'),
    (13,'FixtureMatrixPassed','seven-profile-lineage/result.json'),
    (14,'IndependentAuthorityConflict','cross-feature-authority-conflict.json'),
    (15,'OrdinaryWorkOutstandingAtAuthorityStop','baseline-quality.json'),
    (16,'OrdinaryWorkOutstandingAtAuthorityStop','baseline-quality.json'),
    (17,'OrdinaryWorkOutstandingAtAuthorityStop','baseline-quality.json'),
    (18,'CorpusClosureOutstandingAtAuthorityStop','feature-chain-structural-audit.json'),
    (19,'CorpusClosureOutstandingAtAuthorityStop','positional-redefinition-audit.json'),
    (20,'NotClosed','authority-applicability.json'),
    (21,'NoStructuralExecutionDeferrals','authority-applicability.json'),
    (22,'AcceptanceOperationNotFinalized','strict-publication-status.json'),
    (23,'NotAccepted','strict-publication-status.json'),
    (24,'NotRegeneratedWithoutAcceptance','strict-publication-status.json'),
    (25,'NotIssuedWithoutAcceptance','strict-publication-status.json'),
    (26,'NotClaimedWithoutAcceptedLibraries','strict-publication-status.json'),
    (27,'FullAcceptanceStressNotClaimed','strict-publication-status.json'),
    (28,'Documented','../../docs/adr/0019-operational-reference-binding-correction.md')]]
reports={
    'gate-status.json':dict(format='agentique-v9-gates/1',authority_stop='KLCV9-F-001',gates=gates),
    'feature-chain-structural-audit.json':dict(format='agentique-v9-chain-audit/1',
        contextual_result_primitive_implemented=True,ordered_chain_preserved=True,raw_identity_retained=True,
        fixture_matrix='reference-result-matrices-final/result.json',full_corpus_complete=False,
        reason='Complete corpus expansion remains required after the independently proven cross-feature authority decision.'),
    'positional-redefinition-audit.json':dict(format='agentique-v9-positional-audit/1',
        existing_tests='https://github.com/agentique-systems/agentique/blob/ed2cf3a9cc086e165c075c591a3892472250e9b7/verification/kerml-semantic-closure-v9/full-matrix-1/rust-tests.txt',full_corpus_differential_audit_complete=False,
        reference_local_typing_complete=True,authority_proof='cross-feature-authority-conflict.json'),
    'distinguishability-audit.json':dict(format='agentique-v9-distinguishability/1',
        findings=measurement['namespace_distinguishability_findings'],source='baseline-quality.json',
        zero_gate_passed=measurement['namespace_distinguishability_findings']==0,
        disposition='Ordinary Triggers inference work; no additional library erratum adopted.'),
    'strict-publication-status.json':dict(format='agentique-v9-publication-status/1',
        profile=manifest['profile_id'],profile_registered=True,producer_fixtures_verified=True,
        authority_conflict='KLCV9-F-001 / KERML11-1',
        current_counts_scope=measurement['scope'],
        required_references_complete=False,structural_coverage_closed=False,authority_applicability_closed=False,
        complete_corpus_v6_expansion_verified=False,acceptance_operation_finalized=False,
        semantic_publication_accepted=False,accepted_snapshot_ids=[],accepted_bindings_regenerated=False,
        loaded_kerml_standard_libraries_available=False,authored_accepted_library_consumption=False,
        full_resource_acceptance=False,executable_semantics_implemented=False,
        structural_rules_deferred_as_execution=0,
        kernel_snapshot_validity_is_semantic_acceptance=False,
        completion='KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED')}
for name,report in reports.items():
    encoded=json.dumps(report,indent=2,ensure_ascii=False)+'\n';path=OUT/name
    if '--check' in sys.argv:
        assert path.read_text(encoding='utf8')==encoded
    else:
        path.write_text(encoded,encoding='utf8',newline='\n')
print('V6 producer fixtures verified; KLCV9-F-001 independently reproduced; corpus closure and semantic publication are not accepted.')
if '--gate' in sys.argv:
    sys.exit(1)
