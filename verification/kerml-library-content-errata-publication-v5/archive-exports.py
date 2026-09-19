"""Losslessly archive only this milestone's completed generated model exports."""
import gzip
import hashlib
import io
import json
from pathlib import Path

OUT=Path(__file__).resolve().parent
records=[]
index=OUT/'archived-exports.json'
if index.exists():records=json.loads(index.read_bytes())
for logical in ['declarations-3/model.json','operational-declarations/model.json',
                'published-declarations/model.json','operational-obligations-2/obligations.json']:
    path=(OUT/logical).resolve()
    folder=path.parent
    assert path.is_relative_to(OUT.resolve())
    assert json.loads((folder/'result.json').read_bytes())['exit_code']==0
    if not path.exists():
        assert any(r['logical_path']==logical for r in records)
        continue
    data=path.read_bytes()
    buffer=io.BytesIO()
    with gzip.GzipFile(fileobj=buffer,mode='wb',filename='',mtime=0) as stream:stream.write(data)
    compressed=buffer.getvalue()
    assert gzip.decompress(compressed)==data
    archive=path.with_suffix('.json.gz')
    with archive.open('xb') as stream:stream.write(compressed)
    assert gzip.decompress(archive.read_bytes())==data
    records.append(dict(logical_path=logical,archive=logical+'.gz',
        sha256=hashlib.sha256(data).hexdigest(),bytes=len(data),archive_sha256=hashlib.sha256(compressed).hexdigest(),archive_bytes=len(compressed)))
    index.write_text(json.dumps(records,indent=2)+'\n')
    # Exact bytes have been independently read back. Only the redundant,
    # newly generated uncompressed representation is removed.
    path.unlink()
print(json.dumps(records))
