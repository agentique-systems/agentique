"""Independent, offline proof of the pinned Objects source conflict.

Read actual resolved reference-XMI edges, not Agentique model IDs or query answers.
This is a projection of the published redefinition-suppression operation onto the
two conflicting features, not a second general-purpose KerML implementation.
"""
import difflib
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

OUT = Path(__file__).resolve().parent
ROOT = OUT.parent.parent
OLD = OUT/'separate-authority/release-2024-12'
CURRENT = OUT/'release'
SUFFIX = Path('Kernel Libraries/Kernel Semantic Library')
XID = '{http://www.omg.org/XMI}id'
XSI = '{http://www.w3.org/2001/XMLSchema-instance}type'


def artifact(path):
    return dict(path=path.relative_to(ROOT).as_posix(),
                sha256=hashlib.sha256(path.read_bytes()).hexdigest(), bytes=path.stat().st_size)


class Oracle:
    def __init__(self, directory):
        self.roots = {p.name: ET.parse(p).getroot() for p in directory.glob('*.kermlx')}
        self.files = {e: f for f, root in self.roots.items() for e in root.iter()}
        self.parents = {c: p for root in self.roots.values() for p in root.iter() for c in p}
        self.ids = {(self.files[e], e.get(XID)): e for e in self.files if e.get(XID)}

    def target(self, e, prop):
        if e.get(prop):
            return self.ids[(self.files[e], e.get(prop))]
        href = next(c.get('href') for c in e if c.tag == prop)
        filename, identifier = href.split('#')
        return self.ids[(Path(filename).name, identifier)]

    def relationships(self, e, kind):
        return [r for r in e if r.get(XSI) == 'sysml:'+kind]

    def redefined(self, e):
        return [self.target(r, 'redefinedFeature') for r in self.relationships(e, 'Redefinition')]

    def closure(self, e):
        seen = set()
        pending = [e]
        while pending:
            at = pending.pop()
            if at not in seen:
                seen.add(at)
                pending.extend(self.redefined(at))
        return seen

    def name(self, e):
        if e.get('declaredName'):
            return e.get('declaredName')
        try:
            targets = self.redefined(e)
        except KeyError:
            # Unrelated declarations outside this two-file proof are not used.
            return '<uncaptured-external-name>'
        return self.name(targets[0]) if targets else ''

    def qname(self, e):
        parts = []
        while e is not None:
            if e.tag.endswith('ownedRelatedElement') or e.get('declaredName'):
                name = self.name(e)
                if name:
                    parts.append(name)
            e = self.parents.get(e)
        return '::'.join(reversed(parts))

    def find(self, path):
        matches = [e for e in self.files if e.tag == 'ownedRelatedElement' and self.name(e) and self.qname(e) == path]
        assert len(matches) == 1, (path, len(matches))
        return matches[0]

    def own_features(self, e):
        return [(r, c) for r in self.relationships(e, 'FeatureMembership')
                for c in r if c.tag == 'ownedRelatedElement']

    def record(self, e):
        return dict(file=self.files[e], id=e.get(XID), qualified_name=self.qname(e),
                    xml=ET.tostring(e, encoding='unicode'))


old_source = OLD/'sysml.library'/SUFFIX/'Objects.kerml'
pinned = ROOT/'standards/libraries/Semantic-Library/Kernel Semantic Library/Objects.kerml'
new_source = CURRENT/'sysml.library'/SUFFIX/'Objects.kerml'
assert old_source.read_bytes() == pinned.read_bytes(), 'Historical oracle must match pinned source exactly'
oracle = Oracle(OLD/'sysml.library.xmi'/SUFFIX)
owner = oracle.find('Objects::StructuredSpaceObject')
cells = oracle.find('Objects::StructuredSpaceObject::structuredSpaceObjectCells')
outer = oracle.find('Objects::StructuredSpaceObject::innerSpaceDimension')
assert oracle.target(oracle.relationships(cells, 'FeatureTyping')[0], 'type') is owner
rows = []
for feature, general in [('faces', 'Surface'), ('edges', 'Curve'), ('vertices', 'Point')]:
    specific = oracle.find('Objects::StructuredSpaceObject::'+feature)
    classifier = oracle.find('Objects::'+general)
    inner = oracle.find('Objects::'+general+'::innerSpaceDimension')
    typing = oracle.relationships(specific, 'FeatureTyping')
    subsetting = oracle.relationships(specific, 'Subsetting')
    assert len(typing) == len(subsetting) == 1
    assert oracle.target(typing[0], 'type') is classifier
    assert oracle.target(subsetting[0], 'subsettedFeature') is cells
    members = [next(m for m, f in oracle.own_features(classifier) if f is inner),
               next(m for m, f in oracle.own_features(owner) if f is outer)]
    candidates = [inner, outer]
    closures = [oracle.closure(c) for c in candidates]
    own_redefinitions = {target for _, f in oracle.own_features(specific) for target in oracle.redefined(f)}
    # Published Type::removeRedefinedFeatures: another membership redefines this
    # feature, or a locally owned redefinition targets its redefinition closure.
    retained = [c for i, c in enumerate(candidates)
                if not any(members[j] is not members[i] and c in closures[j] for j in range(2))
                and not (own_redefinitions & closures[i])]
    assert retained == candidates
    assert inner is not outer and oracle.name(inner) == oracle.name(outer) == 'innerSpaceDimension'
    assert all(m.get('visibility', 'public') == 'public' for m in members)
    assert all(c.get('direction') is None and c.get('isEnd', 'false') == 'false' for c in candidates)
    assert oracle.redefined(inner) == oracle.redefined(outer)
    rows.append(dict(source_declaration=oracle.qname(specific),
                     reference_xmi_source_id=specific.get(XID),
                     explicit_general_edges=[oracle.record(r) for r in typing+subsetting],
                     candidate_memberships=[m.get(XID) for m in members],
                     candidates=[oracle.record(c) for c in candidates],
                     common_redefined_base=[oracle.record(c) for c in oracle.redefined(inner)],
                     candidate_count=2, same_metaclass=True, public=True,
                     positional_end_or_parameter_redefinition_applicable=False,
                     distinguishable=False,
                     operational_kerml11_140_path_applicable=False))

