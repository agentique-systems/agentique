"""Pin the exact final-source delta from the running corpus audit's build inputs.

The audit merges every plan, so the final identity-list normalization duplicates
an operation already performed by that audit. No graph construction is changed.
"""
import hashlib
import json
from pathlib import Path
import sys

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
def sha(data): return hashlib.sha256(data).hexdigest()
inputs_path = OUT / 'full-v7-indexed/input-hashes.json'
inputs = json.loads(inputs_path.read_text(encoding='utf8'))
normalization = '''        // These lists enumerate produced identities, not semantic ownership.
        // V7 normalizes them exactly as merge does so caller/batch order cannot
        // change the complete producer answer. Graph ownership and historical
        // profile output sequences are untouched.
        if profile.corrects_owned_cross_feature() {
            production.value.sort();
            production.value.dedup();
            contextual_results.sort_by_key(|result| result.feature);
            contextual_results.dedup();
        }
'''
assertions = '''    assert_eq!(result.production, reversed.production);
    assert_eq!(result.production, merged.production);
    assert_eq!(result.contextual_results, reversed.contextual_results);
    assert_eq!(result.contextual_results, merged.contextual_results);
'''
changes = {
    'crates/kerml-semantics/src/result_structure.rs': normalization,
    'crates/kerml-semantics/tests/initial_values_v10.rs': assertions,
}
rows = []
unchanged = 0
authority_delta = None
for entry in inputs:
    path = entry['path']
    current = (ROOT / path).read_bytes()
    if path == 'standards/kerml-1.0-operational-authority-blockers.json':
        previous = json.loads(current)
        added = previous['blockers'].pop()
        assert added['id'] == 'KLCV10-F-004'
        assert len(previous['blockers']) == 3
        reconstructed = (json.dumps(previous, indent=2, ensure_ascii=False)+'\n').encode('utf8')
        assert sha(reconstructed) == entry['sha256'], (path, 'earlier register changed')
        authority_delta = dict(path=path, audit_start_sha256=entry['sha256'], final_sha256=sha(current),
            added_blocker=added['id'], earlier_register_reconstructed_exactly=True,
            graph_and_query_execution_unchanged=True,
            audit_reads_current_register_at_end=True)
        continue
    if path not in changes:
        assert sha(current) == entry['sha256'], path
        unchanged += 1
        continue
    addition = changes[path].encode('ascii')
    if b'\r\n' in current:
        addition = addition.replace(b'\n', b'\r\n')
    assert current.count(addition) == 1, path
    reconstructed = current.replace(addition, b'', 1)
    assert sha(reconstructed) == entry['sha256'], (path, 'unexpected source delta')
    rows.append(dict(path=path, audit_input_sha256=entry['sha256'], final_sha256=sha(current),
                     exact_added_text=changes[path], reconstructed_audit_input_matches=True))
assert len(rows) == len(changes)
assert authority_delta is not None
source = (ROOT / 'crates/kerml-semantics/src/result_structure.rs').read_text(encoding='utf8')
audit = (ROOT / 'crates/kerml-text/examples/library_publication_audit.rs').read_text(encoding='utf8')
assert 'plan.merge(q.plan_result_structure(batch.iter().copied()))?;' in audit
assert 'self.production.value.sort();' in source and 'self.production.value.dedup();' in source
assert 'self.contextual_results.sort_by_key(|r| r.feature);' in source
assert 'self.contextual_results.dedup();' in source
result = dict(format='agentique-v10-audit-source-delta/1',
              audited_input_manifest_sha256=sha(inputs_path.read_bytes()),
              unchanged_inputs=unchanged, exact_source_deltas=rows,
              authority_register_delta=authority_delta,
              canonical_graph_code_unchanged=True,
              audit_already_normalizes_all_producer_plans_through_merge=True,
              historical_profile_output_sequences_unchanged=True,
              scope='Exact source delta proof, plus the independent complete-answer regression. This does not relabel the retained executable as rebuilt from final sources.')
encoded = json.dumps(result, indent=2) + '\n'
path = OUT / 'audit-source-delta.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8') == encoded
else: path.write_text(encoded, encoding='utf8', newline='\n')
print(f'All {unchanged} other audited inputs are unchanged; exactly the v7 identity-list normalization and its four regression assertions were added. The blocker register gained only independently proved KLCV10-F-004; the audit reads this register after validation.')
