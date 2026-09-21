"""Verify starting standards/evidence bytes and new acquisitions offline."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
BASE='59abd9801d59de025bf7e346464a32848047fd80'
sha=lambda b:hashlib.sha256(b).hexdigest()
original=json.loads((OUT/'preserved.json').read_text())
for r in original:
    assert sha((ROOT/r['path']).read_bytes())==r['sha256'], r['path']
acquisitions=json.loads((OUT/'acquisition.json').read_text())
for r in acquisitions:
    raw=(OUT/r['path']).read_bytes()
    assert len(raw)==r['bytes'] and sha(raw)==r['sha256'], r['path']
historical=ROOT/'verification/kerml-library-content-errata-publication-v5'
discrepancies=[]
for r in json.loads((historical/'acquisition.json').read_text()):
    path=historical/r['path']
    raw=path.read_bytes()
    if sha(raw)==r['sha256']: continue
    relative=path.relative_to(ROOT).as_posix()
    committed=subprocess.check_output(['git','-c','core.longpaths=true','cat-file','blob',BASE+':'+relative],cwd=ROOT)
    assert committed==raw,relative
    discrepancies.append(dict(path=relative,recorded_sha256=r['sha256'],base_and_current_sha256=sha(raw)))
report=dict(format='agentique-v10-preservation/1',base_commit=BASE,
    original_files=len(original),unchanged=True,verified_new_acquisitions=len(acquisitions),
    historical_manifests_and_evidence_modified=False,inherited_acquisition_discrepancies=discrepancies,
    discrepancies_already_present_on_clean_main=True)
text=json.dumps(report,indent=2)+'\n'
path=OUT/'preservation.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8')==text
else: path.write_text(text,encoding='utf8',newline='\n')
print(json.dumps({k:v for k,v in report.items() if k!='inherited_acquisition_discrepancies'}))
print('Inherited acquisition-index discrepancies:',len(discrepancies))
