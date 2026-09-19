"""Offline integrity checks for preserved authority and synthetic evidence."""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


acquisitions = json.loads((HERE/'acquisition.json').read_bytes())
assert len({r['path'] for r in acquisitions}) == len(acquisitions)
for row in acquisitions:
    path = HERE/row['path']
    assert sha(path) == row['sha256'], row['path']
    assert path.stat().st_size == row['bytes'], row['path']
manifest = json.loads((ROOT/'standards/kerml-1.0-operational-errata-v2.json').read_bytes())
entry, = manifest['entries']
assert sha(ROOT/entry['algorithm']) == entry['algorithm_sha256']
assert sha(ROOT/entry['evidence_packet']) == entry['evidence_packet_sha256']
assert sha(ROOT/manifest['extends']['manifest']) == manifest['extends']['sha256']
for row in json.loads((HERE/'authority-packet.json').read_bytes())['acquisition']:
    assert sha(HERE/row['path']) == row['sha256'], row['path']
conflict = json.loads((HERE/'separate-authority/authority-packet.json').read_bytes())
for row in conflict['artifacts']:
    assert sha(ROOT/row['path']) == row['sha256'], row['path']
assert len(conflict['witnesses']) == 3
assert all(r['candidate_count'] == 2 and not r['distinguishable'] for r in conflict['witnesses'])

matrix_log = HERE/'matrix-8/matrix.txt'
matrix = [json.loads(line.removeprefix('MATRIX ')) for line in matrix_log.read_text().splitlines()
          if line.startswith('MATRIX ')]
assert len(matrix) == 42
for row in matrix:
    assert row['candidates'] == row['expected_candidates']
    assert row['completeness'] == 'Complete'
    assert row['ambiguous'] == (len(row['expected_candidates']) > 1)
    assert row['positive_dependencies'] and row['search_dependencies']
    assert row['profile'] == manifest['profile_id']
    assert row['rule'] == 'AGQ-KERML10-002 / agentique-kerml10-redefinition-target/1'
    assert row['lexical_expected'] == any('LexicalContaining' in dep for dep in row['search_dependencies'])
for case in {r['case'] for r in matrix}:
    forward, reverse = sorted((r for r in matrix if r['case'] == case), key=lambda r: r['reverse'])
    assert not forward['reverse'] and reverse['reverse']
    assert forward['candidates'] == reverse['candidates']
record = dict(format='agentique-kerml11-140-adversarial-matrix/1',
              source_log=dict(path=matrix_log.relative_to(ROOT).as_posix(), sha256=sha(matrix_log)),
              cases=matrix)
path = HERE/'synthetic-matrix.json'
if path.exists():
    assert json.loads(path.read_bytes()) == record
else:
    with path.open('x', encoding='utf-8') as stream:
        json.dump(record, stream, indent=2)
        stream.write('\n')
print(json.dumps(dict(acquired_artifacts=len(acquisitions), matrix_cases=len(matrix),
                     independent_source_conflicts=len(conflict['witnesses']), hash_mismatches=0)))
