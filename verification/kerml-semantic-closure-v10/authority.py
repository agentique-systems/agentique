"""Offline KERML11-1 authority verification, independent of Rust queries.

--git additionally verifies acquired bytes against the actual fetched Git objects.
Acquisition is deliberately separate from this verifier and all builds.
"""
import hashlib
import json
import re
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET
from functools import cache
from bs4 import BeautifulSoup
from pypdf import PdfReader

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT/'verification/kerml-library-content-errata-publication-v5'))
from xmi import Model, XID

COMMIT = '553cf8205c19241c9127ab264f8372f5b58d3895'
JAVA = 'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/FeatureUtil.java'
TEST = 'org.omg.sysml.logic/src/test/java/org/omg/sysml/logic/KERML11_Ballot3_Tests.java'
sha = lambda data: hashlib.sha256(data).hexdigest()
object_id = lambda kind, data: hashlib.sha1(kind.encode()+b' '+str(len(data)).encode()+b'\0'+data).hexdigest()
commit = json.loads((OUT/'resolution/commit.json').read_text())
assert commit['sha'] == COMMIT
assert {f['filename'] for f in commit['files']} == {JAVA, TEST}
assert [(f['status'], f['additions'], f['deletions']) for f in commit['files']] == [('modified',6,2),('added',76,0)]

if '--git' in sys.argv:
    git = lambda *args: subprocess.check_output(['git','-C',str(ROOT/'.cache/kerml-v10-pilot'),*args])
    raw = git('cat-file','commit',COMMIT)
    assert object_id('commit', raw) == COMMIT
    diff = git('diff',COMMIT+'^',COMMIT,'--no-ext-diff','--binary','--full-index')
    for name,data in [('commit-object.txt',raw),('git.diff',diff)]:
        path = OUT/'resolution'/name
        if path.exists(): assert path.read_bytes() == data
        else: path.write_bytes(data)
    for f in commit['files']:
        for label, revision in [('before',COMMIT+'^'),('after',COMMIT)]:
            if label == 'before' and f['status'] == 'added': continue
            data = (OUT/'resolution'/label/f['filename']).read_bytes()
            assert data == git('show',revision+':'+f['filename'])
            if label == 'after': assert object_id('blob',data) == f['sha']

raw = (OUT/'resolution/commit-object.txt').read_bytes()
assert object_id('commit',raw) == COMMIT
before = (OUT/'resolution/before'/JAVA).read_bytes()
after = (OUT/'resolution/after'/JAVA).read_bytes()
test = (OUT/'resolution/after'/TEST).read_bytes()
for f in commit['files']:
    assert object_id('blob',(OUT/'resolution/after'/f['filename']).read_bytes()) == f['sha']
# Independently apply the reviewed source delta to the complete original blob.
expected = before.replace(b'import org.omg.sysml.lang.sysml.Behavior;', b'import org.omg.sysml.lang.sysml.Behavior;\nimport org.omg.sysml.lang.sysml.BindingConnector;')
expected = expected.replace(b'return !(namespace instanceof Feature) || !((Feature)namespace).isEnd()? null:', b'return !(namespace instanceof Feature feature) || \n\t\t\t\t\t!feature.isEnd() || \n\t\t\t\t\tfeature.getOwningType() == null? null:')
expected = expected.replace(b'\t\t\t\t\t\t!(element.getOwningMembership() instanceof FeatureMembership)&&', b'\t\t\t\t\t\t!(element instanceof BindingConnector) &&\n\t\t\t\t\t\t!(element.getOwningMembership() instanceof FeatureMembership) &&')
assert expected == after, 'Unexpected additional pilot implementation change'
test_text = test.decode()
assert test_text.index('addOwnedMemberTo(end1, connector)') < test_text.index('addOwnedMemberTo(end1, feature)')
assert 'assertEquals("cross feature", feature, crossFeature)' in test_text

acquisitions = json.loads((OUT/'acquisition.json').read_text())
for row in acquisitions:
    assert sha((OUT/row['path']).read_bytes()) == row['sha256']
inventory = json.loads((ROOT/'verification/kerml-semantic-closure-v6/metamodel-inventory.json').read_text())
normative_path=ROOT/inventory['file'].replace('\\','/')
assert sha(normative_path.read_bytes()) == inventory['sha256']
formal = [r for r in inventory['members'] if 'Cross' in (r['name'] or '') or r['name'] in {'typingFeatures','deriveFeatureType','deriveFeatureFeaturingType','deriveNamespaceOwnedMember','checkExpressionTypeFeaturing','checkFeatureValueBindingConnector'}]
normative = ET.parse(normative_path).getroot()
utype = '{http://www.omg.org/spec/XMI/20161101}type'
uid = '{http://www.omg.org/spec/XMI/20161101}id'
classes = {e.get(uid):e for e in normative.iter() if e.get(utype)=='uml:Class'}
names = {e.get('name'):e for e in classes.values()}
@cache
def conformance(name):
    seen, pending = set(), [names[name]] if name in names else []
    while pending:
        c = pending.pop()
        if c.get('name') in seen: continue
        seen.add(c.get('name'))
        pending.extend(classes[g.find('general').get(uid.replace('id','idref'))] for g in c if g.tag=='generalization')
    return seen

