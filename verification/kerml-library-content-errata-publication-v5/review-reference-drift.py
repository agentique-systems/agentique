"""Isolate current-reference changes outside the reviewed four-library repair."""
import hashlib
import json
from pathlib import Path
from evidence_io import read_json
from xmi import Model

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
current = Model(OUT / 'release/sysml.library.xmi')
old = Model(OUT / 'historical/sysml.library.xmi')
comparison = read_json(OUT / 'current-reference-comparison.json')
audit = read_json(OUT / 'operational-obligations-2/obligations.json')
mismatches = [row for row in comparison['source_target_sets'] if not row['match']]
assert len(mismatches) == 1
assert mismatches[0]['source'] == 'Collections::OrderedMap::elements'
assert comparison['mismatched_assertions'] == comparison['incomplete_compared_assertions'] == 0

def targets(model, element, kind, property):
    return [model.path(t) for r in element if model.kind(r) == kind for t in model.targets(r, property)]

ordered_map = 'Collections::OrderedMap'
old_supers = targets(old, old.find(ordered_map), 'Subclassification', 'superclassifier')
new_supers = targets(current, current.find(ordered_map), 'Subclassification', 'superclassifier')
assert old_supers == ['Collections::Map']
assert set(new_supers) == {'Collections::Map', 'Collections::OrderedCollection'}
path = ROOT / 'standards/libraries/Data-Type-Library/Kernel Data Type Library/Collections.kerml'
data = path.read_bytes()
assert b'datatype OrderedMap :> Map {' in data
assert b'feature elements: KeyValuePair[0..*] ordered :>> Map::elements {' in data
start = data.index(b'datatype OrderedMap :> Map {')
drift = [dict(source=ordered_map, classification='Unrelated current-reference superclass and redefinition addition.',
    pinned_source=dict(path=str(path.relative_to(ROOT)), sha256=hashlib.sha256(data).hexdigest(),
                       range=[start, len(data)], text=data[start:].decode()),
    historical=old.record(old.find(ordered_map)), current=current.record(current.find(ordered_map)),
    raw_target_set_comparison=mismatches[0], imported_into_v3=False)]

expected_missing = {'Occurrences::Occurrence::spaceBoundary::isClosed',
                    'TransitionPerformances::TransitionPerformance::accept::receiver'}
assert {row['source'] for row in comparison['assertions_without_named_reference_match']} == expected_missing
for row in comparison['assertions_without_named_reference_match']:
    parent, name = row['source'].rsplit('::', 1)
    old_element = old.find(row['source'])
    new_parent = current.find(parent)
    assert not any(current.name(child) == name for rel in new_parent for child in rel
                   if child.tag == 'ownedRelatedElement')
    assertion = next(a for a in audit['redefinition_resolutions'] if a['element'] == row['element'])
    drift.append(dict(source=row['source'], classification='Named source declaration absent from current reference parent.',
                      pinned_assertion={k: assertion[k] for k in ['document', 'source', 'source_range', 'sha256', 'targets']},
                      historical=old.record(old_element), current_parent=current.record(new_parent),
                      imported_into_v3=False))

report = dict(format='agentique-current-reference-drift-review/1',
    compared_assertions=comparison['compared_assertions'], exact_assertion_target_matches=True,
    current_reference_has_unrelated_changes=True, raw_comparison_retained='current-reference-comparison.json',
    reviewed_drift=drift, semantic_quality_waivers=0, accepted_semantic_publication=False)
with (OUT / 'reference-drift-review.json').open('x', encoding='utf8') as stream:
    json.dump(report, stream, indent=2)
    stream.write('\n')
print(json.dumps(dict(compared_assertions=report['compared_assertions'],
                     unrelated_current_reference_changes=len(drift), imported_changes=0)))
