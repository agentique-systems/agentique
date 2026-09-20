"""Complete inventory, fail-closed coverage gate. No implicit acceptance by count."""
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import sys

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
sha = lambda b: hashlib.sha256(b).hexdigest()
prior = ROOT/'verification/kerml-semantic-closure-v6/validation-coverage.json'
inventory = json.loads(prior.read_text())
authority = json.loads((OUT/'authority-matrix.json').read_text())
errata = {r['rule']:r for r in authority['constraints']}
model_path = OUT/'declarations-v5.json'
raw = model_path.read_bytes() if model_path.exists() else gzip.decompress(model_path.with_suffix('.json.gz').read_bytes())
model = json.loads(raw)
metamodel_path = ROOT/'standards/generated/kerml-1.0/metamodel.json'
classes = json.loads(metamodel_path.read_text())['metamodel']['classifiers']
by_name = {c['entity']['name']:c for c in classes.values() if c['entity']['key']['kind']=='class'}

def supertypes(name):
    visited = set()
    pending = [by_name[name]]
    while pending:
        current = pending.pop()
        key = current['entity']['external_id'] if 'external_id' in current['entity'] else current['entity']['key']['external_id']
        if key in visited: continue
        visited.add(key)
        pending.extend(classes[g] for g in current['generalizations'])
    return {classes[k]['entity']['name'] for k in visited}

populations = Counter()
for record in model['records'].values():
    populations.update(supertypes(record['metaclass']))
quality_path = OUT/'quality-v5-final-2.json'
quality = json.loads(quality_path.read_text()) if quality_path.exists() else None
executions = Counter()
if quality:
    for document in quality['documents']:
        executions.update(document['full_KerML_constraint_validation']['evaluated_constraints'])
rows = []
for previous in inventory['constraints']:
    row = dict(previous)
    name = row['name']
    if name == 'checkFunctionResultBindingConnector':
        category,status,reason = 'F','NewAuthorityConflict','KLCV7-F-001; independent pinned graph reproduction, not inferred from Agentique incompleteness.'
    elif name in errata:
        category,status,reason = 'E','ReviewedOperationalErratum','Exact v5 rule-target correction; complete effective owner typing remains an explicit inference obligation where needed.'
        row['manifest'] = 'standards/kerml-1.0-operational-formal-target-errata-v5.json'
        row['issue'] = errata[name]['issue']
    elif previous['implementation_status'] == 'explicit_check':
        category,status,reason = 'A','ImplementedAndChecked','Existing explicit structural validator executed over the applicable corpus elements.'
    else:
        category,status,reason = 'B','NotYetImplemented','Full independent validation coverage remains required. Work stops for separately proven KLCV7-F-001, not this workload.'
    row.update(category=category,status=status,implementation_status=status,reason=reason,
        actual_executions=executions[name],owning_metaclass_population=populations[row['owning_metaclass']],
        corpus_applicability='Conservatively required. Even zero construction population is not claimed as final non-applicability before structural expansion closes.')
    rows.append(row)
assert len(rows)==258 and len({r['external_id'] for r in rows})==258
report = dict(format='agentique-v7-formal-coverage/1',complete=False,publication_gate_passed=False,
    authority_stop='KLCV7-F-001',named_constraints=258,explicit_checks=len(executions),
    quality_completed=quality is not None,
    inputs=dict(inventory_sha256=sha(prior.read_bytes()),model_sha256=sha(raw),
        metamodel_sha256=sha(metamodel_path.read_bytes()),quality_sha256=sha(quality_path.read_bytes()) if quality else None),
    category_counts=dict(sorted(Counter(r['category'] for r in rows).items())),
    execution_deferral='Expression/function value execution remains outside scope. No unimplemented structural graph rule is categorized DeferredExecution.',
    constraints=rows)
encoded=json.dumps(report,indent=2)+'\n'
path=OUT/'validation-coverage.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8')==encoded
else: path.write_text(encoded,encoding='utf8',newline='\n')
print('All 258 named constraints inventoried:',report['category_counts'])
print('Formal coverage publication gate: FAIL; KLCV7-F-001 and structural obligations remain explicit.')
if '--gate' in sys.argv: sys.exit(1)
