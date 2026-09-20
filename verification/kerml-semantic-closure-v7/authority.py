"""Independently join pinned XMI, ZIP declarations, issue and reference evidence."""
import hashlib
import html
import json
from pathlib import Path
import re
import sys
import zipfile

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
OLD = ROOT/'verification/kerml-semantic-closure-v6'
sha = lambda data: hashlib.sha256(data).hexdigest()
rules = [
    ('checkFeatureSubobjectSpecialization',205,'Objects::Object::subobjects','Feature','subobject'),
    ('checkStepSubperformanceSpecialization',205,'Performances::Performance::subperformances','Step','subperformance'),
    ('checkFeaturePortionSpecialization',206,'Occurrences::Occurrence::portions','Feature','portion'),
    ('checkFeatureSuboccurrenceSpecialization',206,'Occurrences::Occurrence::suboccurrences','Feature','suboccurrence'),
    ('checkStepOwnedPerformanceSpecialization',207,'Objects::Object::ownedPerformances','Step','ownedPerformance'),
    ('checkStepEnclosedPerformanceSpecialization',207,'Performances::Performance::enclosedPerformances','Step','enclosedPerformance'),
]
inventory = json.loads((OLD/'metamodel-inventory.json').read_text())
assert sha((ROOT/inventory['file'].replace('\\','/')).read_bytes()) == inventory['sha256']
libraries = json.loads((ROOT/'standards/normative/sysml-2.0/library-set.json').read_text())
archive = next(a for a in libraries['artifacts'] if a['source'].endswith('Semantic-Library.kpar'))
assert sha((ROOT/archive['path']).read_bytes()) == archive['sha256']
mapping_path = OUT/'pilot/org.omg.sysml.logic/src/main/java/org/omg/sysml/util/ImplicitGeneralizationMap.java'
mapping = mapping_path.read_text()
head = json.loads((OUT/'pilot-head.json').read_text())['sha']
tree = json.loads((OUT/'pilot-root-tree.json').read_text())
assert not tree.get('truncated')
assert tree['sha'] == json.loads((OUT/'pilot-head.json').read_text())['commit']['tree']['sha']
test_entries = [e for e in tree['tree'] if 'test' in e['path'].lower() and e['path'].endswith('.kerml.xt')]
assert len(test_entries) == 303
map_relative = mapping_path.relative_to(OUT/'pilot').as_posix()
for entry in test_entries + [e for e in tree['tree'] if e['path']==map_relative]:
    data=(OUT/'pilot'/entry['path']).read_bytes()
    assert hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()==entry['sha'],entry['path']
