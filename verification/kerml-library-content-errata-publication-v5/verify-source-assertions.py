"""Verify all original source assertions survive the canonical correction."""
import json
from pathlib import Path

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
before=json.loads((OUT/'published-assertions/assertions.json').read_bytes())
after=json.loads((OUT/'operational-assertions/assertions.json').read_bytes())
review=json.loads((ROOT/'standards/kerml-1.0-operational-library-errata-v3.json').read_bytes())
assert before['profile']=='omg-kerml-1.0-published/1'
assert after['profile']==review['profile_id']
assert not before['superseded_references']
key=lambda r:(r['relationship'],r['property'])
original={key(r):r for r in before['active_references']}
active={key(r):r for r in after['active_references']}
superseded={key(r):r for r in after['superseded_references']}
assert len(original)==len(before['active_references'])
assert len(active)==len(after['active_references'])
assert not active.keys() & superseded.keys()
assert original==active|superseded
expected={review['selectors'][op['element']['pinned']]['id'] for op in review['entries'][0]['operations']
          if op['operation']=='set' and 'pinned' in op['element'] and op['element']['pinned'].endswith('/typing')}
assert {r['relationship'] for r in superseded.values()}==expected
assert {tuple(r['name']) for r in superseded.values()}=={('Surface',),('Curve',),('Point',)}
report=dict(original_assertions=len(original),active_assertions=len(active),superseded_assertions=len(superseded),
    unreviewed_removals=0,unreviewed_rewrites=0,all_original_assertions_preserved=True,
    superseded=list(superseded.values()),scope='Source assertions remain distinct from operational fixed relationship facts and resolution completeness.')
(OUT/'source-assertion-diff.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='superseded'}))
