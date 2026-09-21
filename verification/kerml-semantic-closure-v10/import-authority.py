"""Independent imported/owned name-collision proof for KERML11-75.

Canonical endpoint identities are cross-checked against the source refinement
report, not treated as a second complete resolver. The registered witnesses use
direct public declarations in one explicitly imported root package. Their
membership/name/conformance predicate is independently evaluated from pinned
source and normative XMI; no Rust query is called.
"""
import json
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from corpus import *

p = Pinned()
audit = json.loads((OUT/'full-v7-producers.json').read_text(encoding='utf8'))
answers = {r['relationship']: r for r in audit['reference_answers']}
golden = json.loads((ROOT/'standards/generated/kerml-1.0/full.golden.json').read_text(encoding='utf8'))
literals = {int(row['descriptor_id'].replace('-', ''), 16): row['entity']['name']
            for cls in golden['classifiers'].values() for row in cls['literals']}

def visibility(e):
    value = p.scalar(e, 'visibility')
    if value is None: return 'public'
    return literals[int(re.fullmatch(r'Scalar\(Enumeration\(EnumerationLiteralId\((\d+)\)\)\)', value)[1])]

def endpoint(e):
    row = answers[e]
    assert row['completeness'] == 'Complete' and len(row['candidates']) == 1, row
    return row['candidates'][0]

def member(e):
    stored = p.refs(e, 'ownedRelatedElement') + p.refs(e, 'memberElement')
    if stored:
        assert len(stored) == 1
        return stored[0]
    return endpoint(e)

def member_names(e):
    v = member(e)
    if p.is_kind(e, 'OwningMembership'):
        return {s for s in [p.scalar(v, 'declaredName'), p.scalar(v, 'declaredShortName')] if s}
    return {s for s in [p.scalar(e, 'memberName'), p.scalar(e, 'memberShortName')] if s}

def own(e): return [r for r in p.refs(e, 'ownedRelationship') if p.is_kind(r, 'Membership')]
def imports(e): return [r for r in p.refs(e, 'ownedRelationship') if p.is_kind(r, 'Import')]

def imported(e, excluded):
    assert not p.scalar(e, 'isRecursive') and not p.scalar(e, 'isImportAll')
    target = endpoint(e)
    if p.is_kind(e, 'NamespaceImport'):
        return [] if target in excluded else visible(target, excluded)
    name = answers[e]['name']
    assert len(name) == 2
    namespace = p.find(name[0])
    assert p.is_kind(namespace, 'Package')
    matches = [r for r in visible(namespace, excluded) if member(r) == target and name[-1] in member_names(r)]
    assert len(matches) == 1, (e, name, matches)
    return matches

def visible(namespace, excluded):
    assert p.is_kind(namespace, 'Package')
    result = [r for r in own(namespace) if visibility(r) == 'public']
    for r in imports(namespace):
        if visibility(r) == 'public': result += imported(r, excluded | {namespace})
    return list(dict.fromkeys(result))

rows = []
for namespace in p.records:
    if not p.is_kind(namespace, 'Package'): continue
    owned = own(namespace)
    for imp in imports(namespace):
        for m in imported(imp, {namespace}):
            for local in owned:
                shared = member_names(m) & member_names(local)
                a, b = member(m), member(local)
                if shared and a != b and (p.is_kind(a, p.records[b]['metaclass']) or p.is_kind(b, p.records[a]['metaclass'])):
                    rows.append(dict(namespace=p.path(namespace), import_=imp, imported_membership=m,
                                     imported=p.path(a), local_membership=local, local=p.path(b), names=sorted(shared)))
assert len(rows) == 5
assert {r['namespace'] for r in rows} == {'VectorFunctions'}
assert {n for r in rows for n in r['names']} == {'+', '-', '*', 'sum', 'sum0'}
namespace = p.find('VectorFunctions')
target = p.find('NumericalFunctions')
imp = rows[0]['import_']
assert p.records[imp]['source']['text'] == 'private import NumericalFunctions::*;'
assert p.parents[imp] == namespace and endpoint(imp) == target
library_root = p.owner(target)
assert p.is_kind(target, 'LibraryPackage') and p.is_kind(library_root, 'Namespace')
assert p.scalar(library_root, 'declaredName') is None and p.parents.get(library_root) is None
for row in rows:
    imported_membership, local = row['imported_membership'], row['local_membership']
    assert p.parents[imported_membership] == target and visibility(imported_membership) == 'public'
    assert p.parents[local] == namespace
    assert p.records[member(imported_membership)]['metaclass'] == p.records[member(local)]['metaclass'] == 'Function'
    row['facts'] = dict(imported_membership=p.brief(imported_membership),
                       imported_member=p.brief(member(imported_membership)),
                       local_membership=p.brief(local), local_member=p.brief(member(local)),
                       imported_visibility=visibility(imported_membership),
                       same_metaclass=True, is_distinguishable=False,
                       literal_includes_imported_membership=True,
                       prose_excludes_imported_membership=True)

