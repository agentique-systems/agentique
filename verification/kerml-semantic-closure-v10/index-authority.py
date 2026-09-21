"""Pinned IndexExpression operand proof, distinct from reference-model behavior.

Positive specialization paths suffice to establish both guards are true. A
negative result from the explicitly reconstructed edges is deliberately not
treated as complete implied-typing closure.
"""
import collections
import json
import sys
from corpus import *

p = Pinned()
audit_path = OUT/'full-v7-producers.json'
answers = {row['relationship']: row for row in json.loads(audit_path.read_text(encoding='utf8'))['reference_answers']}
edges = collections.defaultdict(list)

def endpoint(relationship):
    row = answers[relationship]
    assert row['completeness'] == 'Complete' and len(row['candidates']) == 1, row
    return row['candidates'][0]

for element in p.records:
    if not p.is_kind(element, 'Type'): continue
    for relationship in p.refs(element, 'ownedRelationship'):
        if not p.is_kind(relationship, 'Specialization') or p.is_kind(relationship, 'CrossSubsetting'): continue
        if relationship in answers:
            target = endpoint(relationship)
        else:
            properties = ['type','redefinedFeature','referencedFeature','subsettedFeature','superclassifier','general']
            targets = [t for prop in properties for t in p.refs(relationship, prop)]
            assert len(targets) == 1, (relationship, targets)
            target = targets[0]
        edges[element].append((target, relationship))

def path_to(start, target):
    pending = collections.deque([(start, [])])
    seen = set()
    while pending:
        element, path = pending.popleft()
        if element == target: return path
        if element in seen: continue
        seen.add(element)
        pending.extend((general, path+[dict(specific=element, relationship=r, general=general)])
                       for general, r in edges[element])
    return None

def result(expression):
    results = [v for r, v in p.members(expression) if p.is_kind(r, 'ReturnParameterMembership')]
    assert len(results) == 1, expression
    return results[0]

def referent(expression):
    memberships = [r for r in p.refs(expression, 'ownedRelationship')
                   if p.is_kind(r, 'Membership') and not p.is_kind(r, 'ParameterMembership')]
    assert len(memberships) == 1
    membership = memberships[0]
    values = p.refs(membership, 'ownedRelatedElement') + p.refs(membership, 'memberElement')
    if values:
        assert len(values) == 1
        return values[0], membership
    return endpoint(membership), membership

array, collection = p.find('Collections::Array'), p.find('Collections::Collection')
rows = []
for expression in p.records:
    if not p.is_kind(expression, 'IndexExpression'): continue
    parameters = [v for r, v in p.members(expression) if p.is_kind(r, 'ParameterMembership')
                  and not p.is_kind(r, 'ReturnParameterMembership')]
    assert parameters
    values = [(r, v) for r, v in p.members(parameters[0]) if p.is_kind(r, 'FeatureValue')]
    assert len(values) == 1
    value_membership, argument = values[0]
    argument_result = result(argument)
    kind_name = p.records[argument]['metaclass']
    membership = None
    if kind_name == 'FeatureReferenceExpression':
        typing_subject, membership = referent(argument)
        rule = 'checkFeatureReferenceExpressionResultSpecialization'
    elif kind_name == 'FeatureChainExpression':
        typing_subject, membership = referent(argument)
        rule = 'checkFeatureChainExpressionResultSpecialization + Feature::typingFeatures terminal projection'
    elif kind_name == 'InvocationExpression':
        function, membership = referent(argument)
        assert p.is_kind(function, 'Function')
        typing_subject = result(function)
        rule = 'checkFeatureResultRedefinition on the invoked Function result'
    else:
        assert kind_name == 'OperatorExpression' and p.records[argument_result]['source']['text'] == 'Anything'
        typing_subject = argument_result
        rule = 'Explicit cast result FeatureTyping'
    array_path = path_to(typing_subject, array)
    collection_path = path_to(typing_subject, collection)
    positive = array_path is not None and collection_path is not None
    rows.append(dict(expression=p.brief(expression), first_parameter=p.brief(parameters[0]),
                     value_membership=p.brief(value_membership), argument=p.brief(argument),
                     argument_result=p.brief(argument_result), typing_subject=p.brief(typing_subject),
                     semantic_result_edge=rule,
                     endpoint_cross_check=answers.get(membership),
                     explicit_array_path=array_path, explicit_collection_path=collection_path,
                     both_guards_positively_proven=positive,
                     complete_negative_implied_type_closure_proven=False,
                     authority_guard_difference_ruled_out=positive))
assert len(rows) == 23
assert sum(row['both_guards_positively_proven'] for row in rows) == 7
assert all((row['explicit_array_path'] is None) == (row['explicit_collection_path'] is None) for row in rows)
report = dict(format='agentique-v10-pinned-index-authority/1', issue='KERML11-69',
              source_archive_sha256=digest(p.archive), source_refinement_audit_sha256=digest(audit_path),
              normative_xmi_sha256=inventory['sha256'], source_index_expressions=23,
              positive_guard_agreement_proofs=7, remaining_negative_closure_proofs=16,
              witnesses=rows, additional_correction_applied=False,
              authority_applicability_closed=False,
              scope='All 23 pinned first operands and their ordered ownership/results were examined independently. Seven have explicit specialization paths to both Array and Collection; no additional implied edges can make those positive predicates false. The other sixteen currently have neither path in the reconstructed explicit graph; missing complete implied-typing proof is retained, not converted into a negative semantic answer. Endpoint identities are cross-checked against the source refinement report, not a second complete resolver.')
encoded=json.dumps(report, indent=2, ensure_ascii=False)+'\n'
path=OUT/'index-authority.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8') == encoded
else: path.write_text(encoded, encoding='utf8', newline='\n')
print('All 23 pinned IndexExpression first operands traced; seven Array/Collection guard agreements positively proved. Sixteen complete negative type-closure proofs remain open; no Collection correction adopted.')