model = Model(ROOT/'verification/kerml-semantic-closure-v6/release/sysml.library.xmi.implied')
def is_kind(e, cls): return cls in conformance(model.kind(e))
def brief(e): return dict(id=e.get(XID), kind=model.kind(e), path=model.path(e), file=model.files[e])
def sequence(end):
    return [(m,v) for m in end if is_kind(m,'OwningMembership') for v in m if v.tag=='ownedRelatedElement']
def select(end, operational):
    owner_membership = model.parents[end]
    if end.get('isEnd','false') != 'true' or not is_kind(owner_membership,'FeatureMembership'):
        return None
    if not is_kind(model.parents[owner_membership],'Type'): return None
    for m,v in sequence(end):
        excluded = ['Multiplicity','MetadataFeature'] + (['BindingConnector'] if operational else ['FeatureValue'])
        if (is_kind(v,'Feature') and not any(is_kind(v,c) for c in excluded)
            and not is_kind(m,'FeatureMembership') and not (operational and is_kind(m,'FeatureValue'))):
            return v
    return None

old = json.loads((ROOT/'verification/kerml-semantic-closure-v9/cross-feature-authority-conflict.json').read_text())
witnesses = []
for historical in old['witnesses']:
    end = model.find(historical['end']['path'])
    published = select(end,False)
    assert published.get(XID) == historical['literal_selector'][0]['id']
    selected = select(end,True)
    assert selected is None
    witnesses.append(dict(end=brief(end), ordered_owned_members=[dict(membership=brief(m),member=brief(v)) for m,v in sequence(end)],
        published_selection=brief(published), operational_v7_selection=None,
        reference_cross_subsettings=[brief(r) for r in end if is_kind(r,'CrossSubsetting')],
        complete_local_domain_proof=historical))

# Check every current reference blob independently against the acquired release tree.
tree = json.loads((OUT/'release-tree.json').read_text())['tree']
reference = []
for path in sorted((ROOT/'verification/kerml-semantic-closure-v6/release/sysml.library.xmi.implied').rglob('*.kermlx')):
    suffix = path.relative_to(ROOT/'verification/kerml-semantic-closure-v6/release').as_posix()
    entry, = [r for r in tree if r['path']==suffix]
    assert object_id('blob',path.read_bytes()) == entry['sha']
    reference.append(dict(path=path.relative_to(ROOT).as_posix(), sha256=sha(path.read_bytes()), git_blob=entry['sha']))
assert len(reference) == 36

def excerpt(data):
    text = data.decode()
    start = text.index('public static Feature getOwnedCrossFeatureOf(')
    return text[start:text.index('\n\tpublic static boolean isOwnedCrossFeature',start)]
current_selector = excerpt((OUT/'pilot/FeatureUtil.java').read_bytes())
normalize = lambda s: re.sub(r'\s+', '', s)
# Current head rewrites the guard ternary as if/else; eligibility and order agree.
assert normalize(current_selector[current_selector.index('namespace.getOwnedMember()'):current_selector.index('findFirst().orElse(null);')]) == normalize(excerpt(after)[excerpt(after).index('namespace.getOwnedMember()'):excerpt(after).index('findFirst().orElse(null);')])
assert '!(namespace instanceof Feature feature)' in current_selector
assert '!feature.isEnd() || feature.getOwningType() == null' in current_selector
issue = BeautifulSoup((OUT/'issues/KERML11-1.html').read_text(encoding='utf8'),'html.parser').get_text(' ',strip=True)
assert 'Status: open' in issue
pages = []
for i,page in enumerate(PdfReader(OUT/'preliminary/KerML.pdf').pages):
    text = page.extract_text()
    if 'ownedCrossFeature()' in text and 'ownedMemberFeatures' in text:
        pages.append(dict(page=i+1,text=text))
report = dict(format='agentique-owned-cross-feature-authority/1', issue=dict(key='KERML11-1',status='open',updated='2026-04-21T01:22:00Z',text=issue),
    disposition='Project-authorized Operational KerML 1.0/v7; not final OMG adoption',
    normative=dict(path=inventory['file'],sha256=inventory['sha256'],formal=formal),
    resolution=dict(commit=COMMIT,parent=commit['parents'][0]['sha'],git_object_verified=True,
        diff_sha256=sha((OUT/'resolution/git.diff').read_bytes()),before=excerpt(before),after=excerpt(after),
        exact_files=commit['files'],regression=test_text),
    current_pilot=dict(commit=json.loads((OUT/'pilot-head.json').read_text())['sha'],same_predicates_and_order=True,excerpt=current_selector,change='Guard ternary refactored as if/else; no eligibility change'),
    reference=dict(commit=json.loads((OUT/'release-head.json').read_text())['sha'],inputs=reference),
    preliminary=dict(sha256=sha((OUT/'preliminary/KerML.pdf').read_bytes()),normative_authority=False,pages=pages),
    witnesses=witnesses, selection_order='Ordered ownedRelationship -> OwningMembership -> ownedRelatedElement; never member ID order',
    exclusion_semantics='Metaclass conformance; no origin-based or arbitrary implied-element exclusion')
encoded = json.dumps(report,indent=2,ensure_ascii=False)+'\n'
target = OUT/'authority-packet.json'
if '--check' in sys.argv: assert target.read_text(encoding='utf8') == encoded
else: target.write_text(encoded,encoding='utf8',newline='\n')
print('Verified exact Git object/delta/test, current selector, 36 reference blobs, formal/preliminary rules, and both ordered Occurrences witnesses.')
