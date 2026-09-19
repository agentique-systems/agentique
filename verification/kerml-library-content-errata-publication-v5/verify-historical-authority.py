"""Recheck historical authority with explicit resolution of the frozen v2 text.

The old verifier and its evidence are untouched. The living profile document now
has a v3 appendix; the original v2 bytes remain hash-identical and its prefix.
"""
import hashlib
import json
from pathlib import Path

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
V4=ROOT/'verification/kerml-name-resolution-errata-publication-v4'
sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
manifest=json.loads((ROOT/'standards/kerml-1.0-operational-errata-v2.json').read_bytes())
entry,=manifest['entries']
frozen=OUT/'operational-profile-v2-frozen.md'
assert sha(frozen)==entry['algorithm_sha256']
assert (ROOT/entry['algorithm']).read_bytes().startswith(frozen.read_bytes())
assert sha(ROOT/entry['evidence_packet'])==entry['evidence_packet_sha256']
assert sha(ROOT/manifest['extends']['manifest'])==manifest['extends']['sha256']
acquisitions=json.loads((V4/'acquisition.json').read_bytes())
for row in acquisitions:
    assert sha(V4/row['path'])==row['sha256'],row['path']
    assert (V4/row['path']).stat().st_size==row['bytes']
for row in json.loads((V4/'authority-packet.json').read_bytes())['acquisition']:
    assert sha(V4/row['path'])==row['sha256'],row['path']
conflict=json.loads((V4/'separate-authority/authority-packet.json').read_bytes())
for row in conflict['artifacts']:assert sha(ROOT/row['path'])==row['sha256'],row['path']
assert len(conflict['witnesses'])==3
assert all(r['candidate_count']==2 and not r['distinguishable'] for r in conflict['witnesses'])
matrix=[json.loads(line.removeprefix('MATRIX ')) for line in (V4/'matrix-8/matrix.txt').read_text().splitlines() if line.startswith('MATRIX ')]
assert len(matrix)==42
for row in matrix:
    assert row['candidates']==row['expected_candidates']
    assert row['completeness']=='Complete'
    assert row['profile']==manifest['profile_id']
    assert row['positive_dependencies'] and row['search_dependencies']
print(json.dumps(dict(original_v2_algorithm_sha256=sha(frozen),historical_acquisitions=len(acquisitions),historical_matrix_rows=len(matrix),historical_source_conflicts=len(conflict['witnesses']),hash_mismatches=0,
    artifact_resolution='Exact frozen v2 text plus asserted unchanged prefix of extended living document; no v4 files rewritten.')))
