"""Independent pinned-source/XMI reproduction of the result-binding conflict.

Does not call Agentique lowering, semantic queries, or an OCL interpreter.
The retained formal clauses define the small graph predicates evaluated below.
"""
import hashlib
import json
from pathlib import Path
import re
import sys
import zipfile
sys.dont_write_bytecode = True
OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
OLD = ROOT/'verification/kerml-semantic-closure-v6'
sys.path.insert(0,str(ROOT/'verification/kerml-library-content-errata-publication-v5'))
from xmi import Model, XID
sha = lambda b: hashlib.sha256(b).hexdigest()
inventory = json.loads((OLD/'metamodel-inventory.json').read_text())
assert sha((ROOT/inventory['file'].replace('\\','/')).read_bytes()) == inventory['sha256']
names = ['checkFunctionResultBindingConnector','checkConnectorTypeFeaturing',
         'checkFeatureFeatureMembershipTypeFeaturing','isFeaturingType','isFeaturedWithin',
         'isCompatibleWith','supertypes','allSupertypes','deriveTypeOwnedFeature',
         'deriveTypeOwnedFeatureMembership','deriveFeatureFeaturingType']
clauses = [r for r in inventory['members'] if r['name'] in names]
rule = next(r for r in clauses if r['name']=='checkFunctionResultBindingConnector')
formal = next(b['body'] for b in rule['bodies'] if b.get('language')=='OCL2.0')
assert 'ownedFeature.selectByKind(BindingConnector)' in formal
assert 'includes(mem.ownedResultExpression.result)' in formal
model = Model(OLD/'release/sysml.library.xmi.implied')
function = model.find('ControlFunctions::.')
membership, = [r for r in function if model.kind(r)=='ResultExpressionMembership']
expression, = [e for e in membership if e.tag=='ownedRelatedElement']
result_membership, = [r for r in expression if model.kind(r)=='ReturnParameterMembership']
result, = [e for e in result_membership if e.tag=='ownedRelatedElement']
assert model.kind(function)=='Function' and model.kind(expression)=='FeatureReferenceExpression'
assert result.get('isVariable','false')=='false'
assert not any(model.kind(r)=='FeatureChaining' for r in result)
featuring = [t for r in result if model.kind(r)=='TypeFeaturing' for t in model.targets(r,'featuringType')]
assert featuring == [expression]
bindings = [e for r in function for e in r if model.kind(e)=='BindingConnector']
assert len(bindings)==1
binding = bindings[0]
related = [t for r in binding for end in r for s in end if model.kind(s)=='ReferenceSubsetting' for t in model.targets(s,'referencedFeature')]
binding_types = [t for r in binding if model.kind(r)=='TypeFeaturing' for t in model.targets(r,'featuringType')]
assert result in related and binding_types == [expression]
assert model.kind(model.parents[binding]) == 'OwningMembership'

def generals(element):
    result = []
    for relation in element:
        for kind, prop in [('Subclassification','superclassifier'),('Specialization','general'),
                           ('FeatureTyping','type'),('Subsetting','subsettedFeature'),
                           ('Redefinition','redefinedFeature'),('ReferenceSubsetting','referencedFeature')]:
            if model.kind(relation)==kind:
                result.extend(model.targets(relation,prop))
    chain = [t for r in element if model.kind(r)=='FeatureChaining' for t in model.targets(r,'chainingFeature')]
    if chain: result.append(chain[-1])
    return result

closure = set()
pending = [function]
while pending:
    current = pending.pop()
    if current in closure: continue
    closure.add(current)
    pending.extend(generals(current))
assert expression not in closure
# Function is a Classifier, so Type::isCompatibleWith is specialization. The
# Feature override (common redefinition) is inapplicable to this source type.
compatible = expression in closure
featured_within = compatible # result is nonvariable and is not a feature chain
assert not featured_within
# Even an empty featuring set on the connector does not fix the raw result:
# isFeaturedWithin(null) requires every featuring type to be Base::Anything.
anything = model.find('Base::Anything')
assert expression is not anything
assert model.kind(model.parents[expression])=='ResultExpressionMembership'
expression_types = [t for r in expression if model.kind(r)=='TypeFeaturing' for t in model.targets(r,'featuringType')]
assert expression_types == [function]
# The contemplated replacement chain begins at expression, so its featuring
# type includes function rather than expression. It is a DISTINCT Feature;
# relatedFeature->includes(rawResult) is identity-based, not specializes-based.

