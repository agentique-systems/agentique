"""Build an honest milestone summary from completed audit artifacts, offline."""
import collections
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def read(name):
    return json.loads((HERE/name).read_bytes())


def artifact(name):
    path = HERE/name
    return dict(path=name, sha256=hashlib.sha256(path.read_bytes()).hexdigest())


audits = {name: read(f'{label}/obligations.json') for name, label in
          [('published', 'published-2'), ('operational_v1', 'v1-2'), ('operational_v2', 'obligations-7')]}
quality = read('quality-4/library-quality.json')
comparison = read('obligations-7/reference-comparison.json')
profiles = {}
for profile, audit in audits.items():
    refs = audit['references']
    # The canonical audit preserves the actual candidate list and completeness;
    # do not relabel every unresolved endpoint as a standards conflict.
    profiles[profile] = dict(profile_id=audit['baseline_profile'], rule_version=audit['rule_version'],
        reference_obligations=len(refs), mandatory_lower_bounds=len(audit['structural_obligations']),
        incomplete_or_invalid_structural_queries=len(audit['semantic_queries']),
        strict_kernel_snapshot_attempt=audit['strict_publication_attempt'],
        accepted_semantic_publication=False,
        reference_diagnostics=dict(collections.Counter(d['code'] for r in refs for d in r['result']['diagnostics'])))

v2 = audits['operational_v2']
remaining = []
for row in v2['references']:
    remaining.append(dict(source_reference=row['source'], source=row['document'], range=row['source_range'],
                          completeness=row['result']['completeness'], candidates=row['result']['candidates'],
                          diagnostics=row['result']['diagnostics'],
                          classification='ordinary structural implementation obligation requiring individual review; not automatically KERML11-140'))
five = []
for row in v2['redefinition_resolutions']:
    path = row['source_declaration']
    if ('beforeTimeSlice::monitoredFeature' in path or 'afterSnapshot::monitoredFeature' in path
        or path.startswith('Occurrences::InsideOf::')
        or (path.startswith('Observation::ChangeMonitor::AssignObservations::monitor::startingAt::')
            and row['source'].strip() == 'observations')):
        searches = [v2['evidence_dictionary'][i] for i in row['result']['search_dependencies']]
        lexical = any('RedefinitionScope' in s and 'LexicalContaining' in s for s in searches)
        five.append(dict(source_declaration=path, source_reference=row['source'], targets=row['targets'],
                         completeness=row['result']['completeness'],
                         rule_path='operational lexical-containing search' if lexical else 'inherited general-type search',
                         classification=('AGQ-KERML10-002 lexical rule, justified by its recorded scope searches' if lexical
                                         else 'ordinary binary-association/inherited-end support plus v2 independent general-scope search; no lexical retry'),
                         scope_dependencies=[s for s in searches if 'RedefinitionScope' in s],
                         rules=row['result']['rules']))

documents = quality['documents']
semantic_codes = collections.Counter(d['code'] for row in documents
                                    for d in row['diagnostics_by_category']['KerML_semantic']['query_diagnostics'])
report = dict(format='agentique-kerml-name-resolution-publication-v4-summary/1',
    completion='KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED',
    stop_reason='KNRV4-F-001: separately proven pinned Objects library distinguishability conflict',
    source_conflict=artifact('separate-authority/authority-packet.json'),
    profile_results=profiles, original_five_redefinition_cases=five,
    remaining_reference_classification=remaining,
    reference_xmi=dict(exact_kerml11_140_matches=comparison['exact_kerml11_140_matches'],
                       assertions_compared=len(comparison['comparisons']),
                       source_target_sets_compared=len(comparison['source_target_sets']),
                       unmatched_assertions=comparison['unmatched_comparisons'],
                       unmatched_target_sets=comparison['unmatched_target_sets'],
                       target_identity_mismatches=comparison['target_identity_mismatches'],
                       incomplete_reference_answers=comparison['incomplete_reference_answers'],
                       authority='non-normative implementation/interchange evidence'),
    structural_quality=dict(documents=len(documents),
        unresolved=sum(r['unresolved_count'] for r in documents),
        ambiguous=sum(r['ambiguous_count'] for r in documents),
        incomplete_references=sum(r['incomplete_reference_count'] for r in documents),
        mismatched_endpoints=sum(r['mismatched_endpoint_count'] for r in documents),
        incomplete_queries=sum(r['incomplete_query_count'] for r in documents),
        invalid_queries=sum(r['invalid_query_count'] for r in documents),
        diagnostics=dict(semantic_codes),
        finding_disposition='KQ_IMPLIED_NAMING_ORDER and expression-result findings remain ordinary structural implementation/validation work. Only the independently checked Objects conflict is classified as KNRV4-F-001; other distinguishability findings require individual review.',
        unevaluated_expressions_and_functions=sum(r['unevaluated_expression_count'] for r in documents),
        complete_constraint_inventory=False, accepted=False),
    accepted_library_facade=False, accepted_bindings_regenerated=False,
    binding_manifest_disposition='Historical 22-entry candidate manifest preserved; runtime now needs 23 roles. Existing stale-manifest check fails. No candidate IDs presented as accepted bindings.',
    authored_profile_matrix='authored-3/results.json', authored_accepted_library_consumption=False,
    verification=dict(normal_matrix='final-1/results.json', rust_recheck='repair-2/results.json',
                      rustdoc='docs-2/results.json', browser_recheck='browser-2/results.json',
                      profiles='profiles-2/results.json', synthetic_matrix='matrix-8/results.json',
                      authority_witness='conflict-3/results.json', final_lint='lint-5/results.json',
                      final_integrity='review-2/results.json', product_matrix='product-1/results.json',
                      headless_demo='demo-2/results.json', raw_conformance='strict-1/results.json'),
    artifacts=[artifact(n) for n in ['obligations-7/obligations.json','published-2/obligations.json',
                                      'v1-2/obligations.json','quality-4/library-quality.json',
                                      'obligations-7/reference-comparison.json']])
with (HERE/'summary.json').open('x', encoding='utf-8') as stream:
    json.dump(report, stream, indent=2)
    stream.write('\n')
print(json.dumps(dict(profile_results=profiles, structural_quality=report['structural_quality'],
                     compared_original_cases=len(five), reference_xmi=report['reference_xmi'])))
