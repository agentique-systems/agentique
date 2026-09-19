"""Compare resolved assertions with all 36 captured current reference XMI models.

Extraction uses XML and semantic paths, never correction-transform helpers or
reference IDs as Agentique identities. Unmatched declarations remain explicit.
"""
from collections import defaultdict
import hashlib
import json
from pathlib import Path
from evidence_io import read_json
from xmi import Model

OUT = Path(__file__).resolve().parent
directory = OUT / 'release/sysml.library.xmi'
model = Model(directory)
audit = read_json(OUT / 'operational-obligations-2/obligations.json')
oracle = []
artifacts = {p.name: dict(path=str(p.relative_to(OUT)).replace('\\', '/'),
                        sha256=hashlib.sha256(p.read_bytes()).hexdigest())
             for p in directory.rglob('*.kermlx')}
for element in model.files:
    if model.kind(element) != 'Redefinition':
        continue
    source = model.targets(element, 'redefiningFeature') or [model.parents[element]]
    target = model.targets(element, 'redefinedFeature')
    assert len(source) == len(target) == 1
    oracle.append(dict(artifact=artifacts[model.files[element]],
                       relationship=model.record(element),
                       source=model.path(source[0]), target=model.path(target[0])))
by_source = defaultdict(list)
for row in oracle:
    by_source[row['source']].append(row)
comparisons = []
unmatched = []
for row in audit['redefinition_resolutions']:
    matches = by_source[row['source_declaration']]
    if not matches:
        unmatched.append(dict(element=row['element'], document=row['document'],
                              source=row['source_declaration'], source_range=row['source_range']))
        continue
    actual = sorted({target['path'] for target in row['targets']})
    expected = sorted({match['target'] for match in matches})
    complete = row['result']['completeness'] == 'Complete'
    comparisons.append(dict(source=row['source_declaration'], element=row['element'],
        document=row['document'], source_range=row['source_range'],
        actual=actual, expected=expected, complete=complete,
        target_match=len(actual) == 1 and set(actual) <= set(expected),
        reference_relationships=[m['relationship']['id'] for m in matches]))
groups = []
for source in sorted({row['source'] for row in comparisons}):
    actual = sorted({target for row in comparisons if row['source'] == source for target in row['actual']})
    expected = sorted({row['target'] for row in by_source[source]})
    groups.append(dict(source=source, actual=actual, expected=expected, match=actual == expected))
report = dict(format='agentique-current-reference-comparison/1',
    reference_status='Non-normative current implementation corroboration; no portable canonical IDs claimed.',
    current_release=read_json(OUT / 'release-head.json')['sha'],
    files=len(artifacts), oracle=oracle, comparisons=comparisons, source_target_sets=groups,
    compared_assertions=len(comparisons), assertions_without_named_reference_match=unmatched,
    mismatched_assertions=sum(not row['target_match'] for row in comparisons),
    incomplete_compared_assertions=sum(not row['complete'] for row in comparisons),
    mismatched_target_sets=sum(not row['match'] for row in groups),
    accepted_semantic_publication=audit['publication_accepted'])
with (OUT / 'current-reference-comparison.json').open('x', encoding='utf8') as stream:
    json.dump(report, stream, indent=2)
    stream.write('\n')
print(json.dumps({key: report[key] for key in [
    'files', 'compared_assertions', 'mismatched_assertions',
    'incomplete_compared_assertions', 'mismatched_target_sets', 'accepted_semantic_publication']}))
assert not report['mismatched_assertions']
assert not report['incomplete_compared_assertions']
assert not report['mismatched_target_sets']
