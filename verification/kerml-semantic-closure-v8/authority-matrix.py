"""Freeze each of the five KERML11-145 families without adopting pilot graphs."""
import gzip
import hashlib
import json
from pathlib import Path
import sys
import zipfile
from pypdf import PdfReader

sys.dont_write_bytecode = True
OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
OLD = ROOT/'verification/kerml-semantic-closure-v6'
sys.path.insert(0,str(ROOT/'verification/kerml-library-content-errata-publication-v5'))
from xmi import Model, XID


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    text = json.dumps(value,indent=2,ensure_ascii=False)+'\n'
    if '--check' in sys.argv: assert path.read_text(encoding='utf8')==text, str(path)
    else: path.write_text(text,encoding='utf8',newline='\n')


inventory = json.loads((OLD/'metamodel-inventory.json').read_text())
assert sha(ROOT/inventory['file'])==inventory['sha256']
rule_names = ['checkFeatureValuationSpecialization','checkExpressionResultBindingConnector',
    'checkFunctionResultBindingConnector','checkIndexExpressionResultSpecialization',
    'checkSelectExpressionResultSpecialization']
rules = {r['name']:r for r in inventory['members'] if r['kind']=='ownedRule'}
related = ['validateSubsettingFeaturingTypes','checkConnectorTypeFeaturing',
    'deriveConnectorDefaultFeaturingType','checkFeatureFeatureMembershipTypeFeaturing',
    'deriveFeatureFeaturingType','deriveFeatureFeatureTarget','deriveFeatureChainingFeature',
    'validateFeatureChainingFeatureConformance','checkFeatureValueBindingConnector',
    'deriveExpressionResult','deriveFunctionResult','validateExpressionResultParameterMembership',
    'validateFunctionResultParameterMembership','checkExpressionTypeFeaturing']
for name in related: assert name in rules, name
issue = OUT/'issues/KERML11-145.html'
issue_text = issue.read_text(encoding='utf8')
assert 'status-public-open">open</span>' in issue_text
assert all(name in issue_text for name in rule_names)

# Freeze the actual PDF clauses separately from the normative-XMI bodies.
pdf = ROOT/'KerML.pdf'
page_names = rule_names+related+['checkFeatureReferenceExpressionBindingConnector',
    'checkFeatureReferenceExpressionResultSpecialization','isFeaturedWithin','isCompatibleWith']
pages = []
for index,page in enumerate(PdfReader(pdf).pages):
    text = page.extract_text()
    matches = [name for name in page_names if name in text]
    if matches: pages.append(dict(page=index+1,matches=matches,text=text))
write(OUT/'formal-pdf-clauses.json',dict(path='KerML.pdf',sha256=sha(pdf),pages=pages))

# Exact pinned source spans are independently checked against original archives.
# Prior canonical IDs locate witnesses only; they are not current closure claims.
archive_path = ROOT/'verification/kerml-semantic-closure-v7/declarations-v5.json.gz'
archive = json.loads(gzip.decompress(archive_path.read_bytes()))
metamodel_path=ROOT/'standards/generated/kerml-1.0/metamodel.json'
classifiers=json.loads(metamodel_path.read_text())['metamodel']['classifiers']
by_name={c['entity']['name']:c for c in classifiers.values() if c['entity']['key']['kind']=='class'}


def is_kind(name, base):
    pending=[by_name[name]]
    seen=set()
    while pending:
        current=pending.pop()
        current_name=current['entity']['name']
        if current_name==base: return True
        if current_name in seen: continue
        seen.add(current_name)
        pending.extend(classifiers[g] for g in current['generalizations'])
    return False


def source_refs(identity, property):
    return [t for slot in archive['records'][identity]['slots'].values() if slot['name']==property for t in slot['references']]


source_parents={t:identity for identity in archive['records'] for property in ['ownedRelationship','ownedRelatedElement'] for t in source_refs(identity,property)}
libraries = json.loads((ROOT/'standards/normative/sysml-2.0/library-set.json').read_text())
source_bytes = {}
for artifact in libraries['artifacts']:
    if artifact['source'].endswith('Systems-Library.kpar'): continue
    assert sha(ROOT/artifact['path'])==artifact['sha256']
    with zipfile.ZipFile(ROOT/artifact['path']) as z:
        for entry in artifact['entries']:
            if entry['path'].endswith('.kerml'):
                raw = z.read(entry['path'])
                assert hashlib.sha256(raw).hexdigest()==entry['sha256']
                source_bytes[entry['path']] = raw


