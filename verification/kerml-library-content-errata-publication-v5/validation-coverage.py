"""Conservative per-constraint publication inventory; never infers green coverage.

No structural expression check receives an executable-evaluation waiver. Missing
checks remain work after the independently established KLCV5-F-001 authority stop.
"""
import collections
import hashlib
import json
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
formal=ROOT/'standards/normative/kerml-1.0/KerML.xmi'
source=ROOT/'crates/kerml-semantics/src/validation.rs'
quality_path=Path(sys.argv[1])
quality=json.loads(quality_path.read_text())
tree=ET.parse(formal).getroot()
parents={c:p for p in tree.iter() for c in p}
XID='{http://www.omg.org/spec/XMI/20161101}id'
XTYPE='{http://www.omg.org/spec/XMI/20161101}type'
implemented=set(re.findall(r'"((?:validate|check|derive)[A-Za-z]+)"',source.read_text()))
executed=collections.Counter()
for d in quality['documents']:executed.update(d['full_KerML_constraint_validation']['evaluated_constraints'])
namespace_owners={'Namespace','Membership','OwningMembership','NamespaceImport','MembershipImport','Import'}
rules=[]
for e in tree.iter():
    if e.tag!='ownedRule' or not e.get('name'):continue
    name=e.get('name');owner=parents[e].get('name')
    body=next(c.get('body') for c in e if c.tag=='specification')
    category=2 if owner in namespace_owners or name in ['deriveElementName','deriveElementShortName','deriveElementQualifiedName'] else 3 if name.startswith(('derive','check')) else 1
    # Model-level evaluability is structural eligibility, not execution itself.
    # It stays category 3. Actual executable bodies are counted separately.
    status='explicit_check_executed' if name in implemented and executed[name] else 'coverage_not_established'
    reason=('Actual check executions recorded; this alone does not prove all premises complete.' if status=='explicit_check_executed' else
            'Required applicability and implementation remain open after KLCV5-F-001; not waived as execution. No accepted publication exists.')
    rules.append(dict(name=name,external_id=e.get(XID),owning_metaclass=owner,category=category,
        formal_ocl=body,implementation_status=status,actual_executions=executed[name],reason=reason,
        corpus_applicability='Conservatively retained as required until exact-corpus applicability and all derived facts are established.'))
assert implemented<={r['name'] for r in rules}
assert set(executed)<=implemented
report=dict(format='agentique-kerml-formal-validation-coverage/2',complete=False,authority_stop='KLCV5-F-001',
    authority_sha256=hashlib.sha256(formal.read_bytes()).hexdigest(),implementation_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
    quality_evidence=str(quality_path),named_constraints=len(rules),explicit_checks=len(implemented),
    categories={'1':'structural publication','2':'namespace/resolution','3':'derived/implied structure','4':'evaluation/runtime','5':'metamodel-authoring/source','6':'irrelevant to exact corpus'},
    category_counts=dict(collections.Counter(r['category'] for r in rules)),
    deferred_executable_elements=sum(d['unevaluated_expression_count'] for d in quality['documents']),
    raw_metamodel_conformance='Separate authoring audit; does not substitute for these model-instance constraints.',
    constraints=sorted(rules,key=lambda r:r['name']))
output=OUT/sys.argv[2]
with output.open('x',encoding='utf8') as f:json.dump(report,f,indent=2);f.write('\n')
print(json.dumps({k:report[k] for k in ['complete','named_constraints','explicit_checks','category_counts','deferred_executable_elements']}))