rows = []
with zipfile.ZipFile(ROOT/archive['path']) as z:
    for name, issue, corrected, cls, role in rules:
        rule = next(r for r in inventory['members'] if r['name'] == name)
        formal = next(b['body'] for b in rule['bodies'] if b.get('language') == 'OCL2.0')
        prose = next(b['body'] for b in rule['bodies'] if not b.get('language'))
        wrong, = re.findall(r"specializesFromLibrary\('([^']+)'\)", formal)
        issue_path = OUT/f'issues/KERML11-{issue}.html'
        issue_text = issue_path.read_text(encoding='utf8')
        assert 'status-public-open">open</span>' in issue_text and name in issue_text
        summary_start = issue_text.index('Summary:')
        summary_end = issue_text.index('Reported:', summary_start)
        issue_summary = ' '.join(html.unescape(re.sub('<[^>]*>', ' ', issue_text[summary_start:summary_end])).split())
        if issue != 206:
            assert corrected in issue_summary and wrong in issue_summary
        else:
            assert '"Occurence"' in issue_summary and '"Occurences"' in issue_summary
        pkg, owner, feature = corrected.split('::')
        entry = next(e for e in archive['entries'] if e['path'].endswith('/'+pkg+'.kerml'))
        source = z.read(entry['path'])
        assert sha(source) == entry['sha256']
        text = source.decode('utf8')
        # Explicit source declarations; full ownership is additionally checked by
        # the canonical path binding tests, never inferred from display names.
        package = re.search(r'standard\s+library\s+package\s+'+pkg+r'\b',text)
        declaration = re.search(r'(?m)^\s*(?:abstract\s+)?(?:class|struct|behavior)\s+'+owner+r'\b[^\n]*',text)
        member = re.search(r'(?m)^\s*(?:(?:composite|portion)\s+)?(?:feature|step)\s+(?:all\s+)?'+feature+r'\s*:[^\n]*',text)
        assert package and declaration and member
        # The target is in this declaration's direct brace scope; comments do
        # not introduce structural braces in the inspected prefix.
        between = re.sub(r'/\*.*?\*/','',text[declaration.end():member.start()],flags=re.S)
        assert between.count('{') == between.count('}')
        reference = f'put({cls}Impl.class, "{role}", "{corrected}")'
        assert reference in mapping
        tests = []
        contextual_tests = []
        for test in sorted((OUT/'pilot').rglob('*.kerml.xt')):
            body = test.read_text(encoding='utf8')
            hits = [dict(line=i+1,text=line) for i,line in enumerate(body.splitlines()) if corrected in line or name in line]
            if hits:
                tests.append(dict(path=test.relative_to(ROOT).as_posix(),sha256=sha(test.read_bytes()),matches=hits))
            elif re.search(r'\b'+re.escape(feature)+r'\b',body):
                contextual_tests.append(dict(path=test.relative_to(ROOT).as_posix(),sha256=sha(test.read_bytes()),
                    matches=[dict(line=i+1,text=line) for i,line in enumerate(body.splitlines()) if re.search(r'\b'+re.escape(feature)+r'\b',line)],
                    scope='Contextual spelling/scoping fixture only; not an assertion of this formal constraint target. Source inspected, not executed.'))
        rows.append(dict(rule_id=next(v for k,v in rule['attributes'].items() if k.endswith('}id')),
            rule=name,published_body=formal,prose_requirement=prose,published_target=wrong,
            operational_target=corrected,issue=f'KERML11-{issue}',issue_status='open',
            issue_evidence=dict(path=issue_path.relative_to(ROOT).as_posix(),sha256=sha(issue_path.read_bytes()),
                summary=issue_summary,
                spelling_note='The issue itself writes Occurence/Occurences. The exact Occurrences spelling is independently established by pinned declaration, normative prose and reference map.' if issue==206 else None),
            authority=dict(artifact=inventory['file'],sha256=inventory['sha256'],xml=rule['xml']),
            pinned_declaration=dict(archive=archive['path'],archive_sha256=archive['sha256'],entry=entry['path'],
                sha256=entry['sha256'],package=package.group(),owner=declaration.group().strip(),
                text=member.group().strip(),range=[len(text[:member.start()].encode()),len(text[:member.end()].encode())]),
            reference=dict(commit=head,file=mapping_path.relative_to(ROOT).as_posix(),sha256=sha(mapping_path.read_bytes()),mapping=reference),
            reference_tests=tests,reference_contextual_tests=contextual_tests,
            reference_test_scope='All 303 KerML Xpect .kerml.xt files in the frozen recursive tree, captured in acquisition.json. No direct fully qualified target/rule assertion found when reference_tests is empty. Sources inspected, not executed.',
            authority_scope='Project-authorized exact formal library target replacement only; retained OCL is not executed as OCL. No changes to antecedent, source declarations, or generic resolution.'))
report = dict(format='agentique-formal-target-authority/1',profile='agentique-kerml-1.0-operational/5',
    reference_test_inventory=dict(commit=head,complete_tree_sha=tree['sha'],ker_ml_xpect_fixtures=len(test_entries),
        fixture_and_map_git_blob_hashes_verified=True,executed=False),constraints=rows)
encoded = json.dumps(report,indent=2)+'\n'
target = OUT/'authority-matrix.json'
if '--check' in sys.argv:
    assert target.read_text(encoding='utf8') == encoded
else:
    target.write_text(encoded,encoding='utf8',newline='\n')
print('Six independent formal-target authority rows verified; pinned declarations and reference map agree.')
