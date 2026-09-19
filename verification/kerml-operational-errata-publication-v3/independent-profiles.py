"""Independent direct XML/UUID derivation plus compiled runtime profile diff."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
spec = importlib.util.spec_from_file_location('independent', OUT.parent/'language-core-completion-v3/independent_xmi.py')
ind = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ind)
mm = ind.old.read_source('kerml-1.0')
ind.verify_runtime([mm, ind.old.read_source('sysml-2.0')], ind.primitives())
manifest = json.loads((ROOT/'standards/kerml-1.0-operational-errata.json').read_bytes())
entry = manifest['entries'][0]
assert mm['source']['sha256'] == entry['artifact_sha256']
assert mm['source']['artifact_uri'] == entry['artifact_uri']
roots = [e for e in mm['ids'].values() if ind.old.kind(e) == 'Association'
         and any(c.get('name') == 'participantFeature' for c in e.findall('ownedEnd'))]
assert len(roots) == 2
xml_closure = {e.get(ind.old.XMI+'id') for r in roots for e in r.iter() if e.get(ind.old.XMI+'id')}
assert sorted(xml_closure) == entry['xml_owned_closure']
descriptors = [e for r in roots for e in r.iter() if ind.old.kind(e) in ('Association', 'Property')]
assert {e.get(ind.old.XMI+'id') for e in descriptors} == {d['external_id'] for d in entry['descriptors']}
for d in entry['descriptors']:
    e = mm['ids'][d['external_id']]
    kind = {'Association':'association','Property':'property'}[d['kind']]
    assert ind.old.identity(mm,e,kind)[1] == d['descriptor_id']
    assert mm['ranges'][d['external_id']] == d['byte_range']
for e in mm['tree'].iter():
    for k, value in e.attrib.items():
        if k == ind.old.XMI+'id':
            continue
        for target in value.split():
            if target in xml_closure:
                ancestor = e
                while ancestor is not None and ancestor.get(ind.old.XMI+'id') not in xml_closure:
                    ancestor = mm['parents'].get(ancestor)
                assert ancestor is not None, ('unexpected incoming dependency', e.tag, target)
command = ['cargo','run','--locked','--offline','-q','-p','agq-kerml','--example','profile_descriptors']
completed = subprocess.run(command, cwd=ROOT, capture_output=True, check=True)
runtime = json.loads(completed.stdout)
published, operational = runtime['published'], runtime['operational']
assert published['profile'] == manifest['published_profile_id']
assert operational['profile'] == manifest['profile_id']
for key in ('models','classes','enumerations','primitives','reviews'):
    assert published[key] == operational[key], key
for key, kind in [('associations','Association'),('properties','Property')]:
    removed = {d['descriptor_id'] for d in entry['descriptors'] if d['kind'] == kind}
    assert set(published[key])-set(operational[key]) == removed
    assert {k:v for k,v in published[key].items() if k not in removed} == operational[key]
removed_sources = {k for k,s in published['sources'].items() if s['external_id'] in xml_closure}
assert len(removed_sources) == len(descriptors)
assert {k:v for k,v in published['sources'].items() if k not in removed_sources} == operational['sources']
report = dict(format='agentique-independent-operational-diff/1', command=command, exit_code=completed.returncode,
              published_profile=published['profile'], operational_profile=operational['profile'],
              published_properties=len(published['properties']), operational_properties=len(operational['properties']),
              published_associations=len(published['associations']), operational_associations=len(operational['associations']),
              exact_removed_descriptor_count=len(descriptors), xml_owned_closure_count=len(xml_closure),
              non_errata_changes=0, external_incoming_references=0,
              manifest_sha256=hashlib.sha256((ROOT/'standards/kerml-1.0-operational-errata.json').read_bytes()).hexdigest())
print(json.dumps(report, indent=2))
if '--output' in sys.argv:
    with Path(sys.argv[sys.argv.index('--output')+1]).open('x') as stream:
        json.dump(report,stream,indent=2)
