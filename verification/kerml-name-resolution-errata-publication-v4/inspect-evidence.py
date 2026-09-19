"""Independent XML/byte inspection; no Agentique resolver or generated IDs."""
import hashlib
import difflib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET
import zipfile

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
XSI = '{http://www.w3.org/2001/XMLSchema-instance}type'
XID = '{http://www.omg.org/XMI}id'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


rows = []
for path in (OUT/'release/sysml.library.xmi/Kernel Libraries/Kernel Semantic Library').glob('*.kermlx'):
    root = ET.parse(path).getroot()
    parents = {c: p for p in root.iter() for c in p}
    ids = {e.get(XID): e for e in root.iter() if e.get(XID)}

    def qname(e):
        names = []
        while e is not None:
            name = e.get('declaredName')
            if not name and e.tag == 'ownedRelatedElement':
                targets = [c.get('redefinedFeature') for c in e if c.get(XSI) == 'sysml:Redefinition']
                target = next((ids[t] for t in targets if t in ids and ids[t].get('declaredName')), None)
                if target is not None:
                    name = target.get('declaredName')
            if name:
                names.append(name)
            e = parents.get(e)
        return '::'.join(reversed(names))

    for e in root.iter():
        if e.get(XSI) != 'sysml:Redefinition':
            continue
        target = e.get('redefinedFeature')
        source = e.get('redefiningFeature')
        external = next((c.get('href') for c in e if c.tag == 'redefinedFeature'), None)
        rows.append(dict(artifact=str(path.relative_to(OUT)).replace('\\', '/'), sha256=digest(path),
                         relationship_id=e.get(XID), source_id=source,
                         source_declaration=qname(ids[source]) if source in ids else qname(parents[e]),
                         target_id=target, target=qname(ids[target]) if target in ids else external,
                         authority_status='non-normative reference implementation interchange',
                         xml=ET.tostring(e, encoding='unicode')))
exact = [r for r in rows if any(n in r['source_declaration'] for n in
                              ['beforeTimeSlice::monitoredFeature', 'afterSnapshot::monitoredFeature'])]
assert len(exact) == 2, exact
assert all(r['target_id'] == '6b06695b-e0d3-5d4e-aca5-af32f4ad40d8' for r in exact)
(OUT/'reference-xmi-targets.json').write_text(json.dumps(rows, indent=2)+'\n')

notes = []
for r in json.loads((OUT/'pilot-releases.json').read_bytes()):
    if r['tag_name'] in ['2025-11', '2025-10', '2025-09.1']:
        notes.append(dict(tag=r['tag_name'], published_at=r['published_at'], url=r['html_url'], body=r['body']))
(OUT/'redefinition-release-notes.json').write_text(json.dumps(notes, indent=2)+'\n')
matches = []
for archive in (ROOT/'standards/artifacts').rglob('*.kpar'):
    with zipfile.ZipFile(archive) as z:
        for name in z.namelist():
            if Path(name).name in ['FeatureReferencingPerformances.kerml', 'Observation.kerml', 'SpatialFrames.kerml']:
                data = z.read(name)
                pilot = OUT/'pilot/org.omg.kerml.xpect.tests/library'/Path(name).name
                release = OUT/'release/sysml.library/Kernel Libraries/Kernel Semantic Library'/Path(name).name
                normalized = data.decode().replace('\r\n', '\n')
                release_text = release.read_text()
                matches.append(dict(archive=str(archive.relative_to(ROOT)).replace('\\', '/'), entry=name,
                                    pinned_sha256=hashlib.sha256(data).hexdigest(),
                                    pilot_sha256=digest(pilot), release_sha256=digest(release),
                                    same_bytes=data == pilot.read_bytes() == release.read_bytes(),
                                    normalized_diff=list(difflib.unified_diff(normalized.splitlines(), release_text.splitlines(),
                                                                            fromfile='pinned', tofile='current-reference'))))
assert matches
feature = next(m for m in matches if m['entry'].endswith('FeatureReferencingPerformances.kerml'))
assert not any('beforeTimeSlice' in line or 'afterSnapshot' in line for line in feature['normalized_diff'])
packet = dict(format='agentique-semantic-erratum-authority/1', issue='KERML11-140',
              issue_status='open', issue_updated='2025-11-11T23:25:00Z',
              normative_baseline='KerML 1.0 formal/2026-03-01 clause 8.2.3.5.1',
              preliminary_status='KerML 1.1 Beta 2; corroboration only, unchanged failing rule',
              disposition='User-authorized Agentique operational interpretation; not adopted OMG erratum',
              acquisition=json.loads((OUT/'acquisition.json').read_bytes()), source_byte_comparisons=matches,
              independent_exact_mappings=exact,
              local_evidence=[dict(path=str(p.relative_to(ROOT)).replace('\\', '/'), sha256=digest(p)) for p in
                              [ROOT/'KerML.pdf', OUT/'published-witness-imports.rs', OUT/'pdf-clauses.json',
                               OUT/'reference-xmi-targets.json', OUT/'redefinition-release-notes.json']],
              inspected_code_path=['KerMLScopeProvider.getScope -> scope_owningNamespace -> scope_Namespace -> scopeFor',
                                   'KerMLScope.resolve -> gen -> resolveIfUnvisited (no general lexical parents)',
                                   'KerMLScope.getSingleElement -> parent.getSingleElement only without prefix shadowing',
                                   'KerMLScope.gen traverses all generalizations and suppresses redefined inherited candidates',
                                   'KerMLScope.owned enforces inherited visibility and skips the redefining owningType'],
              deliberate_agentique_safety='Preserve all incomparable candidates; do not port findFirst winner selection')
(OUT/'authority-packet.json').write_text(json.dumps(packet, indent=2)+'\n')
print(json.dumps(dict(exact_mappings=exact, source_byte_comparisons=matches), indent=2))
