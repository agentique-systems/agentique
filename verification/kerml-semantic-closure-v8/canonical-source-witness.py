"""Tie the fresh construction identities to the independently proven source case."""
import gzip
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent
raw=gzip.decompress((OUT/'declarations-v5.json.gz').read_bytes())
model=json.loads(raw)
records=model['records']
proof=json.loads((OUT/'feature-reference-authority-conflict.json').read_text())


def refs(identity,property):
    return [target for slot in records[identity]['slots'].values() if slot['name']==property for target in slot['references']]


parents={target:identity for identity in records for prop in ['ownedRelationship','ownedRelatedElement'] for target in refs(identity,prop)}
expression,=[identity for identity,r in records.items() if r['metaclass']=='FeatureReferenceExpression' and r.get('source')
    and r['source']['document']==proof['pinned_source']['entry'] and r['source']['range']==[645,650]]
source=records[expression]['source']
assert source['text']=='chain' and source['sha256']==proof['pinned_source']['sha256']
result_membership,=[r for r in refs(expression,'ownedRelationship') if records[r]['metaclass']=='ReturnParameterMembership']
result,=refs(result_membership,'ownedRelatedElement')
result_expression_membership=parents[expression]
assert records[result_expression_membership]['metaclass']=='ResultExpressionMembership'
function=parents[result_expression_membership]
assert records[function]['metaclass']=='Function'
assert parents[result]==result_membership and parents[result_membership]==expression
reference_membership,=[r for r in refs(expression,'ownedRelationship') if records[r]['metaclass']=='Membership']
assert any(s['name']=='isVariable' and s['value']=='Scalar(Boolean(false))' for s in records[result]['slots'].values())
rows={role:dict(id=identity,record=records[identity]) for role,identity in
    [('function',function),('result_expression_membership',result_expression_membership),('expression',expression),
     ('result_membership',result_membership),('raw_result',result),('reference_membership',reference_membership)]}
report=dict(format='agentique-v8-canonical-source-witness/1',construction_sha256=hashlib.sha256(raw).hexdigest(),
    profile=model['profile'],record_count=len(records),fresh_command='declarations-v5/result.json',
    source_range=[645,650],canonical_roles=rows,
    reference_xmi_roles={k:proof['reference'][k] for k in ['function','expression','raw_result','referent','reference_binding','result_binding']},
    identity_namespaces_are_distinct=True,
    scope='Fresh declaration ownership/result identity comparison. This export retains unresolved construction obligations and is not the complete semantic graph or an accepted publication.',
    complete_local_domain_proof='feature-reference-authority-conflict.json',
    production_contextual_result=None,production_v6_binding_graph=None,accepted=False)
text=json.dumps(report,indent=2)+'\n'
path=OUT/'canonical-source-witness.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8')==text
else: path.write_text(text,encoding='utf8',newline='\n')
print('Fresh canonical source/result ownership matches the exact pinned authority witness; reference IDs are compared separately; no complete production binding graph is claimed.')