libraries = json.loads((ROOT/'standards/normative/sysml-2.0/library-set.json').read_text())
archive = next(a for a in libraries['artifacts'] if a['source'].endswith('Function-Library.kpar'))
assert sha((ROOT/archive['path']).read_bytes())==archive['sha256']
entry = next(e for e in archive['entries'] if e['path'].endswith('/ControlFunctions.kerml'))
with zipfile.ZipFile(ROOT/archive['path']) as z: source=z.read(entry['path'])
assert sha(source)==entry['sha256']
text=source.decode('utf8')
start=text.index("abstract function '.'")
end=text.index("abstract function 'if'",start)
witness=text[start:end]
assert 'private feature chain chains source.target;' in witness
assert re.search(r'\bchain\s*\}',witness)
issue=OUT/'issues/KERML11-145.html'
assert 'status-public-open">open</span>' in issue.read_text(encoding='utf8')
assert 'checkFunctionResultBindingConnector' in issue.read_text(encoding='utf8')
reference_path=next((OLD/'release/sysml.library.xmi.implied').rglob('ControlFunctions.kermlx'))
adapter_path=OUT/'pilot/org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/TypeAdapter.java'
adapter=adapter_path.read_text()
assert 'addBindingConnector(sourceResult, target)' in adapter
assert 'if (contextType == type)' in adapter
assert 'addImplicitFeatureBindingConnector(connector)' in adapter
assert 'addImplicitMemberBindingConnector(connector)' in adapter
pdf=json.loads((OUT/'formal-pdf-clauses.json').read_text())
assert sha((ROOT/pdf['path']).read_bytes())==pdf['sha256']
def brief(e):
    return dict(file=model.files[e],id=e.get(XID),metaclass=model.kind(e),path=model.path(e))
report=dict(format='agentique-independent-result-binding-conflict/1',id='KLCV7-F-001',
    issue=dict(key='KERML11-145',status='open',url='https://issues.omg.org/issues/KERML11-145',sha256=sha(issue.read_bytes())),
    affected_rule=rule,required_graph_predicates=clauses,
    pinned_xmi=dict(path=inventory['file'],sha256=inventory['sha256']),
    pinned_pdf=pdf,
    pinned_source=dict(archive=archive['path'],archive_sha256=archive['sha256'],entry=entry['path'],sha256=entry['sha256'],
        range=[len(text[:start].encode()),len(text[:end].encode())],text=witness),
    reference=dict(release=json.loads((OLD/'release-head.json').read_text())['sha'],path=reference_path.relative_to(ROOT).as_posix(),
        sha256=sha(reference_path.read_bytes()),function=brief(function),result_membership=brief(membership),
        closure_inputs=[dict(path=p.relative_to(ROOT).as_posix(),sha256=sha(p.read_bytes())) for p in sorted((OLD/'release/sysml.library.xmi.implied').rglob('*.kermlx'))],
        implementation=dict(commit=json.loads((OUT/'pilot-head.json').read_text())['sha'],
            path=adapter_path.relative_to(ROOT).as_posix(),sha256=sha(adapter_path.read_bytes()),
            behavior='TypeAdapter.addResultBinding passes raw results. addBindingConnector selects feature ownership only when contextType equals the owner; otherwise plain member ownership and the context featuring type.'),
        expression=brief(expression),raw_result=brief(result),binding=brief(binding),related_features=[brief(t) for t in related],
        function_all_supertypes=sorted([brief(t) for t in closure],key=lambda r:r['id']),
        binding_featuring_types=[brief(t) for t in binding_types],result_featuring_types=[brief(t) for t in featuring],
        binding_membership_kind=model.kind(model.parents[binding]),
        literal_owned_feature_requirement_satisfied=False),
    independently_evaluated=dict(scope='Published required ownedFeature interpretation, not the reference plain OwningMembership interpretation.',result_binding_antecedent=True,raw_result_related=True,result_is_variable=False,
        result_has_chain=False,function_specializes_expression=False,raw_result_featured_within_function=False,
        raw_result_featured_within_null=False,required_owned_feature_connector_type_featuring=False),
    alternatives=[dict(choice='Published raw result identity',result='Required by checkFunctionResultBindingConnector; violates checkConnectorTypeFeaturing under the required function featuring type.'),
        dict(choice='Reference plain OwningMembership and nested-expression featuring type',result='Keeps raw result identities and avoids the Function domain, but violates the literal ownedFeature selection. Not authorized by the six v5 target replacements.'),
        dict(choice='Chain of result expression and its result',result='Changes relatedFeature identity. Repairs domain but does not satisfy the literal includes(rawResult) constraint.'),
        dict(choice='Additional function-to-expression specialization or altered ownership/flags',result='Changes canonical semantic meaning; not an implication prescribed by these rules or an authorized erratum.')],
    implementation_independence=['Pinned tail-expression grammar establishes ResultExpressionMembership.',
        'An ordinary owned BindingConnector feature must be featured by its Function.',
        'The nested nonvariable result must be featured by its expression.',
        'Complete independently read reference supertype closure excludes the nested expression.',
        'The reference keeps both raw results but uses plain OwningMembership and the nested expression domain; it does not satisfy the formal ownedFeature requirement.',
        'Adding missing Agentique bindings, positional redefinitions or inference does not equate these distinct canonical identities.'],
    scope='Structural abstract syntax only. No runtime value or execution is evaluated.',
    excluded_corrections=['81','140','76','68','205','206','207'],correction_applied=False,accepted_publication=False)
encoded=json.dumps(report,indent=2)+'\n'
path=OUT/'result-binding-authority-conflict.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8')==encoded
else: path.write_text(encoded,encoding='utf8',newline='\n')
print('KLCV7-F-001 reproduced independently: pinned ControlFunctions::. requires raw result binding; connector featuring check fails; chain changes required identity.')
