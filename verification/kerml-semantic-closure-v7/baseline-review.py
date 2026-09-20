"""Verify the pre-change run captured the fetched clean main source bytes."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
base=json.loads((OUT/'preflight.json').read_text())['base_commit']
paths=['crates/kerml/src/profiles.rs','crates/kerml-semantics/src/implicit.rs',
       'crates/kerml-text/src/library/construction.rs','crates/kerml-text/examples/library_quality.rs']
initial={r['path']:r['sha256'] for r in json.loads((OUT/'baseline-v4-command/input-hashes.json').read_text())}
rows=[]
for path in paths:
    data=subprocess.check_output(['git','cat-file','blob',base+':'+path],cwd=ROOT)
    digest=hashlib.sha256(data).hexdigest()
    assert digest==initial[path],path
    rows.append(dict(path=path,base_and_captured_sha256=digest))
report=dict(base_commit=base,source_inputs_match_clean_main=True,rows=rows)
encoded=json.dumps(report,indent=2)+'\n'
path=OUT/'baseline-input-review.json'
if '--check' in sys.argv:assert path.read_text(encoding='utf8')==encoded
else:path.write_text(encoded,encoding='utf8',newline='\n')
print('Baseline source hashes match clean main:',len(rows))
