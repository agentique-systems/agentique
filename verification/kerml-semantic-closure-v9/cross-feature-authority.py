"""Independent cross-feature/value conflict proof. No Agentique semantic helpers.

The literal published selector selects the value Expression. Even if the misplaced
FeatureValue exclusion is read as a membership exclusion, the required binding is
selected instead. Both readings require incompatible exact featuring-type sets.
"""
import hashlib
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET
import zipfile
from functools import cache
from bs4 import BeautifulSoup
from pypdf import PdfReader

sys.dont_write_bytecode = True
OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
sys.path.insert(0, str(ROOT/'verification/kerml-library-content-errata-publication-v5'))
from xmi import Model, XID

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

inventory = json.loads((ROOT/'verification/kerml-semantic-closure-v6/metamodel-inventory.json').read_text())
assert sha(ROOT/inventory['file']) == inventory['sha256']
normative = ET.parse(ROOT/inventory['file']).getroot()
uid = '{http://www.omg.org/spec/XMI/20161101}id'
utype = '{http://www.omg.org/spec/XMI/20161101}type'
classes = {e.get(uid): e for e in normative.iter() if e.get(utype)=='uml:Class'}
by_name = {e.get('name'): e for e in classes.values()}

@cache
def class_closure(name):
    if name not in by_name:
        return set()
    result, todo = set(), [by_name[name]]
    while todo:
        c = todo.pop()
        if c.get('name') in result:
            continue
        result.add(c.get('name'))
        todo.extend(classes[g.find('general').get('{http://www.omg.org/spec/XMI/20161101}idref')]
                    for g in c if g.tag=='generalization')
    return result

m = Model(ROOT/'verification/kerml-semantic-closure-v6/release/sysml.library.xmi.implied')
def kind(e, name):
    return name in class_closure(m.kind(e))
def brief(e):
    return dict(id=e.get(XID), kind=m.kind(e), path=m.path(e), file=m.files[e])
def owned_members(e):
    return [v for r in e if kind(r,'OwningMembership') for v in r if v.tag=='ownedRelatedElement']
def related(e, cls, prop):
    return [v for r in e if kind(r,cls) for v in m.targets(r,prop)]
def featuring(e):
    result = set(related(e,'TypeFeaturing','featuringType'))
    chains = related(e,'FeatureChaining','chainingFeature')
    if chains:
        result.update(featuring(chains[0]))
    return result
def specializes(e):
    result, todo = set(), [e]
    while todo:
        v=todo.pop()
        if v in result:
            continue
        result.add(v)
        for r in v:
            for prop in ['general','superclassifier','type','subsettedFeature','redefinedFeature','referencedFeature']:
                todo.extend(m.targets(r,prop))
        chains=related(v,'FeatureChaining','chainingFeature')
        if chains:
            todo.append(chains[-1])
    return result
def types(e):
    candidates, seen, todo = set(), set(), [e]
    while todo:
        v=todo.pop()
        if v in seen:
            continue
        seen.add(v)
        conjugation=related(v,'Conjugation','originalType')
        if conjugation:
            assert len(conjugation)==1
            todo.extend(t for t in conjugation if kind(t,'Feature'))
        else:
            candidates.update(related(v,'FeatureTyping','type'))
            for r in v:
                if kind(r,'Subsetting') and not kind(r,'CrossSubsetting'):
                    for prop in ['subsettedFeature','redefinedFeature','referencedFeature']:
                        todo.extend(m.targets(r,prop))
            chains=related(v,'FeatureChaining','chainingFeature')
            if chains:
                todo.append(chains[-1])
    reduced = {t for t in candidates if not any(t!=u and t in specializes(u) for u in candidates)}
    return reduced, seen
def selector(end, exclude_value_membership=False):
    candidates=[]
    for v in owned_members(end):
        r=m.parents[v]
        if (kind(v,'Feature') and not any(kind(v,c) for c in ['Multiplicity','MetadataFeature','FeatureValue'])
            and not kind(r,'FeatureMembership') and not (exclude_value_membership and kind(r,'FeatureValue'))):
            candidates.append(v)
    return candidates

