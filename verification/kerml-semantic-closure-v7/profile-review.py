"""Pin profile lineage, reviewed authority and unresolved adjacent issue scope."""
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()
manifests=[
    ('v1','kerml-1.0-operational-errata.json',['KERML11-81']),
    ('v2','kerml-1.0-operational-errata-v2.json',['KERML11-140']),
    ('v3','kerml-1.0-operational-library-errata-v3.json',['KERML11-76']),
    ('v4','kerml-1.0-operational-validation-errata-v4.json',['KERML11-68']),
    ('v5','kerml-1.0-operational-formal-target-errata-v5.json',['KERML11-205','KERML11-206','KERML11-207'])]
rows=[dict(profile='omg-kerml-1.0-published/1',new_corrections=[],effective_corrections=[],formal_targets='published')]
effective=[]
for label,name,issues in manifests:
    effective.extend(issues)
    path=ROOT/'standards'/name
    rows.append(dict(profile='agentique-kerml-1.0-operational/'+label[1:],manifest=path.relative_to(ROOT).as_posix(),
        sha256=sha(path.read_bytes()),new_corrections=issues,effective_corrections=list(effective),
        formal_targets='reviewed six exact targets' if label=='v5' else 'published'))
matrix=json.loads((OUT/'authority-matrix.json').read_text())
manifest=json.loads((ROOT/'standards'/manifests[-1][1]).read_text())
assert len(manifest['entries'])==len(matrix['constraints'])==6
for row in matrix['constraints']:
    entry,=[r for r in manifest['entries'] if r['rule_id']==row['rule_id']]
    for key in ['rule','published_body','prose_requirement','published_target','operational_target','issue','issue_status']:
        assert entry[key]==row[key],(row['rule_id'],key)
profile=dict(format='agentique-v7-six-profile-review/1',profiles=rows,
    evidence='historical-profile-matrix-final/result.json',default_operational='agentique-kerml-1.0-operational/2',
    v5_inherited_end_rule_evidence='v5-inherited-end-rule/result.json',end_flag_owner_profile_cases=264,
    v5_authority_matrix_sha256=sha((OUT/'authority-matrix.json').read_bytes()),
    historical_manifests_unchanged=True,accepted_publication=False)
adjacent=dict(format='agentique-v7-adjacent-authority-review/1',corrections_applied=[],issues=[
    dict(issue='KERML11-145',sha256=sha((OUT/'issues/KERML11-145.html').read_bytes()),status='open',
        disposition='Exact pinned checkFunctionResultBindingConnector conflict independently reproduced as KLCV7-F-001. Other associated constraints are metadata, not independently claimed additional conflicts.',authorized=False),
    dict(issue='KERML11-182',sha256=sha((OUT/'issues/KERML11-182.html').read_bytes()),status='open',
        disposition='Future authority metadata. No independently reproduced corpus conflict or operational correction asserted in this stop packet.',authorized=False),
    dict(issue='KERML11-210',sha256=sha((OUT/'issues/KERML11-210.html').read_bytes()),status='open',
        disposition='Future authority metadata. No independently reproduced corpus conflict or operational correction asserted in this stop packet.',authorized=False)])
for name,value in [('historical-profile-matrix.json',profile),('adjacent-issue-review.json',adjacent)]:
    encoded=json.dumps(value,indent=2)+'\n'
    path=OUT/name
    if '--check' in sys.argv: assert path.read_text(encoding='utf8')==encoded
    else:path.write_text(encoded,encoding='utf8',newline='\n')
print('Six profile identities and six authorized target rows match; adjacent issues remain unauthorized.')