diff = ''.join(difflib.unified_diff(pinned.read_text().splitlines(keepends=True),
                                   new_source.read_text().splitlines(keepends=True),
                                   fromfile='pinned Objects.kerml', tofile='current reference Objects.kerml'))
diff_path = OUT/'separate-authority/Objects-pinned-vs-current.diff'
if diff_path.exists():
    assert diff_path.read_text() == diff
else:
    with diff_path.open('x', encoding='utf-8', newline='\n') as stream:
        stream.write(diff)
current = Oracle(CURRENT/'sysml.library.xmi'/SUFFIX)
repairs = []
for name in ['StructuredSurface', 'StructuredCurve', 'StructuredPoint']:
    repaired = current.find('Objects::StructuredSpaceObject::'+name+'::innerSpaceDimension')
    targets = current.redefined(repaired)
    assert len(targets) == 2
    repairs.append(dict(source_declaration=current.qname(repaired),
                        explicit_redefined_features=[current.qname(t) for t in targets],
                        source_xml=current.record(repaired)))
formal = ROOT/'standards/normative/kerml-1.0/KerML.xmi'
formal_root = ET.parse(formal).getroot()
authority = [ET.tostring(e, encoding='unicode') for e in formal_root.iter()
             if e.get('name') in ['validateNamespaceDistinguishibility', 'removeRedefinedFeatures',
                                  'allRedefinedFeaturesOf', 'inheritedMemberships', 'deriveFeatureName']]
assert any('removeRedefinedFeatures' in s for s in authority)
report = dict(format='agentique-pinned-objects-authority-conflict/1',
    status='separate-authority-conflict-publication-blocked',
    finding='KNRV4-F-001',
    authority_status='KerML 1.0 formal rules applied to pinned source; reference XMI is non-normative corroboration',
    historical_source_byte_identical_to_pinned=True,
    historical_release_commit='e0ccd90b5567f873f99ec6afe62e3502a9f63c47',
    current_release_commit='fb97b754f29588b8e9c7a35f370880cd15eb29e7',
    artifacts=[artifact(p) for p in [pinned, old_source, new_source, formal, diff_path]
               +list((OLD/'sysml.library.xmi'/SUFFIX).glob('*.kermlx'))],
    source_preservation='No pinned source or canonical library declarations modified to apply the later repair',
    published_authority_xml=authority, witnesses=rows, current_reference_explicit_repairs=repairs,
    related_issue=dict(key='KERML11-72', status='Open', updated='2025-07-09T02:55 GMT',
                       relationship='General multiple-inheritance anomaly only; not authority for a repair of these exact source declarations',
                       artifact=artifact(OUT/'separate-authority/KERML11-72.html')),
    chronology_note='release-before-correction contains the 2025-07 repair, before the 2026 refinement; it is NOT the uncorrected pinned source',
    disposition='Do not choose one sibling redefinition, alter normal resolution, weaken distinguishability, copy current sources, or fabricate missing common redefinitions. A separate reviewed library-source correction is required before acceptance.')
packet_path = OUT/'separate-authority/authority-packet.json'
if packet_path.exists():
    assert json.loads(packet_path.read_text()) == report
else:
    with packet_path.open('x', encoding='utf-8') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
print(json.dumps(dict(witnesses=len(rows), all_indistinguishable=True, current_explicit_repairs=len(repairs),
                     pinned_source_matches_historical_release=True, publication_blocked=True)))