rules = {'ownedCrossFeature','isOwnedCrossFeature','checkExpressionTypeFeaturing',
    'checkFeatureValueBindingConnector','checkFeatureOwnedCrossFeatureTypeFeaturing',
    'checkFeatureOwnedCrossFeatureSpecialization','checkFeatureOwnedCrossFeatureRedefinitionSpecialization',
    'deriveFeatureOwnedCrossSubsetting','validateFeatureOwnedCrossSubsetting',
    'validateCrossSubsettingCrossedFeature','validateCrossSubsettingCrossingFeature',
    'deriveFeatureType','typingFeatures','deriveFeatureFeaturingType','isCartesianProduct','asCartesianProduct',
    'deriveNamespaceOwnedMember','checkFeatureFeatureMembershipTypeFeaturing','isFeaturingType'}
clauses=[r for r in inventory['members'] if r['name'] in rules]
source_path=ROOT/'standards/artifacts/2026-04/Kernel_Semantic_Library-1.0.0.kpar'
with zipfile.ZipFile(source_path) as archive:
    source_name=next(n for n in archive.namelist() if n.endswith('Occurrences.kerml'))
    source=archive.read(source_name)

witnesses=[]
for path in ['Occurrences::Occurrence::incomingTransfersToSelf::target',
             'Occurrences::Occurrence::outgoingTransfersFromSelf::source']:
    end=m.find(path)
    domain=m.parents[m.parents[end]]
    assert kind(m.parents[end],'FeatureMembership') and end.get('isEnd')=='true'
    assert featuring(end)=={domain}
    assert not related(end,'CrossSubsetting','crossedFeature')
    ends=[v for v in owned_members(domain) if v.get('isEnd')=='true']
    assert len(ends)==2 and end in ends
    opposite=next(e for e in ends if e is not end)
    expression=selector(end)[0]
    binding=selector(end,True)[0]
    assert kind(expression,'Expression') and kind(m.parents[expression],'FeatureValue')
    assert kind(binding,'BindingConnector') and m.kind(m.parents[binding])=='OwningMembership'
    assert featuring(expression)==featuring(binding)=={domain}
    opposite_types, opposite_closure=types(opposite)
    end_types, end_closure=types(end)
    domain_types, domain_closure=types(domain)
    assert opposite_types==end_types=={m.find('Occurrences::Occurrence')}
    assert domain_types=={m.find('Transfers::Transfer')}
    assert {domain} != opposite_types
    # Literal OCL excludes the cross feature rather than its owning end. Thus both
    # end Features remain and the Cartesian branch is taken. Its exact set equality
    # also fails: asCartesianProduct(domain) includes domain.type, hence Transfer.
    literal_other_ends=[e for e in ends if e is not expression]
    assert literal_other_ends==ends
    all_end_types=set().union(*(types(e)[0] for e in ends))
    assert not domain_types.issubset(all_end_types)
    start=source.index(('var feature all '+m.name(domain)).encode())
    stop=source.index(b'\n\t\t}',start)+len(b'\n\t\t}')
    local={end,domain,opposite,expression,binding}|opposite_closure|end_closure|domain_closure
    witnesses.append(dict(source=dict(path=source_name,sha256=hashlib.sha256(source).hexdigest(),
        byte_range=[start,stop],text=source[start:stop].decode()),
        end=brief(end),domain=brief(domain),opposite=brief(opposite),
        literal_selector=[brief(e) for e in selector(end)],
        membership_exclusion_reading_selector=[brief(e) for e in selector(end,True)],
        required_value_domain=[brief(domain)],required_binary_cross_domain=[brief(e) for e in opposite_types],
        literal_cartesian_disproof=dict(other_ends=[brief(e) for e in literal_other_ends],
            required_product=[brief(e) for e in all_end_types],
            actual_product_contains=[brief(e) for e in domain_types],valid=False),
        prose_binary_cross_domain_valid=False,
        reference_local_facts=[m.record(e) for e in sorted(local,key=lambda e:(m.files[e],e.get(XID)))],
        owned_cross_subsetting=None,
        no_prior_cross_member='The source end owns only its redefinition and FeatureValue before implication. Multiplicities are excluded by the selector. No reviewed or ordinary rule prescribes adding/reordering a different owned cross member ahead of the value Expression.',
        complete_local_facts=True,unresolved_local_references=0))

