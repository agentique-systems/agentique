"""Verify all original pinned bytes and historical verification artifacts."""
import hashlib
import gzip
import json
from pathlib import Path

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
preserved=json.loads((OUT/'preserved.json').read_text())
for record in preserved:
    path=ROOT/record['path']
    assert path.is_file(),record['path']
    assert hashlib.sha256(path.read_bytes()).hexdigest()==record['sha256'],record['path']
acquisition=json.loads((OUT/'acquisition.json').read_text())
for record in acquisition:
    data=(OUT/record['path']).read_bytes()
    assert hashlib.sha256(data).hexdigest()==record['sha256'],record['path']
    assert len(data)==record['bytes']
v2=json.loads((ROOT/'standards/kerml-1.0-operational-errata-v2.json').read_text())
frozen=(OUT/'operational-profile-v2-frozen.md').read_bytes()
assert hashlib.sha256(frozen).hexdigest()==v2['entries'][0]['algorithm_sha256']
assert (ROOT/v2['entries'][0]['algorithm']).read_bytes().startswith(frozen)
archives=json.loads((OUT/'archived-exports.json').read_bytes())
for record in archives:
    data=(OUT/record['archive']).read_bytes()
    assert hashlib.sha256(data).hexdigest()==record['archive_sha256']
    raw=gzip.decompress(data)
    assert len(raw)==record['bytes']
    assert hashlib.sha256(raw).hexdigest()==record['sha256']
report=dict(preserved_original_artifacts=len(preserved),unchanged=True,acquired_artifacts_verified=len(acquisition),
    v2_algorithm_bytes_unchanged=True,v3_definition_is_append_only=True,
    original_kpar_bytes_unchanged=True,historical_verification_bytes_unchanged=True,lossless_generated_exports_verified=len(archives))
print(json.dumps(report))