m = reference()
def reference_members(namespace):
    result = []
    for relationship in namespace:
        if not kind(m, relationship, 'Membership'): continue
        values = [v for v in relationship if v.tag == 'ownedRelatedElement'] or m.targets(relationship, 'memberElement')
        assert len(values) == 1
        value = values[0]
        names = ({value.get('declaredName'), value.get('declaredShortName')} if kind(m, relationship, 'OwningMembership')
                 else {relationship.get('memberName'), relationship.get('memberShortName')}) - {None, ''}
        result.append((relationship, value, names))
    return result

ref_namespace, ref_target = m.find('VectorFunctions'), m.find('NumericalFunctions')
ref_imports = [r for r in ref_namespace if kind(m, r, 'NamespaceImport') and m.targets(r, 'importedNamespace') == [ref_target]]
assert len(ref_imports) == 1
reference_rows = []
for row in rows:
    name = row['names'][0]
    local = [(r, v) for r, v, ns in reference_members(ref_namespace) if name in ns]
    imported_members = [(r, v) for r, v, ns in reference_members(ref_target) if name in ns]
    assert len(local) == len(imported_members) == 1
    local_r, local_v = local[0]
    imported_r, imported_v = imported_members[0]
    assert m.kind(local_v) == m.kind(imported_v) == 'Function'
    reference_rows.append(dict(name=name, local_membership=brief(m, local_r), local_member=brief(m, local_v),
                               imported_membership=brief(m, imported_r), imported_member=brief(m, imported_v)))

formal = [r for r in inventory['members'] if (r['owner'], r['name']) in [
    ('Namespace', 'importedMemberships'), ('NamespaceImport', 'importedMemberships'),
    ('Namespace', 'visibleMemberships'), ('Namespace', 'membershipsOfVisibility'),
    ('Membership', 'isDistinguishableFrom'), ('Namespace', 'deriveNamespaceImportedMembership')]]
operation = next(r for r in formal if (r['owner'], r['name']) == ('Namespace', 'importedMemberships'))
ocl = [b['body'] for b in operation['bodies'] if b.get('language') == 'OCL2.0']
assert ocl == ['ownedImport.importedMemberships(excluded->including(self))']
assert any('collisions' in b.get('body', '') and 'ownedMembership' in b.get('body', '') for b in operation['bodies'])
pilot_paths = ['pilot/NamespaceAdapter.java', 'pilot/NamespaceUtil.java', 'pilot/ImportAdapter.java', 'pilot/NamespaceImportAdapter.java']
adapter = (OUT/pilot_paths[-1]).read_text(encoding='utf8')
assert 'importedMemberships.addAll(namespaceMembership);' in adapter
assert 'isDistinguishableFrom' not in adapter
issue = next(i for i in json.loads((OUT/'issue-inventory.json').read_text(encoding='utf8'))['issues'] if i['key'] == 'KERML11-75')
# Keep only frozen tracker fields, avoiding a dependency on sweep dispositions.
issue = {k: issue[k] for k in ['key', 'title', 'status', 'url', 'anchor', 'text', 'updated']}
search_path = OUT/'resolution/search-KERML11-75.json'
search = json.loads(search_path.read_text(encoding='utf8'))
assert not search['incomplete_results'] and search['total_count'] == len(search['items'])
report = dict(format='agentique-v10-import-authority/1', issue=issue, formal=formal,
              source_archive_sha256=digest(p.archive), source_refinement_audit_sha256=digest(OUT/'full-v7-producers.json'),
              namespace=dict(id=namespace, path=p.path(namespace)),
              imported_namespace=dict(id=target, path=p.path(target)), import_=p.brief(imp),
              witnesses=rows, package_scopes_scanned=sum(p.is_kind(e, 'Package') for e in p.records),
              local_contradiction_independently_proven=True, exhaustive_import_closure_proven=False,
              reference=dict(import_=m.record(ref_imports[0]), witnesses=reference_rows,
                  serialized_derived_import_membership=False,
                  behavior='Reference XMI retains all five colliding source pairs. Pinned current pilot NamespaceAdapter delegates to NamespaceImportAdapter, which appends every visible membership without a distinguishability filter; this corroborates the literal OCL result, not the conflicting prose.',
                  pilot_sources={path:digest(OUT/path) for path in pilot_paths}),
              issue_associated_resolution=dict(source=search_path.relative_to(ROOT).as_posix(), sha256=digest(search_path),
                  total_count=search['total_count'], interpretation='No issue-labelled resolution commit found; this does not exclude unlabelled changes or establish OMG adoption.'),
              correction_applied=False,
              conclusion='Five direct public NumericalFunctions memberships must both be included by the published OCL and excluded by its prose when imported into VectorFunctions. The predicates depend only on canonical ownership, names and metaclass conformance. Neither runtime evaluation nor unresolved inherited lookup is needed. KERML11-1 and earlier profiles do not correct importedMemberships.')
encoded = json.dumps(report, indent=2, ensure_ascii=False)+'\n'
path = OUT/'import-authority.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8') == encoded
else: path.write_text(encoded, encoding='utf8', newline='\n')
print('KERML11-75 independently reproduced on five direct imported/owned membership collisions; literal OCL and prose disagree. No correction applied.')
