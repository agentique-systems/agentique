"""Summarize actual completed checks; never turn failed publication into success."""
import json
from pathlib import Path

OUT = Path(__file__).resolve().parent
normal = {}
for directory in ('final-1', 'final-repair-1', 'final-lint', 'profiles-final', 'gate-7-witness-1', 'gate-6-diff-1', 'review-1'):
    for row in json.loads((OUT/directory/'results.json').read_bytes()):
        normal[row['command']] = dict(row, evidence=f'{directory}/{row["log"]}')
quality_results = json.loads((OUT/'quality-1/results.json').read_bytes())
quality = json.loads((OUT/'quality-1/library-quality.json').read_bytes())
strict = json.loads((OUT/'strict-1/results.json').read_bytes())
diff = json.loads((OUT/'gate-6-diff-1/obligation-diff.json').read_bytes())
preservation = json.loads((OUT/'review-1/preservation.txt').read_bytes())
assert all(row['exitCode'] == 0 for row in normal.values())
assert all(row['exitCode'] == 1 for row in strict)
assert quality_results[0]['exitCode'] == 1
assert quality['semantic_quality_gate_passed'] is False
assert quality['published_snapshot'] is False
assert len(quality['documents']) == 36
report = dict(
    format='agentique-kerml-operational-publication-verification/1',
    result='KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED',
    blocker='KOPV3-F-001 / KERML11-140',
    base='0927d90e2d882dd798e03e30b25150a75ce3ca4e',
    branch='semantics/kerml-operational-errata-publication-v3',
    ordinary_commands=list(normal.values()), ordinary_command_count=len(normal),
    ordinary_exit_zero=sum(row['exitCode'] == 0 for row in normal.values()),
    strict_published_conformance=strict,
    quality_command=quality_results,
    published_participant_obligations=diff['published_participant_obligations'],
    operational_participant_obligations=diff['operational_participant_obligations'],
    unchanged_other_obligations=diff['unchanged_non_errata_obligations'],
    library_quality=dict(
        baseline_profile=quality['baseline_profile'],
        sources=len(quality['documents']),
        lossless=sum(d['exact_source_preservation'] for d in quality['documents']),
        recovery=sum(d['recovery_count'] for d in quality['documents']),
        unresolved=sum(d['unresolved_count'] for d in quality['documents']),
        ambiguous=sum(d['ambiguous_count'] for d in quality['documents']),
        incomplete_references=sum(d['incomplete_reference_count'] for d in quality['documents']),
        mismatched=sum(d['mismatched_endpoint_count'] for d in quality['documents']),
        incomplete_queries=sum(d['incomplete_query_count'] for d in quality['documents']),
        invalid_queries=sum(d['invalid_query_count'] for d in quality['documents']),
        mandatory_obligations=quality['structural_obligation_count'],
        unevaluated_expressions=sum(d['unevaluated_expression_count'] for d in quality['documents']),
        accepted_snapshot=quality['published_snapshot'],
        quality_gate_passed=quality['semantic_quality_gate_passed']),
    protected_files=preservation['protected_files'],
    historical_attempts='Initial compiler and Clippy failures remain in their original run directories; subsequent successful corrections are listed above.',
)
with (OUT/'verified-results.json').open('x', encoding='utf-8') as stream:
    json.dump(report,stream,indent=2,ensure_ascii=False)
    stream.write('\n')
print(json.dumps({k:v for k,v in report.items() if k not in ('ordinary_commands','strict_published_conformance','quality_command')},indent=2,ensure_ascii=False))
