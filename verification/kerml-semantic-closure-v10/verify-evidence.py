"""Cross-check v10 identities and reject misleading completion claims offline."""
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[1]
def read(path):return json.loads(path.read_text(encoding='utf8'))
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
profile=read(ROOT/'standards/kerml-1.0-operational-profile-v7.json')
assert profile['profile_id']=='agentique-kerml-1.0-operational/7'
assert profile['default_profile']=='agentique-kerml-1.0-operational/2'
assert not profile['accepted_publication']
for item in [profile['extends'],profile['authority'],*profile['manifests']]:
    path=item.get('path',item.get('manifest'))
    assert sha(ROOT/path)==item['sha256'],path
authority=read(OUT/'authority-packet.json')
selector=read(OUT/'selector-verification.json')
assert selector['all_exact_order_and_identity_comparisons_passed']
assert selector['feature_count']==11632 and selector['selected_count']==152
assert selector['synthetic_cases']==120 and len(selector['profiles'])==8
assert len(selector['witnesses'])==2 and all(w['v7'] is None and w['published_through_v6'] for w in selector['witnesses'])
sweep=read(OUT/'authority-applicability.json')
assert sweep['constraints_visited']==258 and sweep['issues_visited']==410 and sweep['derived_members_visited']==218
assert sweep['traversal_complete'] and not sweep['early_termination_on_conflict']
assert not sweep['complete']
scope=read(OUT/'additional-issue-scope-review.json')
assert scope['issues_reviewed']==63 and scope['issue_inventory_sha256']==sha(OUT/'issue-inventory.json')
assert not scope['authority_applicability_closed'] and not scope['structural_coverage_closed']
assert all(not r['structural_constraints_deferred'] and not r['authorizes_correction'] for r in scope['reviews'])
populations=read(OUT/'source-structural-populations.json')
assert len(populations['source_documents'])==36 and populations['source_inventory_complete']
assert not populations['structural_semantics_complete'] and not populations['semantic_publication_accepted']
assert populations['counts']['feature_values']==1144 and populations['counts']['feature_chains']==123
parameter=read(OUT/'parameter-position-authority.json')
assert parameter['issue_antecedent_exercised'] and parameter['local_positional_requirement_proven']
assert not parameter['correction_applied'] and not parameter['full_structural_conformance_proven']
assert parameter['explicit_target']['id']!=parameter['published_additional_positional_target']['id']
assert all(row['direction_predicate_satisfied'] for row in parameter['direction_checks'])
blockers=read(ROOT/'standards/kerml-1.0-operational-authority-blockers.json')
assert len(blockers['blockers'])==sweep['independently_proven_blockers']==4
imports=read(OUT/'import-authority.json')
assert imports['local_contradiction_independently_proven'] and len(imports['witnesses'])==5
assert not imports['correction_applied']
assert blockers['blockers'][-1]['complete_local_structural_facts']['independent_packet_sha256']==sha(OUT/'import-authority.json')
indexes=read(OUT/'index-authority.json')
assert indexes['source_index_expressions']==23 and indexes['positive_guard_agreement_proofs']==7
assert indexes['remaining_negative_closure_proofs']==16 and not indexes['authority_applicability_closed']
assert not indexes['additional_correction_applied']
constructors=read(OUT/'supplemental-authority-review.json')['constructors']
assert constructors['formal_prose'] and constructors['formal_ocl']=='TBD'
assert len(constructors['witnesses'])==2
assert all(w['pinned_expression'] and w['inherited_default_candidates'] and not w['reference_result_owned_binding_connectors'] for w in constructors['witnesses'])
assert not constructors['independently_proven_authority_contradiction']
assert not blockers['unauthorized_corrections_applied']
assert all(b['independent_local_reproduction'] and not b['authorized_correction'] and not b['execution_required'] for b in blockers['blockers'])
preserved=read(OUT/'preservation.json')
assert preserved['unchanged'] and not preserved['historical_manifests_and_evidence_modified']
assert len(preserved['inherited_acquisition_discrepancies'])==9
restoration=read(OUT/'historical-standards-output-restoration.json')
assert sha(ROOT/restoration['restored_path'])==restoration['base_and_restored_sha256']
assert sha(ROOT/restoration['preserved_new_run'])==restoration['new_run_sha256']
runtime=read(OUT/'final-verification-plan-order/results.json')
runtime_rows=[r for r in runtime if '--require-runtime' in r['command']]
assert len(runtime_rows)==2 and all(r['exitCode']==0 for r in runtime_rows)
required=['cargo fmt --all -- --check','cargo clippy --workspace --all-targets -- -D warnings','cargo test --workspace',
    'cargo run --locked --offline -p agq-metamodel-gen -- --check','npm run standards:check','npm run check','npm run build','npm test','npm run test:e2e']
assert all(any(r['command']==command and r['exitCode']==0 for r in runtime) for command in required)
if '--with-corpus' in sys.argv:
    audit=read(OUT/'full-v7-complete.json');review=read(OUT/'publication-review.json')
    diagnostics=read(OUT/'audit-diagnostics.json')
    assert diagnostics['audit_sha256']==sha(OUT/'full-v7-complete.json')
    assert diagnostics['distinct_diagnostics']==len(audit['validation_findings'])==1925
    assert diagnostics['findings_waived']==0 and diagnostics['additional_authority_conflicts_proven']==0
    expanded=read(OUT/'expanded-selector-verification.json')
    delta=read(OUT/'audit-source-delta.json')
    assert delta['canonical_graph_code_unchanged'] and delta['audit_already_normalizes_all_producer_plans_through_merge']
    assert delta['historical_profile_output_sequences_unchanged'] and len(delta['exact_source_deltas'])==2
    assert delta['authority_register_delta']['added_blocker']=='KLCV10-F-004'
    assert delta['authority_register_delta']['graph_and_query_execution_unchanged']
    assert audit['authority_blockers']==4
    assert all(d['reconstructed_audit_input_matches'] and sha(ROOT/d['path'])==d['final_sha256'] for d in delta['exact_source_deltas'])
    assert expanded['audit_sha256']==sha(OUT/'full-v7-complete.json')
    assert expanded['all_comparisons_passed'] and len(expanded['canonical_expanded_witnesses'])==2
    assert all(w['selected'] is None and w['original_order_preserved'] and w['value_expressions'] and w['binding_connectors'] for w in expanded['canonical_expanded_witnesses'])
    assert audit['kernel_storage_valid'] and audit['overlay_error'] is None
    assert audit['derived_overlay_count']==audit['planned_derived_records']
    assert audit['query_count']>0 and audit['expanded_references']['count']==4000
    assert len(audit['expanded_occurrences_witnesses'])==2
    assert all(w['selected'] is None and w['completeness']=='Complete' for w in audit['expanded_occurrences_witnesses'])
    assert not review['semantic_publication_accepted'] and not review['ordinary_structural_implementation_complete']
    assert review['accepted_snapshot_ids']==[]
    assert review['completion_phrase']=='KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED'
    assert not all(review['criteria'].values())
print('V7 manifest/authority identities, eight-profile selector, aggregate blockers, preservation, and both runtime gates agree. No incomplete review is represented as accepted publication.')
