"""Recount completed expanded diagnostics without waiving partial semantics."""
import collections
import json
import sys
from corpus import OUT, Pinned, digest, inventory

p = Pinned()
audit_path = OUT / 'full-v7-complete.json'
audit = json.loads(audit_path.read_text(encoding='utf8'))
groups = collections.defaultdict(list)
for finding in audit['validation_findings']:
    assert finding['subject'] in p.records
    groups[finding['code']].append(dict(**finding, pinned_subject=p.brief(finding['subject'])))
expected = {'KQ_AMBIGUOUS_IMPLIED_NAME': 1, 'KQ_FORMAL_ANTECEDENT_TYPE_CLOSURE': 2,
            'KQ_VARIABLE_FEATURING': 212, 'validateElementIsImpliedIncluded': 1706,
            'validateNamespaceDistinguishibility': 4}
assert {k: len(v) for k, v in groups.items()} == expected
assert sum(expected.values()) == 1925
formal = [r for r in inventory['members']
          if r['owner'] == 'Element' and r['name'] in ['isImpliedIncluded', 'validateElementIsImpliedIncluded']]
assert len(formal) == 2
namespaces = sorted({r['subject'] for r in groups['validateNamespaceDistinguishibility']})
assert namespaces == ['4a74747a-c385-5a89-89c8-be2fadaa9ecb', '518b9181-2fd8-5bfa-953c-3292058bc119']
assert groups['KQ_AMBIGUOUS_IMPLIED_NAME'][0]['pinned_subject']['path'] == 'VectorFunctions::sum0::s'
report = dict(format='agentique-v10-expanded-diagnostic-review/1',
              audit_sha256=digest(audit_path), counts=expected, distinct_diagnostics=1925,
              query_count=audit['query_count'], incomplete_queries=audit['incomplete_queries'],
              invalid_queries=audit['invalid_queries'], formal_implied_inclusion=formal,
              findings_by_code=dict(groups), distinguishability_namespace_ids=namespaces,
              interpretation={
                  'validateElementIsImpliedIncluded': 'The staging overlay has partial implied relationships. Published Element semantics permit all required implied relationships or none. Setting isImpliedIncluded to true without complete producer closure would claim facts not established by this audit; these 1706 findings remain failures.',
                  'KQ_VARIABLE_FEATURING': '212 pinned variable-feature domain answers remain unsupported; they are ordinary structural obligations, not deferred execution.',
                  'KQ_FORMAL_ANTECEDENT_TYPE_CLOSURE': 'The two effective-owner typing gaps also prevent ten required reference answers from being Complete.',
                  'validateNamespaceDistinguishibility': 'Four member-pair findings occur in the two historical Triggers namespaces: observer and signal each collide in both. The earlier source-only count is not substituted for this expanded count.',
                  'KQ_AMBIGUOUS_IMPLIED_NAME': 'The expanded query reports result/u implied-name alternatives for the anonymous plus-function reference under VectorFunctions::sum0::s. Complete positional/redefinition order must be independently traced before this can be classified as an implementation defect or a further authority conflict. This diagnostic alone does not prove a fifth blocker.',
              }, additional_authority_conflicts_proven=0, findings_waived=0,
              ordinary_structural_implementation_complete=False, semantic_publication_accepted=False)
encoded = json.dumps(report, indent=2, ensure_ascii=False) + '\n'
path = OUT / 'audit-diagnostics.json'
if '--check' in sys.argv:
    assert path.read_text(encoding='utf8') == encoded
else:
    path.write_text(encoded, encoding='utf8', newline='\n')
print('Recounted all 1925 expanded diagnostics, traced every subject to pinned source, and retained all failures without asserting implied closure or a fifth authority blocker.')
