"""Losslessly compress the generated canonical dump, never pinned input bytes."""
import gzip
import hashlib
import json
from pathlib import Path

OUT=Path(__file__).resolve().parent
source=OUT/'declarations-v5.json'
archive=source.with_suffix('.json.gz')
raw=source.read_bytes() if source.exists() else gzip.decompress(archive.read_bytes())
compressed=archive.read_bytes() if archive.exists() else gzip.compress(raw,compresslevel=9,mtime=0)
if not archive.exists(): archive.write_bytes(compressed)
assert gzip.decompress(archive.read_bytes())==raw
report=dict(source=source.name,archive=archive.name,bytes=len(raw),
    sha256=hashlib.sha256(raw).hexdigest(),archive_bytes=len(compressed),
    archive_sha256=hashlib.sha256(compressed).hexdigest(),lossless=True)
(OUT/'archived-exports.json').write_text(json.dumps([report],indent=2)+'\n',encoding='utf8',newline='\n')
if source.exists(): source.unlink()
print(json.dumps(report))
