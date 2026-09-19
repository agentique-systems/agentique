"""Verify this turn's untouched input bytes, without rewriting historical evidence."""
import hashlib
import json
from pathlib import Path
import subprocess
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()
initial=json.loads((OUT/'preserved.json').read_text())
for row in initial:
    assert sha((ROOT/row['path']).read_bytes())==row['sha256'],row['path']
acquisition=json.loads((OUT/'acquisition.json').read_text())
for row in acquisition:
    data=(OUT/row['path']).read_bytes()
    assert sha(data)==row['sha256'] and len(data)==row['bytes'],row['path']
frozen=ROOT/'verification/kerml-library-content-errata-publication-v5/operational-profile-v2-frozen.md'
assert (ROOT/'docs/operational-kerml-profile.md').read_bytes().startswith(frozen.read_bytes())
legacy=ROOT/'verification/kerml-library-content-errata-publication-v5'
baseline_discrepancies=[]
for row in json.loads((legacy/'acquisition.json').read_text()):
    p=legacy/row['path']
    actual=sha(p.read_bytes())
    if actual!=row['sha256']:
        relative=p.relative_to(ROOT).as_posix()
        committed=subprocess.check_output(['git','show','HEAD:'+relative],cwd=ROOT)
        assert committed==p.read_bytes(),relative
        baseline_discrepancies.append(dict(path=relative,manifest_sha256=row['sha256'],
                                          committed_and_current_sha256=actual,
                                          disposition='Pre-existing acquisition-index mismatch; original evidence retained unchanged'))
report=dict(preserved_original_files=len(initial),original_bytes_unchanged=True,
            acquired_v6_artifacts_verified=len(acquisition),frozen_profile_prefix_unchanged=True,
            preexisting_v5_acquisition_index_discrepancies=baseline_discrepancies)
(OUT/'preservation.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
