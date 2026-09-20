"""Independent KERML11-8 witness after the relevant structural inference.

Reads pinned normative XMI, exact KPAR bytes and separately retained reference
XMI. Does not import Agentique lowering, queries or KERML11-145 helpers.
The counterfactual chain below is a proof object, NOT an operational profile.
"""
import hashlib
import json
from pathlib import Path
import sys
import zipfile

sys.dont_write_bytecode = True
OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
OLD = ROOT/'verification/kerml-semantic-closure-v6'
sys.path.insert(0, str(ROOT/'verification/kerml-library-content-errata-publication-v5'))
from xmi import Model, XID


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


inventory = json.loads((OLD/'metamodel-inventory.json').read_text())
assert sha(ROOT/inventory['file']) == inventory['sha256']
names = {
    'checkFeatureReferenceExpressionBindingConnector',
    'checkFeatureReferenceExpressionResultSpecialization',
    'checkConnectorTypeFeaturing', 'deriveConnectorDefaultFeaturingType',
    'deriveFeatureFeaturingType', 'checkFeatureFeatureMembershipTypeFeaturing',
    'isFeaturingType', 'isFeaturedWithin', 'isCompatibleWith', 'canAccess',
    'supertypes', 'allSupertypes', 'deriveFeatureFeatureTarget',
    'deriveFeatureChainingFeature', 'validateFeatureChainingFeatureConformance',
    'checkFunctionResultBindingConnector',
}
clauses = [r for r in inventory['members'] if r['name'] in names]
rule = next(r for r in clauses if r['name']=='checkFeatureReferenceExpressionBindingConnector')
body = next(b['body'] for b in rule['bodies'] if b.get('language')=='OCL2.0')
assert 'ownedMember' in body and 'includes(result)' in body
# Published OCL says targetFeature/relatedFeatures. The published prose says
# referent/relatedFeature. Preserve both; this proof also fails under the clear
# prose interpretation, independently of those editorial identifier errors.
assert 'referent' in next(b['body'] for b in rule['bodies'] if b.get('language') is None)
model = Model(OLD/'release/sysml.library.xmi.implied')


def targets(e, kind, prop):
    return [t for r in e if model.kind(r)==kind for t in model.targets(r, prop)]


def owned(e, kind):
    return [t for r in e if model.kind(r)==kind for t in r if t.tag=='ownedRelatedElement']


def chain(e):
    # XML child sequence records normative FeatureChaining order. Never sort.
    return targets(e, 'FeatureChaining', 'chainingFeature')


def generals(e):
    assert e.get('isConjugated', 'false')=='false'
    result = []
    for kind, prop in [('Specialization','general'),('Subclassification','superclassifier'),
        ('FeatureTyping','type'),('Subsetting','subsettedFeature'),
        ('Redefinition','redefinedFeature'),('ReferenceSubsetting','referencedFeature')]:
        result.extend(targets(e, kind, prop))
    if chain(e): result.append(chain(e)[-1])
    return result


def closure(e, step):
    seen = set()
    pending = [e]
    while pending:
        current = pending.pop()
        if current in seen: continue
        seen.add(current)
        pending.extend(step(current))
    return seen


def featuring(e, seen=frozenset()):
    assert e not in seen, 'cyclic featuring is outside this finite witness'
    result = targets(e, 'TypeFeaturing', 'featuringType')
    if chain(e): result += featuring(chain(e)[0], seen | {e})
    return set(result)


function = model.find('ControlFunctions::.')
expression, = owned(function, 'ResultExpressionMembership')
raw, = owned(expression, 'ReturnParameterMembership')
referent, = targets(expression, 'Membership', 'memberElement')
inner, = [t for r in expression for t in r if model.kind(t)=='BindingConnector']
outer, = [t for r in function for t in r if model.kind(t)=='BindingConnector']


def ends(binding):
    # The current reference serializes isEnd=true under FeatureMembership.
    return [e for r in binding if model.kind(r) in ('FeatureMembership','EndFeatureMembership')
            for e in r if e.tag=='ownedRelatedElement' and e.get('isEnd')=='true']


def endpoints(binding):
    return [t for end in ends(binding) for t in targets(end, 'ReferenceSubsetting', 'referencedFeature')]


