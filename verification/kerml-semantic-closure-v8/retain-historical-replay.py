"""Retain legacy verifier outputs here and restore verified starting bytes."""
import hashlib
import json
from pathlib import Path
import subprocess

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
BASE='ed05cc1e30f955ed73af3689e9b98b20f192d63f'
sha=lambda b:hashlib.sha256(b).hexdigest()
records=json.loads((OUT/'preserved.json').read_text())
allowed={f'verification/kerml-library-content-errata-publication-v5/{name}' for name in
    ['authority-stop.json','four-model-witnesses.json','operational-patch-diff.json','source-assertion-diff.json']}
restored=[]
for record in records:
    path=ROOT/record['path']
    current=path.read_bytes()
    if sha(current)==record['sha256']: continue
    assert record['path'] in allowed,record['path']
    original=subprocess.check_output(['git','-c','core.longpaths=true','cat-file','blob',BASE+':'+record['path']],cwd=ROOT)
    assert sha(original)==record['sha256'],record['path']
    destination=OUT/'historical-replay-outputs'/path.name
    assert not destination.exists()
    destination.parent.mkdir(exist_ok=True)
    destination.write_bytes(current)
    path.write_bytes(original)
    assert sha(path.read_bytes())==record['sha256']
    restored.append(dict(path=record['path'],preserved_sha256=record['sha256'],
        replay_output=destination.relative_to(OUT).as_posix(),replay_sha256=sha(current),
        json_value_equal=json.loads(current)==json.loads(original)))
report=OUT/'historical-replay-restoration.json'
assert not report.exists()
report.write_text(json.dumps(dict(restored=restored,historical_bytes_unchanged_after_replay=True),indent=2)+'\n',encoding='utf8',newline='\n')
print('Retained and restored',len(restored),'legacy-generated reports; all starting hashes verified.')
