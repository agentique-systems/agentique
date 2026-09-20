"""Losslessly retain the fresh canonical export with reproducible compression."""
import gzip
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()
source=OUT/'declarations-v5.json'
destination=source.with_suffix('.json.gz')
if '--check' not in sys.argv:
    raw=source.read_bytes()
    encoded=gzip.compress(raw,compresslevel=9,mtime=0)
    assert gzip.decompress(encoded)==raw
    assert not destination.exists()
    destination.write_bytes(encoded)
else:
    encoded=destination.read_bytes()
    raw=gzip.decompress(encoded)
previous=gzip.decompress((ROOT/'verification/kerml-semantic-closure-v7/declarations-v5.json.gz').read_bytes())
report=dict(format='agentique-v8-lossless-export/1',path=destination.name,
    archive_sha256=sha(encoded),archive_bytes=len(encoded),sha256=sha(raw),bytes=len(raw),
    fresh_command='declarations-v5/result.json',byte_equal_to_historical_v7=raw==previous)
text=json.dumps(report,indent=2)+'\n'
path=OUT/'archived-exports.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8')==text
else:
    path.write_text(text,encoding='utf8',newline='\n')
    source.unlink()  # Only this verified, losslessly archived generated export.
print(json.dumps(report))