assert model.kind(expression)=='FeatureReferenceExpression'
assert model.kind(function)=='Function'
assert set(endpoints(inner)) == {referent, raw}
assert model.kind(model.parents[inner])=='OwningMembership'
assert model.parents[model.parents[inner]] is expression
assert featuring(raw)=={expression} and featuring(expression)=={function}
assert featuring(referent)=={function} and featuring(inner)=={function}
assert not chain(raw) and raw.get('isVariable','false')=='false'
assert referent in generals(raw), 'include required result-to-referent subsetting'
assert targets(raw,'Redefinition','redefinedFeature'), 'include result positional redefinition'
assert chain(referent), 'include the actual source.target feature chain'
assert chain(referent)[0].get('isVariable','false')=='false'
assert targets(inner,'Subsetting','subsettedFeature'), 'include selfLinks base specialization'
for end in ends(inner):
    assert targets(end,'Redefinition','redefinedFeature'), 'include end positional redefinitions'
    assert targets(end,'ReferenceSubsetting','referencedFeature')

function_supers = closure(function, generals)
expression_supers = closure(expression, generals)
assert expression not in function_supers and function not in expression_supers
# Feature::isCompatibleWith's common-redefinition alternative cannot apply to
# F (a Function classifier), or E (which owns a result Feature). Thus the
# required Type/Feature cases below reduce to specializes, including equality.
assert owned(expression, 'ReturnParameterMembership')
candidates = set().union(*(closure(t, featuring) for f in (referent, raw) for t in featuring(f)))
assert candidates=={function,expression}


def featured_within(feature, domain):
    assert feature in (referent, raw)
    assert feature.get('isVariable','false')=='false'
    assert not chain(feature) or chain(feature)[0].get('isVariable','false')=='false'
    if domain is None:
        return all(t is model.find('Base::Anything') for t in featuring(feature))
    assert domain in (function,expression)
    supers = function_supers if domain is function else expression_supers
    return featuring(feature).issubset(supers)


common = [t for t in candidates if all(featured_within(f,t) for f in (referent,raw))]
assert not common, 'no normative default featuring type'
assert not all(featured_within(f,None) for f in (referent,raw))
assert not all(featured_within(f,function) for f in (referent,raw))
assert not all(featured_within(f,expression) for f in (referent,raw))
assert featured_within(raw,expression) and featured_within(referent,function)
for previous, following in zip(chain(referent),chain(referent)[1:]):
    # Existing source.target chain is complete and conforming; it is not the
    # cause of the separate raw-result connector contradiction.
    assert featuring(following).issubset(closure(previous,generals))
# Construct the authorized direction of the OUTER result-binding correction
# independently: contextual feature C=[E,R], with domain inherited from E.
# No source identity, raw result identity, inner endpoint or specialization is
# changed. This proves the inner conflict survives the relevant 145 repair.
contextual_domain = featuring(expression)
assert contextual_domain=={function}
outer_result, = [f for f in endpoints(outer) if f is not raw]
assert featuring(outer_result).issubset(function_supers)
assert contextual_domain.issubset(function_supers)
assert not featured_within(raw,function)


def brief(e):
    return dict(file=model.files[e],id=e.get(XID),metaclass=model.kind(e),path=model.path(e))


def rows(elements):
    # Sorting report serialization is not a language ordering decision.
    return sorted([brief(e) for e in elements],key=lambda r:(r['file'],r['id']))