pilot_path=OUT/'pilot/FeatureUtil.java'
pilot=pilot_path.read_text()
start=pilot.index('public static Feature getOwnedCrossFeatureOf(')
stop=pilot.index('\n\tpublic static boolean isOwnedCrossFeature',start)
excerpt=pilot[start:stop]
assert '!(element instanceof BindingConnector)' in excerpt
assert '!(element.getOwningMembership() instanceof FeatureValue)' in excerpt
preliminary=[]
for index,page in enumerate(PdfReader(OUT/'preliminary/KerML.pdf').pages):
    content=page.extract_text()
    if 'ownedCrossFeature()' in content and 'ownedMemberFeatures' in content:
        preliminary.append(dict(page=index+1,text=content))
issue=OUT/'issues/KERML11-1.html'
issue_text=BeautifulSoup(issue.read_text(encoding='utf8'),'html.parser').get_text(' ',strip=True)
assert 'Status: open' in issue_text
report=dict(format='agentique-cross-feature-authority-conflict/1',finding='KLCV9-F-001',
    issue=dict(key='KERML11-1',status='open',url='https://issues.omg.org/issues/KERML11-1',
        sha256=sha(issue),text=issue_text),
    normative=dict(path=inventory['file'],sha256=inventory['sha256'],formal=clauses),
    pinned_archive=dict(path=source_path.relative_to(ROOT).as_posix(),sha256=sha(source_path)),
    reference_release=json.loads((OUT/'release-head.json').read_text())['sha'],
    reference_inputs='reference-binding-authority.json#/reference/inputs',
    witnesses=witnesses,
    pilot=dict(commit=json.loads((OUT/'pilot-head.json').read_text())['sha'],
        path=pilot_path.relative_to(ROOT).as_posix(),sha256=sha(pilot_path),excerpt=excerpt,normative_authority=False),
    preliminary=dict(path='preliminary/KerML.pdf',sha256=sha(OUT/'preliminary/KerML.pdf'),
        pages=preliminary,normative_authority=False),
    scope='Cross-feature selection/domain; not reference-binding endpoint conformance or KERML11-145 contextual results.',
    disposition='IndependentAuthorityConflict',
    covered_by_v1_v6=False,execution_required=False,correction_adopted=False,
    ordinary_inference_cannot_repair='The exact same selected member is required to have unequal exact featuring sets. More inferred typing, positional redefinition, reference resolution or KERML11-8 validation cannot change that equality. Replacing, excluding, reordering or reclassifying that member changes canonical semantics.',
    canonical_choice_required='Change ownedCrossFeature selection to exclude value Expressions and value BindingConnectors, or change cross/value domain and ownership semantics. Neither is authorized by v1-v6.',
    stop_policy=dict(exercised_by_pinned_corpus=True,independent_complete_local_reproduction=True,
        unfinished_agentique_semantics=False,not_covered_by_v1_v6=True,materially_different_canonical_choice=True))
encoded=json.dumps(report,indent=2,ensure_ascii=False)+'\n'
target=OUT/'cross-feature-authority-conflict.json'
if '--check' in sys.argv:
    assert target.read_text(encoding='utf8')==encoded
else:
    target.write_text(encoded,encoding='utf8',newline='\n')
print('KLCV9-F-001 / KERML11-1: two pinned end-value witnesses; literal and prose-domain contradictions independently reproduced; no correction adopted')