def source_witnesses(classes, rule_name):
    found = []
    for identity,record in archive['records'].items():
        if record['metaclass'] not in classes or not record.get('source'): continue
        owner=source_parents.get(identity)
        if rule_name in ('checkExpressionResultBindingConnector','checkFunctionResultBindingConnector'):
            base='Expression' if rule_name=='checkExpressionResultBindingConnector' else 'Function'
            if owner is None or not is_kind(archive['records'][owner]['metaclass'],base): continue
        if rule_name=='checkFeatureValuationSpecialization':
            owner_record=archive['records'][owner]
            if any(s['name']=='direction' for s in owner_record['slots'].values()): continue
            specializations=[archive['records'][r] for r in source_refs(owner,'ownedRelationship')
                if is_kind(archive['records'][r]['metaclass'],'Specialization')]
            if any(not any(s['name']=='isImplied' and s['value']=='Scalar(Boolean(true))' for s in r['slots'].values()) for r in specializations): continue
        source = record['source']
        raw = source_bytes[source['document']]
        assert hashlib.sha256(raw).hexdigest()==source['sha256']
        start,end = source['range']
        assert raw[start:end].decode('utf8')==source['text']
        found.append(dict(canonical_id=identity,metaclass=record['metaclass'],source=source,
            owner=dict(canonical_id=owner,metaclass=archive['records'][owner]['metaclass']) if owner else None))
    found.sort(key=lambda r:(r['source']['document'],r['source']['range']))
    return dict(population=len(found),representatives=found[:3],
        interpretation='Source populations selected by rule-owning metaclass; valuation additionally checks the published undirected/no-nonimplied-owned-specialization antecedent. Zero is not a final expansion completeness claim.')


model = Model(OLD/'release/sysml.library.xmi.implied')


def brief(e):
    return dict(file=model.files[e],id=e.get(XID),metaclass=model.kind(e),path=model.path(e))


def targets(e,kind,prop):
    return [t for r in e if model.kind(r)==kind for t in model.targets(r,prop)]


def owned(e,kind):
    return [t for r in e if model.kind(r)==kind for t in r if t.tag=='ownedRelatedElement']


def bindings(e):
    result=[]
    for rel in e:
        for binding in rel:
            if model.kind(binding)!='BindingConnector': continue
            ends=[t for r in binding for t in r if t.get('isEnd')=='true']
            result.append(dict(binding=brief(binding),membership=brief(rel),
                endpoints=[brief(t) for end in ends for t in targets(end,'ReferenceSubsetting','referencedFeature')],
                featuring_types=[brief(t) for t in targets(binding,'TypeFeaturing','featuringType')]))
    return result


reference = {}
valuations=[]
for fv in model.files:
    if model.kind(fv)!='FeatureValue': continue
    owner=model.parents[fv]
    expressions=[e for e in fv if e.tag=='ownedRelatedElement']
    if not expressions or owner.get('direction') is not None: continue
    expression=expressions[0]
    results=owned(expression,'ReturnParameterMembership')
    if len(results)!=1: continue
    raw=results[0]
    contextual=[t for t in targets(owner,'Subsetting','subsettedFeature')
        if targets(t,'FeatureChaining','chainingFeature')==[expression,raw]]
    if contextual:
        valuations.append(dict(feature=brief(owner),value=brief(expression),raw_result=brief(raw),
            contextual_result=brief(contextual[0]),chain=[brief(expression),brief(raw)],
            contextual_membership=brief(model.parents[contextual[0]]),bindings=bindings(owner)))
