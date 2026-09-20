"""Index actual exits and verify the boundary of the v8 authority stop."""
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]


def read(name):
    return json.loads((OUT/name).read_text(encoding='utf-8-sig'))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(name,value):
    text=json.dumps(value,indent=2)+'\n'
    path=OUT/name
    if '--check' in sys.argv: assert path.read_text(encoding='utf8')==text,name
    else: path.write_text(text,encoding='utf8',newline='\n')


commands=[]
for name in ['result.json','results.json']:
    for path in sorted(OUT.rglob(name)):
        relative=path.relative_to(OUT)
        # The command recording this index is retained separately. Excluding
        # its own capture avoids a self-referential digest/update cycle.
        if relative.parts[0].startswith('closing-'): continue
        data=json.loads(path.read_text())
        for row in data if isinstance(data,list) else [data]:
            log=path.parent/(row.get('output') or row['log'])
            assert log.is_file(),str(log)
            commands.append(dict(result=relative.as_posix(),result_sha256=sha(path),
                command=row['command'],started=row['started'],
                exit_code=row.get('exit_code',row.get('exitCode')),
                output=log.relative_to(OUT).as_posix(),output_sha256=sha(log),
                duration_seconds=row.get('duration_seconds',row.get('durationMs',0)/1000)))
commands.sort(key=lambda r:(r['started'],r['result'],r['output']))
write('command-index.json',dict(format='agentique-v8-command-index/1',commands=commands,
    interpretation='All actual exits retained; nonzero quality/coverage/stale/conformance and baseline failures are not converted into acceptance.'))
full={r['log']:r for r in read('full-matrix-1/results.json')}
assert len(full)==18
assert all(r['exitCode']==0 for name,r in full.items() if name!='preservation.txt')
assert all(r['exitCode']==0 for r in read('final-rust-2/results.json'))
assert read('historical-profile-matrix/result.json')['exit_code']==0
assert read('baseline-quality-2/result.json')['exit_code']==1
assert read('all-constraint-coverage-gate-final/result.json')['exit_code']==1
assert read('binding-stale-check/result.json')['exit_code']==1
for label in ['authority-map-offline','authority-matrix-offline','feature-reference-proof-offline',
    'canonical-source-offline','archive-offline','corpus-audits-final-offline','preservation-final']:
    assert read(f'{label}/result.json')['exit_code']==0,label
proof=read('feature-reference-authority-conflict.json')
assert proof['id']=='KLCV8-F-001' and not proof['correction_authorized']
assert proof['counterfactual_authorized_145']['outer_connector_domain_passes']
assert not proof['counterfactual_authorized_145']['inner_connector_domain_passes']
coverage=read('validation-coverage.json')
assert len(coverage['constraints'])==258 and not coverage['publication_gate_passed']
assert coverage['reviewed_errata_awaiting_implementation']==5
publication=read('strict-publication-status.json')
assert not publication['accepted'] and not publication['v6_implemented']
assert read('preservation.json')['unchanged']
assert read('archived-exports.json')['byte_equal_to_historical_v7']
restored=read('historical-replay-restoration.json')
assert all(r['json_value_equal'] for r in restored['restored'])
samples=read('memory-samples.json')
report=dict(format='agentique-v8-closing-verification/1',
    base_commit=read('preflight.json')['base_commit'],
    branch=read('preflight.json')['branch'],authority_stop='KLCV8-F-001',
    operational_v6_implemented=False,semantic_publication_accepted=False,
    both_complete_structural_runtime_gates_green=all(full[n]['exitCode']==0 for n in ['kerml-readiness.txt','sysml-readiness.txt']),
    strict_raw_conformance_exits=[r['exitCode'] for r in read('raw-conformance-1/results.json')],
    fresh_quality=read('quality-comparison.json')['current'],
    formal_coverage=dict(category_counts=coverage['category_counts'],pending_errata_implementation=5,
        structural_implementation_obligations=coverage['structural_implementation_obligations'],closed=False),
    memory=dict(samples=len(samples),maximum_observed_peak_working_set_since_start=max(s['peak_working_set_since_process_start'] for s in samples),
        sampling_scope='Late-start sampling of the existing deterministic 128-element-batch full corpus audit; not full authored/deep/parallel stress acceptance.'),
    not_claimed=['production contextual-result API','v6 profile/manifests','five-family production corrections',
        'seven-profile correction matrix','full reference completeness','full chain/position closure',
        'all structural validation implemented','new strict acceptance operation','accepted library Snapshot',
        'accepted bindings','LoadedKermlStandardLibraries','authored accepted-library integration'],
    historical_bytes_preserved=True,default_profile_changed=False,no_execution_deferral_for_missing_structure=True,
    completion_phrase='KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED')
write('closing-verification.json',report)
print('Both runtime gates green; final workspace/authority checks recorded; KLCV8-F-001 remains unauthorized; publication withheld.')
