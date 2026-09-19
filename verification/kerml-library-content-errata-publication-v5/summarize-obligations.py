"""Classify every actual incomplete answer without replacing the full proof audit."""
from collections import Counter
import json
from pathlib import Path
from evidence_io import read_json

OUT = Path(__file__).resolve().parent
path = OUT / 'operational-obligations-2/obligations.json'
audit = read_json(path)
references = []
for index, row in enumerate(audit['references']):
    answer = row['result']
    codes = sorted({d['code'] for d in answer['diagnostics']})
    references.append(dict(
        audit_pointer=f'/references/{index}', document=row['document'],
        element=row['element'], metaclass=row['metaclass'], property=row['property'],
        name=row['name'], reference_range=row['reference_range'],
        expression_context=row['expression_context'], completeness=answer['completeness'],
        candidates=answer['candidates'], diagnostic_codes=codes,
        diagnostics=answer['diagnostics'],
        cause=('Implied positional redefinition naming requires established semantic ordering.'
               if codes == ['KQ_IMPLIED_NAMING_ORDER'] else 'Additional unresolved diagnostic family.'),
        waived=False))
assert all(row['diagnostic_codes'] == ['KQ_IMPLIED_NAMING_ORDER'] for row in references)
assert all(row['completeness'] == 'Incomplete' for row in references)
queries = [dict(audit_pointer=f'/semantic_queries/{index}',
                **{key: row[key] for key in ['document', 'element', 'metaclass', 'query']},
                completeness=row['result']['completeness'], diagnostics=row['result']['diagnostics'])
           for index, row in enumerate(audit['semantic_queries'])]
report = dict(format='agentique-library-obligation-causes/1',
    evidence='operational-obligations-2/obligations.json', profile=audit['baseline_profile'],
    rule_version=audit['rule_version'], library_set=audit['library_set'],
    incomplete_reference_count=len(references),
    by_diagnostic_family=dict(Counter('|'.join(row['diagnostic_codes']) for row in references)),
    by_document=dict(sorted(Counter(row['document'] for row in references).items())),
    by_expression_context=dict(Counter(str(row['expression_context']) for row in references)),
    references=references, incomplete_query_count=len(queries), queries=queries,
    structural_obligation_count=len(audit['structural_obligations']),
    ordinary_kernel_storage_attempt=audit['strict_publication_attempt'],
    legacy_field_clarification='strict_publication_attempt in the raw audit calls ordinary Snapshot::apply only; it is not language semantic acceptance.',
    strict_semantic_publication_accepted=audit['publication_accepted'],
    execution_waivers=0, separate_authority_stop='KLCV5-F-001 / KERML11-68')
with (OUT / 'obligation-causes.json').open('x', encoding='utf8') as stream:
    json.dump(report, stream, indent=2)
    stream.write('\n')
print(json.dumps({key: report[key] for key in [
    'incomplete_reference_count', 'by_diagnostic_family', 'by_expression_context',
    'incomplete_query_count', 'structural_obligation_count',
    'ordinary_kernel_storage_attempt', 'strict_semantic_publication_accepted']}))
