"""Independent canonical end-membership and expression population inventories."""
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
raw=gzip.decompress((OUT/'declarations-v5.json.gz').read_bytes())
model=json.loads(raw)
records=model['records']
prior=json.loads((ROOT/'verification/kerml-semantic-closure-v6/individual-findings.json').read_text())
previous={r['subject'] for r in prior['findings'] if r['code']=='validateEndFeatureMembershipIsEnd'}
rows=[]
for identity,record in records.items():
    if record['metaclass']!='EndFeatureMembership': continue
    members=[r for s in record['slots'].values() if s['name']=='ownedRelatedElement' for r in s['references']]
    assert len(members)==1
    feature=records[members[0]]
    end,=[s for s in feature['slots'].values() if s['name']=='isEnd']
    rows.append(dict(membership=identity,feature=members[0],isEnd=end['value']=='Scalar(Boolean(true))',
        flag_origin=end['origin'],feature_metaclass=feature['metaclass'],source=record['source'],
        was_v6_finding=identity in previous))
assert previous <= {r['membership'] for r in rows}
assert all(r['isEnd'] for r in rows)
report=dict(format='agentique-v7-end-lowering-audit/1',profile=model['profile'],
    construction_sha256=hashlib.sha256(raw).hexdigest(),memberships_checked=len(rows),
    findings=sum(not r['isEnd'] for r in rows),historical_findings_rechecked=len(previous),
    documents=dict(sorted(Counter(r['source']['document'] for r in rows).items())),
    scope='Every EndFeatureMembership in the exact three-library construction; newly owned Features only. This is not strict semantic publication.',
    rows=rows)
encoded=json.dumps(report,indent=2)+'\n'
path=OUT/'end-membership-lowering.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8')==encoded
else: path.write_text(encoded,encoding='utf8',newline='\n')
print('EndFeatureMembership audit:',report['memberships_checked'],'checked,',report['findings'],'findings')
