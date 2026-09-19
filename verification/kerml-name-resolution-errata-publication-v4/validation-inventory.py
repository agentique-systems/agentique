"""Inventory named formal class constraints and actual quality-audit executions.

An implementation string or an executed query is not automatically full coverage
of a formal rule. Unevaluated constraints remain explicit publication obligations.
"""
import collections
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
formal = ROOT/'standards/normative/kerml-1.0/KerML.xmi'
source = ROOT/'crates/kerml-semantics/src/validation.rs'
tree = ET.parse(formal).getroot()
parents = {c: p for p in tree.iter() for c in p}
XID = '{http://www.omg.org/spec/XMI/20161101}id'
XTYPE = '{http://www.omg.org/spec/XMI/20161101}type'
implemented = set(re.findall(r'"((?:validate|check|derive)[A-Za-z]+)"', source.read_text()))
quality = json.loads((HERE/'quality-4/library-quality.json').read_bytes())
executed = collections.Counter()
for document in quality['documents']:
    executed.update(document['full_KerML_constraint_validation']['evaluated_constraints'])
rules = []
for element in tree.iter():
    if element.tag != 'ownedRule' or not element.get('name'):
        continue
    name = element.get('name')
    owner = parents[element]
    assert owner.get(XTYPE) in ['uml:Class', 'uml:AssociationClass'], (name, owner.attrib)
    body = next(c.get('body') for c in element if c.tag == 'specification')
    rules.append(dict(name=name, external_id=element.get(XID), owning_metaclass=owner.get('name'),
                      kind=('implied structural relationship check' if name.startswith('check') else
                            'derived structural property constraint' if name.startswith('derive') else
                            'structural validation constraint'),
                      publication_scope='Required for applicable canonical library elements; no executable-evaluation exemption inferred',
                      implementation=('validation.rs check present' if name in implemented else
                                      'Complete runtime validation coverage not established by this inventory'),
                      actual_check_executions=executed[name],
                      obligation=('Retain adversarial validation and full corpus findings' if name in implemented else
                                  'Establish applicability and complete required runtime validation before accepted publication'),
                      formal_ocl=body))
assert implemented <= {r['name'] for r in rules}
assert set(executed) <= implemented
report = dict(format='agentique-kerml-structural-validation-inventory/1',
              authority=dict(path=formal.relative_to(ROOT).as_posix(),sha256=hashlib.sha256(formal.read_bytes()).hexdigest()),
              implementation=dict(path=source.relative_to(ROOT).as_posix(),sha256=hashlib.sha256(source.read_bytes()).hexdigest()),
              quality_evidence='quality-4/library-quality.json',
              named_formal_class_constraints=len(rules), explicit_validation_checks=len(implemented),
              complete=False, execution_evaluation='separate, unevaluated; not a waiver for any listed structural constraint',
              strict_raw_metamodel_authoring_conformance='Separate strict-1 results; not counted as library validation success',
              constraints=sorted(rules,key=lambda r:r['name']))
path = HERE/'validation-inventory.json'
if path.exists():
    assert json.loads(path.read_bytes()) == report
else:
    with path.open('x',encoding='utf-8') as stream:
        json.dump(report,stream,indent=2)
        stream.write('\n')
print(json.dumps({k:v for k,v in report.items() if k not in ['constraints','authority','implementation']}))
