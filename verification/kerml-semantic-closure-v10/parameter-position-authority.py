"""Independent local positional calculation for the exact KERML11-79 witness.

Endpoint identities are cross-checked against the retained source-refinement
audit; this script does not claim a second complete name resolver. Ownership
order and parameter selection are independently reconstructed from pinned input.
"""
import json
import re
import sys
from corpus import *

p = Pinned()
m = reference()
audit_path = OUT / 'full-v7-producers.json'
audit = json.loads(audit_path.read_text(encoding='utf8'))
assert audit['profile'] == 'agentique-kerml-1.0-operational/7'
answers = {row['relationship']: row for row in audit['reference_answers']}
golden_path = ROOT / 'standards/generated/kerml-1.0/full.golden.json'
golden = json.loads(golden_path.read_text(encoding='utf8'))
literals = {int(row['descriptor_id'].replace('-', ''), 16): row['entity']['name']
            for cls in golden['classifiers'].values() for row in cls['literals']}

def direction(feature):
    value = p.scalar(feature, 'direction')
    if value is None: return None
    encoded = re.fullmatch(r'Scalar\(Enumeration\(EnumerationLiteralId\((\d+)\)\)\)', value)
    assert encoded, value
    return literals[int(encoded[1])]

def parameters(owner):
    return [(membership, feature) for membership, feature in p.members(owner)
            if p.is_kind(membership, 'FeatureMembership')
            and not p.is_kind(membership, 'ReturnParameterMembership')
            and direction(feature) is not None]

def endpoint(relationship, expected_name):
    row = answers[relationship]
    assert row['name'] == [expected_name] and row['completeness'] == 'Complete'
    assert len(row['candidates']) == 1
    return row['candidates'][0], row

owner = p.find('TransitionPerformances::TransitionPerformance::accept')
general = p.find('Transfers::AcceptPerformance')
assert p.is_kind(owner, 'Step') and not p.is_kind(owner, 'InvocationExpression')
assert p.is_kind(general, 'Behavior')
typed = [r for r in p.refs(owner, 'ownedRelationship') if p.is_kind(r, 'FeatureTyping')]
assert len(typed) == 1
typed_target, typing_answer = endpoint(typed[0], 'AcceptPerformance')
assert typed_target == general
local = parameters(owner)
inherited = parameters(general)
assert len(local) == 1 and len(inherited) == 2
membership, feature = local[0]
payload, receiver = [f for _, f in inherited]
assert p.scalar(payload, 'declaredName') == 'payload'
assert p.scalar(receiver, 'declaredName') == 'receiver'
assert p.records[feature]['source']['text'] == 'in feature redefines receiver = triggerTarget;'
redefinitions = [r for r in p.refs(feature, 'ownedRelationship') if p.is_kind(r, 'Redefinition')]
assert len(redefinitions) == 1
explicit, redefinition_answer = endpoint(redefinitions[0], 'receiver')
assert explicit == receiver
assert not p.scalar(redefinitions[0], 'isImplied')
index = [f for _, f in local].index(feature)
required = inherited[index][1]
assert required == payload and required != explicit
assert [direction(f) for f in [feature, payload, receiver]] == ['in', 'inout', 'in']

rules = [r for r in inventory['members'] if r['name'] in
         ['checkFeatureParameterRedefinition', 'validateRedefinitionDirectionConformance']]
assert len(rules) == 2
formal = next(r for r in rules if r['name'] == 'checkFeatureParameterRedefinition')
assert any('supertype.ownedFeature->select(direction <> null)' in b.get('body', '') for b in formal['bodies'])
direction_checks = []
for target in [payload, receiver]:
    required_direction = direction(target)
    actual_direction = direction(feature)
    valid = (actual_direction == required_direction if required_direction in ['in', 'out']
             else actual_direction is not None if required_direction == 'inout' else True)
    assert valid
    direction_checks.append(dict(target=p.brief(target), target_direction=required_direction,
                                 redefining_direction=actual_direction, direction_predicate_satisfied=valid))

ref = m.find('TransitionPerformances::TransitionPerformance::accept')
reference_parameters = [brief(m, v) for r, v in members(m, ref)
                        if kind(m, r, 'FeatureMembership') and v.get('direction')
                        and not kind(m, r, 'ReturnParameterMembership')]
assert not reference_parameters
issue = next(i for i in json.loads((OUT/'issue-inventory.json').read_text(encoding='utf8'))['issues']
             if i['key'] == 'KERML11-79')
report = dict(format='agentique-v10-parameter-position-authority/1', issue=issue,
              source_archive_sha256=digest(p.archive), descriptor_identity_map_sha256=digest(golden_path),
              source_refinement_audit_sha256=digest(audit_path), formal=rules,
              owner=p.brief(owner), direct_general=p.brief(general),
              canonical_endpoint_cross_checks=dict(typing=typing_answer, explicit_redefinition=redefinition_answer),
              local_owned_parameter_sequence=[dict(membership=p.brief(r), feature=p.brief(f)) for r, f in local],
              general_owned_parameter_sequence=[dict(membership=p.brief(r), feature=p.brief(f)) for r, f in inherited],
              zero_based_local_position=index, explicit_target=p.brief(explicit),
              published_additional_positional_target=p.brief(required),
              direction_checks=direction_checks,
              reference=dict(owner=brief(m, ref), owned_parameters=reference_parameters,
                             source_change_adopted=False),
              issue_antecedent_exercised=True, local_positional_requirement_proven=True,
              independently_proven_additional_authority_conflict=False,
              full_structural_conformance_proven=False, correction_applied=False,
              conclusion='The exact pinned first local parameter explicitly redefines receiver and is also required to redefine payload at position one. Direction conformance permits both redefinitions. This reproduces the issue\'s unintended positional consequence; that consequence alone is not a contradiction between the two checked structural rules. Complete dependent typing/domain/suppression validation is still required, and KLCV10-F-002 already blocks the accept multiplicity domain.')
encoded = json.dumps(report, indent=2, ensure_ascii=False) + '\n'
path = OUT / 'parameter-position-authority.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8') == encoded
else: path.write_text(encoded, encoding='utf8', newline='\n')
print('KERML11-79 exact owned parameter order independently reproduced; required extra payload redefinition and compatible directions confirmed. No additional correction or unproved authority blocker introduced.')