libraries = json.loads((ROOT/'standards/normative/sysml-2.0/library-set.json').read_text())
archive = next(a for a in libraries['artifacts'] if a['source'].endswith('Function-Library.kpar'))
assert sha(ROOT/archive['path'])==archive['sha256']
entry = next(e for e in archive['entries'] if e['path'].endswith('/ControlFunctions.kerml'))
with zipfile.ZipFile(ROOT/archive['path']) as z: source=z.read(entry['path'])
assert hashlib.sha256(source).hexdigest()==entry['sha256']
text = source.decode('utf8')
start = text.index("abstract function '.'")
end = text.index("abstract function 'if'",start)
assert 'private feature chain chains source.target;' in text[start:end]
issue = OUT/'issues/KERML11-8.html'
assert 'status-public-open">open</span>' in issue.read_text(encoding='utf8')
adapter = OUT/'pilot/FeatureReferenceExpressionAdapter.java'
assert 'addBindingConnector(referent, result)' in adapter.read_text()
assert json.loads((OUT/'release-head.json').read_text())['sha']==json.loads((OLD/'release-head.json').read_text())['sha']
report = dict(format='agentique-independent-feature-reference-conflict/1',id='KLCV8-F-001',
    issue=dict(key='KERML11-8',status='open',url='https://issues.omg.org/issues/KERML11-8',sha256=sha(issue)),
    affected_rule=rule,required_graph_predicates=clauses,
    normative_xmi=dict(path=inventory['file'],sha256=inventory['sha256']),
    pinned_source=dict(archive=archive['path'],archive_sha256=archive['sha256'],entry=entry['path'],sha256=entry['sha256'],
        range=[len(text[:start].encode()),len(text[:end].encode())],text=text[start:end]),
    reference=dict(release=json.loads((OUT/'release-head.json').read_text())['sha'],
        current_head_matches_retained_xmi=True,
        inputs=[dict(path=p.relative_to(ROOT).as_posix(),sha256=sha(p)) for p in sorted((OLD/'release/sysml.library.xmi.implied').rglob('*.kermlx'))],
        implementation=dict(commit=json.loads((OUT/'pilot-head.json').read_text())['sha'],path=adapter.relative_to(ROOT).as_posix(),sha256=sha(adapter)),
        function=brief(function),expression=brief(expression),referent=brief(referent),raw_result=brief(raw),
        reference_binding=brief(inner),result_binding=brief(outer),reference_binding_membership=model.kind(model.parents[inner]),
        reference_binding_endpoints=rows(endpoints(inner)),reference_binding_featuring_types=rows(featuring(inner)),
        raw_result_featuring_types=rows(featuring(raw)),referent_chain=[brief(e) for e in chain(referent)],
        function_all_supertypes=rows(function_supers),expression_all_supertypes=rows(expression_supers),
        required_inference=[model.record(raw),model.record(referent),model.record(inner)]),
    independently_evaluated=dict(required_raw_endpoints=True,result_subsets_referent=True,
        positional_result_redefinition=True,positional_end_redefinitions=True,chain_terminal_in_supertype_closure=True,
        existing_source_chain_conformance=True,contextual_result_chain_conformance=True,
        base_specializations_included=True,raw_result_is_variable=False,
        function_specializes_expression=False,expression_specializes_function=False,
        common_default_featuring_candidates=rows(candidates),valid_default_featuring_types=[],
        domain_truth_table=[dict(domain=brief(t),referent=featured_within(referent,t),raw_result=featured_within(raw,t)) for t in (function,expression)],
        empty_featuring_set_passes=False,reference_connector_type_featuring_passes=False),
    counterfactual_authorized_145=dict(status='independent proof object; no v6 implementation or accepted profile claimed',
        outer_connector_owner=brief(function),outer_membership='FeatureMembership',outer_featuring_types=[brief(function)],
        outer_endpoints=[brief(outer_result),dict(role='contextual-result',distinct_from_raw=True,chaining_features=[brief(expression),brief(raw)])],
        outer_connector_domain_passes=True,inner_connector_unchanged=True,inner_connector_domain_passes=False),
    alternatives=[
        dict(choice='Contextualize the inner connector raw-result endpoint',effect='Different canonical endpoint; violates the uncorrected includes(result) requirement and requires a sixth rule-family correction.'),
        dict(choice='Plain OwningMembership',effect='Already used in reference XMI; removes forced expression ownership domain but does not create any common featuring type for these endpoints.'),
        dict(choice='Empty featuring types',effect='Fails isFeaturedWithin(null) for the raw result and referent.'),
        dict(choice='Add Function-to-Expression specialization or an invented common subtype',effect='Adds materially different semantics not implied by the source, positional rules, chain rules or any authorized correction.'),
        dict(choice='Change result ownership, identity, variable flag or weaken connector conformance',effect='Contradicts other required structural semantics; not an inference completion.')],
    editorial_note='Published OCL uses targetFeature/relatedFeatures; the contradiction persists under the normative prose referent/relatedFeature interpretation.',
    excluded_corrections=['KERML11-81','KERML11-140','KERML11-76','KERML11-68','KERML11-205','KERML11-206','KERML11-207','five KERML11-145 rules'],
    no_execution_required=True,correction_authorized=False,correction_applied=False,accepted_publication=False)
encoded = json.dumps(report,indent=2)+'\n'
path = OUT/'feature-reference-authority-conflict.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8')==encoded
else: path.write_text(encoded,encoding='utf8',newline='\n')
print('KLCV8-F-001: exact pinned inner FeatureReferenceExpression binding fails domain conformance after result/end/chain/base inference; independent outer KERML11-145 repair does not change it.')