reference[rule_names[0]]=dict(matches=len(valuations),representatives=valuations[:3])
for owner_kind,rule_name in [('Function',rule_names[2]),('Expression',rule_names[1])]:
    matches=[]
    for membership in model.files:
        if model.kind(membership)!='ResultExpressionMembership': continue
        owner=model.parents[membership]
        kind=model.kind(owner)
        if not is_kind(kind,owner_kind): continue
        expression,=[e for e in membership if e.tag=='ownedRelatedElement']
        matches.append(dict(owner=brief(owner),result_expression=brief(expression),
            raw_results=[brief(t) for t in owned(expression,'ReturnParameterMembership')],bindings=bindings(owner)))
    reference[rule_name]=dict(matches=len(matches),representatives=matches[:3])
reference[rule_names[2]]['control_functions_dot']=dict(function=brief(model.find('ControlFunctions::.')),
    bindings=bindings(model.find('ControlFunctions::.')))
for kind,rule_name in [('IndexExpression',rule_names[3]),('SelectExpression',rule_names[4])]:
    matches=[]
    for e in model.files:
        if model.kind(e)!=kind: continue
        arguments=[value for parameter in owned(e,'ParameterMembership') for value in owned(parameter,'FeatureValue')]
        result=owned(e,'ReturnParameterMembership')
        matches.append(dict(expression=brief(e),arguments=[brief(a) for a in arguments],
            result=[brief(r) for r in result],result_subsetting=[brief(t) for r in result for t in targets(r,'Subsetting','subsettedFeature')],
            first_argument_results=[brief(r) for a in arguments[:1] for r in owned(a,'ReturnParameterMembership')]))
    reference[rule_name]=dict(matches=len(matches),representatives=matches[:3])

interpretations = [
    dict(graph='Undirected Feature with only implied owned specializations specializes the raw value-expression result.',
        contradiction='The raw result is featured by the nested value Expression; the valued Feature generally cannot access that domain.',
        pilot='FeatureAdapter.getBoundValueResult builds FeatureUtil.chainFeatures(value,value.getResult()); addBoundValueSubsetting adds a Subsetting to it. Its non-default and no-owned-specialization guards are narrower than the published all-implied guard; do not copy them.',
        files=['FeatureAdapter.java'],sources=['FeatureValue'],
        proposed='Keep the published direction and all-implied-specialization antecedent; imply Subsetting to a distinct contextual Feature with chainingFeature=[value,value.result]. Reuse that structural chain concept with checkFeatureValueBindingConnector, preserving its separate non-default and initial-value conditions.'),
    dict(graph='An Expression with a ResultExpressionMembership owns a BindingConnector Feature relating its own raw result and its nested result Expression raw result.',
        contradiction='Feature ownership requires the outer Expression domain, while the raw nested result requires the nested Expression domain.',
        pilot='ExpressionAdapter invokes TypeAdapter.addResultBinding; raw results are passed to addBindingConnector. TypeAdapter chooses FeatureMembership only when contextType equals the owner; otherwise it uses OwningMembership and a context featuring type.',
        files=['ExpressionAdapter.java','TypeAdapter.java'],sources=['ResultExpressionMembership'],
        proposed='Relate the outer raw result to contextual_result(nestedExpression); own the BindingConnector through FeatureMembership of the outer Expression. Its required featuring domain follows that ownership. Prove both endpoints are featured within that domain; no arbitrary added domain or raw-result substitution is permitted.'),
    dict(graph='A Function with a ResultExpressionMembership owns a BindingConnector Feature relating its raw result and the result Expression raw result.',
        contradiction='Function ownership requires the Function domain; the nested raw result requires the nested Expression domain. Function specialization of that Expression is not prescribed.',
        pilot='FunctionAdapter invokes the same raw-result TypeAdapter path. ControlFunctions dot XMI uses plain OwningMembership and nested-expression TypeFeaturing, not the proposed contextual chain.',
        files=['FunctionAdapter.java','TypeAdapter.java'],sources=['ResultExpressionMembership'],
        proposed='Relate the canonical Function result (including an inherited result) to contextual_result(resultExpression), owned via Function FeatureMembership and featured by the Function. Retain the inherited result identity and prove compatibility through existing specialization.'),
    dict(graph='If arguments exist and the first result does not specialize Collections::Array, result specializes the first argument raw result.',
        contradiction='Raw first-argument result is featured by its own argument Expression, not necessarily accessible to the IndexExpression result.',
        pilot='IndexExpressionAdapter subsets the raw seqResult and tests Collections::Collection. The Array-to-Collection guard change is KERML11-69, outside the authorized KERML11-145 domain correction.',
        files=['IndexExpressionAdapter.java'],sources=['IndexExpression'],
        proposed='Preserve the published Array conditional pending independent review of KERML11-69; within the applicable branch, subset the distinct [firstArgument,firstArgument.result] chain. Complete argument featuring evidence rather than changing index collection semantics.'),
    dict(graph='When arguments exist, result specializes the raw first-argument result.',
        contradiction='The raw source result has the source Expression domain; the select result needs a structurally accessible result feature.',
        pilot='SelectExpressionAdapter still subsets the raw first-argument result. No SelectExpression appears in either the pinned construction inventory or retained reference XMI.',
        files=['SelectExpressionAdapter.java'],sources=['SelectExpression'],
        proposed='Preserve the argument-existence antecedent and result meaning; subset the [firstArgument,firstArgument.result] contextual chain with complete featuring evidence. No filtering or runtime evaluation is introduced.'),
]
entries=[]
for name,interpretation in zip(rule_names,interpretations):
    rule=rules[name]
    entries.append(dict(rule=name,external_rule_id=rule['attributes']['{http://www.omg.org/spec/XMI/20161101}id'],
        owning_metaclass=rule['owner'],published_body=next(b['body'] for b in rule['bodies'] if b.get('language')=='OCL2.0'),
        relevant_prose=[b['body'] for b in rule['bodies'] if b.get('language') is None],
        exact_normative_record=rule,published_graph_requirement=interpretation['graph'],
        featuring_domain_contradiction=interpretation['contradiction'],
        issue_direction=('Explicitly specialize the feature chain of the value Expression and its result.' if name==rule_names[0] else
            'Explicitly named as a similar problem; issue does not provide a replacement OCL body or one uniform connector ownership recipe.'),
        related_structural_constraints=[rules[r] for r in related],
        pinned_corpus_witnesses=source_witnesses(interpretation['sources'],name),
        reference_implementation=dict(behavior=interpretation['pilot'],
            inputs=[dict(path=f'pilot/{f}',sha256=sha(OUT/'pilot'/f)) for f in interpretation['files']]),
        reference_xmi=reference[name],
        operational_v6_interpretation=dict(direction=interpretation['proposed'],status='authorized design direction; not implemented or accepted because Gate 1 independently exposes KLCV8-F-001'),
        earliest_authorized_profile='v6',formally_adopted_omg_correction=False))
