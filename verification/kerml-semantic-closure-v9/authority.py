"""Independent KERML11-8 witness after the relevant structural inference.

Reads pinned normative XMI, exact KPAR bytes and separately retained reference
XMI. Does not import Agentique lowering, queries or KERML11-145 helpers.
V9 independently re-evaluates the graph and the narrowly authorized exception.
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
from bs4 import BeautifulSoup
from pypdf import PdfReader

# Verify the retained reference files against the freshly fetched Git tree, not
# merely a release label. Git blob identities and byte-content SHA-256 are distinct.
tree = json.loads((OUT/'release-tree.json').read_text())
assert not tree['truncated']
blobs = {r['path']: r['sha'] for r in tree['tree'] if r['type']=='blob'}
inputs = []
for path in sorted((OLD/'release/sysml.library.xmi.implied').rglob('*.kermlx')):
    data = path.read_bytes()
    relative = path.relative_to(OLD/'release').as_posix()
    git_blob = hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
    assert blobs[relative] == git_blob, relative
    inputs.append(dict(path=path.relative_to(ROOT).as_posix(), sha256=sha(path), git_blob=git_blob))

validator_path = OUT/'pilot/KerMLValidator.xtend'
validator = validator_path.read_text()
start_validator = validator.index('private def doCheckConnector(')
end_validator = validator.index('\n\t@Check', start_validator)
special_case = validator[start_validator:end_validator]
assert '(location instanceof FeatureReferenceExpression || location instanceof FeatureChainExpression)' in special_case
assert 'relatedFeature.getOwningType() == location' in special_case
assert 'addBindingConnector(referent, result)' in adapter.read_text()
chain_rules = [r for r in inventory['members'] if r['owner']=='FeatureChainExpression']
assert not any('BindingConnector' in b.get('body','') for r in chain_rules for b in r['bodies'])
chain_adapter = OUT/'pilot/FeatureChainExpressionAdapter.java'
assert 'addBindingConnector' not in chain_adapter.read_text()

preliminary_pdf = OUT/'preliminary/KerML.pdf'
preliminary_pages = []
for index, page in enumerate(PdfReader(preliminary_pdf).pages):
    page_text = page.extract_text()
    matches = sorted(n for n in names if n in page_text)
    if matches:
        preliminary_pages.append(dict(page=index+1, matches=matches, text=page_text))

def exception(endpoint, domain, role='FeatureReferenceResult'):
    # Endpoint qualification is deliberately asymmetric. These are independently
    # read reference facts; no Agentique producer/validator is invoked here.
    return (role=='FeatureReferenceResult' and endpoint is raw and
            model.parents[model.parents[endpoint]] is expression and
            domain is function and featured_within(referent, domain))

truth_table = [dict(domain=brief(t) if t is not None else None,
    referent_domain_valid=featured_within(referent,t),
    raw_result_domain_valid=featured_within(raw,t),
    published_connector_check_result=all(featured_within(f,t) for f in (referent,raw)),
    operational_exception_result=all(featured_within(f,t) or exception(f,t) for f in (referent,raw)))
    for t in (function,expression,None)]
assert [r['operational_exception_result'] for r in truth_table]==[True,False,False]
assert not exception(referent,function)
assert not exception(raw,function,'ResultExpression')

report = dict(format='agentique-reference-binding-authority/1',
    authority='Explicit Agentique project authorization for operational v6; not adopted OMG authority',
    issue=dict(key='KERML11-8', status='open', url='https://issues.omg.org/issues/KERML11-8',
        sha256=sha(issue), text=BeautifulSoup(issue.read_text(encoding='utf8'),'html.parser').get_text(' ',strip=True)),
    formal_constraints=clauses, normative_xmi=dict(path=inventory['file'],sha256=inventory['sha256']),
    pinned_source=dict(archive=archive['path'],archive_sha256=archive['sha256'],entry=entry['path'],sha256=entry['sha256'],
        range=[len(text[:start].encode()),len(text[:end].encode())],text=text[start:end]),
    reference=dict(release=json.loads((OUT/'release-head.json').read_text())['sha'],inputs=inputs,
        function=brief(function),expression=brief(expression),referent=brief(referent),raw_result=brief(raw),
        binding=model.record(inner),result=model.record(raw),chain=model.record(referent),
        function_supertype_closure=rows(function_supers),expression_supertype_closure=rows(expression_supers)),
    pilot=dict(commit=json.loads((OUT/'pilot-head.json').read_text())['sha'],
        construction=dict(path=adapter.relative_to(ROOT).as_posix(),sha256=sha(adapter),text=adapter.read_text()),
        validator=dict(path=validator_path.relative_to(ROOT).as_posix(),sha256=sha(validator_path),text=special_case),
        normative_authority=False, todo_is_authority=False),
    preliminary=dict(path=preliminary_pdf.relative_to(ROOT).as_posix(),sha256=sha(preliminary_pdf),
        normative_authority=False,pages=preliminary_pages),
    default_featuring_type=dict(candidates=rows(candidates),ordinary_valid_candidates=rows(common),ordinary_result=None,
        operational_context=brief(function),
        rule='When the ordinary common context is absent, select the unique nearest direct/indirect featuring context of the expression in which the referent is featured. No traversal-order choice; zero or multiple nearest contexts remain unresolved.'),
    truth_table=truth_table,
    feature_chain_scope=dict(included=False,formal=chain_rules,adapter_sha256=sha(chain_adapter),
        reason='No separate raw reference-result BindingConnector obligation is prescribed by the FeatureChainExpression constraints. Its contextual result Subsetting does not establish the same contradiction. The pilot conditional alone is insufficient authority.',
        nested_feature_reference_expression_included=True),
    canonical_arbitrary_name_witness='crates/kerml-semantics/tests/reference_binding_v9.rs',
    no_execution_required=True, accepted_publication=False)
encoded=json.dumps(report,indent=2,ensure_ascii=False)+'\n'
path=OUT/'reference-binding-authority.json'
if '--check' in sys.argv:
    assert path.read_text(encoding='utf8')==encoded
else:
    path.write_text(encoded,encoding='utf8',newline='\n')
print('KERML11-8: exact raw endpoints retained; only expression-owned result qualifies; FeatureChainExpression excluded; independent truth table passes')
