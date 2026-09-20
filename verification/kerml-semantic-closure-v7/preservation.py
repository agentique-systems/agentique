"""Preserve this turn's original bytes; report inherited acquisition defects."""
import hashlib
import gzip
import json
from pathlib import Path
import subprocess
import sys

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
sha = lambda b: hashlib.sha256(b).hexdigest()
original = json.loads((OUT/'preserved.json').read_text())
for record in original:
    assert sha((ROOT/record['path']).read_bytes()) == record['sha256'], record['path']
acquired = json.loads((OUT/'acquisition.json').read_text())
for record in acquired:
    data = (OUT/record['path']).read_bytes()
    assert sha(data) == record['sha256'] and len(data) == record['bytes'], record['path']
inherited = ROOT/'verification/kerml-library-content-errata-publication-v5'
discrepancies = []
base = json.loads((OUT/'preflight.json').read_text())['base_commit']
for record in json.loads((inherited/'acquisition.json').read_text()):
    path = inherited/record['path']
    data = path.read_bytes()
    if sha(data) != record['sha256']:
        relative = path.relative_to(ROOT).as_posix()
        committed = subprocess.check_output(['git','-c','core.longpaths=true','cat-file','blob',base+':'+relative],cwd=ROOT)
        assert data == committed, relative
        discrepancies.append(dict(path=relative,recorded_sha256=record['sha256'],
            committed_and_worktree_sha256=sha(data),present_on_clean_main=True))
report = dict(format='agentique-v7-preservation/1',unchanged=True,
    original_files=len(original),verified_new_acquisitions=len(acquired),
    inherited_acquisition_discrepancies=discrepancies,
    historical_files_or_indices_rewritten=False)
archives=json.loads((OUT/'archived-exports.json').read_text())
for record in archives:
    compressed=(OUT/record['archive']).read_bytes()
    raw=gzip.decompress(compressed)
    assert sha(compressed)==record['archive_sha256'] and len(compressed)==record['archive_bytes']
    assert sha(raw)==record['sha256'] and len(raw)==record['bytes']
report['lossless_exports_verified']=len(archives)
encoded = json.dumps(report,indent=2)+'\n'
path = OUT/'preservation.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8') == encoded
else: path.write_text(encoded,encoding='utf8',newline='\n')
print(json.dumps({k:v for k,v in report.items() if k!='inherited_acquisition_discrepancies'}))
print('Pre-existing acquisition-index discrepancies:',len(discrepancies))