write(OUT/'authority-matrix.json',dict(format='agentique-kerml145-authority-matrix/1',
    issue=dict(key='KERML11-145',status='open',url='https://issues.omg.org/issues/KERML11-145',path='issues/KERML11-145.html',sha256=sha(issue)),
    normative_xmi=dict(path=inventory['file'],sha256=inventory['sha256']),
    metaclass_hierarchy=dict(path=metamodel_path.relative_to(ROOT).as_posix(),sha256=sha(metamodel_path)),
    pinned_pdf=dict(path='KerML.pdf',sha256=sha(pdf),extracted='formal-pdf-clauses.json'),
    pinned_source_identity_index=dict(path=archive_path.relative_to(ROOT).as_posix(),sha256=sha(archive_path),status='historical identity locator; all selected source spans checked against exact KPAR bytes'),
    reference_implementation_commit=json.loads((OUT/'pilot-head.json').read_text())['sha'],
    reference_xmi_commit=json.loads((OUT/'release-head.json').read_text())['sha'],
    reference_xmi_inputs=json.loads((OUT/'feature-reference-authority-conflict.json').read_text())['reference']['inputs'],
    contextual_identity_requirement=['rule/profile','expression canonical identity','terminal canonical result identity','output role'],
    provenance_requirement='Implied/derived operational provenance, never authored StandardLibrary source; KERML11-145 must be explicit in explanations.',
    profile_implementation_exists=False,strict_publication_passed=False,constraints=entries))
print('Five distinct KERML11-145 authority records; raw-result pilot differences and separate KERML11-69 guard change retained; no operational implementation asserted.')
