"""Independently replay the frozen pilot selector on expanded witness sequences.

No Rust query is called. The source ownership sequence is recovered from the
content-checked declaration archive; metaclass conformance comes from pinned XMI.
The expanded sequence is the audit's canonical ordered membership projection.
"""
import json
import sys
from corpus import OUT, Pinned, conforms, digest

p = Pinned()
audit_path = OUT / 'full-v7-complete.json'
source_path = OUT / 'selector-verification.json'
audit = json.loads(audit_path.read_text(encoding='utf8'))
source = json.loads(source_path.read_text(encoding='utf8'))
assert audit['profile'] == 'agentique-kerml-1.0-operational/7'
assert audit['kernel_storage_valid'] and audit['overlay_error'] is None
assert audit['derived_overlay_count'] == audit['planned_derived_records'] > 0
expected = {r['feature']: r for r in source['witnesses']}
actual = {r['feature']: r for r in audit['expanded_occurrences_witnesses']}
assert set(actual) == set(expected) and len(actual) == 2
rows = []
for identity, row in actual.items():
    assert row['path'] == expected[identity]['path']
    assert p.scalar(identity, 'isEnd') is True
    assert p.is_kind(p.parents[identity], 'FeatureMembership')
    assert p.is_kind(p.owner(identity), 'Type')
    original = [(m, v) for m, v in p.members(identity)]
    sequence = row['owned_membership_sequence']
    assert len({r['membership'] for r in sequence}) == len(sequence)
    retained = [(r['membership'], r['member']) for r in sequence if r['membership'] in p.records]
    assert retained == original, (identity, 'source ownership order changed')
    selected = None
    excluded = []
    binding_members = []
    value_members = []
    for candidate in sequence:
        membership = conforms(candidate['membership_metaclass'])
        member = conforms(candidate['member_metaclass'])
        reasons = []
        if 'OwningMembership' not in membership:
            reasons.append('not an owned member')
        reasons.extend(sorted(membership.intersection({'FeatureMembership', 'FeatureValue'})))
        reasons.extend(sorted(member.intersection({'Multiplicity', 'MetadataFeature', 'BindingConnector'})))
        if 'Feature' not in member:
            reasons.append('not a Feature')
        if 'FeatureValue' in membership:
            value_members.append(candidate['member'])
        if 'BindingConnector' in member:
            binding_members.append(candidate['member'])
        if reasons:
            excluded.append(dict(**candidate, exclusion_reasons=reasons))
        elif selected is None:
            selected = candidate['member']
    assert value_members and binding_members, (identity, 'expanded infrastructure not exercised')
    assert expected[identity]['published_through_v6'] in value_members
    assert row['completeness'] == 'Complete' and row['selected'] is selected is None
    rows.append(dict(feature=identity, path=row['path'], selected=selected,
                     original_order_preserved=True, exclusions=excluded,
                     value_expressions=value_members, binding_connectors=binding_members))
result = dict(format='agentique-v10-expanded-selector-verification/1',
              audit_sha256=digest(audit_path), source_verification_sha256=digest(source_path),
              pilot_commit='553cf8205c19241c9127ab264f8372f5b58d3895',
              canonical_expanded_witnesses=rows, all_comparisons_passed=True,
              semantic_publication_accepted=False,
              scope='Both exact expanded Occurrences witnesses. The separate source verifier covers all source Features and the eight-profile matrix.')
encoded = json.dumps(result, indent=2, ensure_ascii=False) + '\n'
path = OUT / 'expanded-selector-verification.json'
if '--check' in sys.argv:
    assert path.read_text(encoding='utf8') == encoded
else:
    path.write_text(encoded, encoding='utf8', newline='\n')
print('Both expanded Occurrences witnesses independently select none; FeatureValue expressions and BindingConnectors are excluded, and source membership order is preserved.')
