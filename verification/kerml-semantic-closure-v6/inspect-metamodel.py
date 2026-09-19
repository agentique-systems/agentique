"""Independent named constraint/operation/property inventory of the pinned XMI."""
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
file = ROOT/'standards/artifacts/KerML.xmi'
if not file.exists():
    file = next((ROOT/'standards').rglob('KerML.xmi'))
root = ET.parse(file).getroot()
parents = {c:e for e in root.iter() for c in e}
rows = []
for e in root.iter():
    if e.tag not in ['ownedRule','ownedOperation','ownedAttribute']:
        continue
    owner = parents[e]
    rows.append(dict(kind=e.tag,owner=owner.get('name'),name=e.get('name'),attributes=e.attrib,
                     bodies=[d.attrib for d in e.iter() if d.get('body')],xml=ET.tostring(e,encoding='unicode')))
(OUT/'metamodel-inventory.json').write_text(json.dumps(dict(file=str(file.relative_to(ROOT)),sha256=hashlib.sha256(file.read_bytes()).hexdigest(),members=rows),indent=2)+'\n')
selected = {'namingFeature','owningType','ownedSpecialization','ownedFeatureMembership','featureMembership',
            'parameter','result','endFeature','inheritedMemberships','ownedRedefinition',
            'checkFeatureParameterRedefinition','checkFeatureResultRedefinition','checkFeatureEndRedefinition',
            'validateRedefinitionEndConformance','deriveFeatureOwningType'}
for r in rows:
    if r['name'] in selected:
        print(r['owner'],r['name'],r['attributes'])
        print(json.dumps(r['bodies'],indent=2))
