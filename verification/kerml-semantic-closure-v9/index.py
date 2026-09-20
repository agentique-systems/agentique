"""Index actual captured commands and distinguish verification from acceptance."""
import hashlib
import json
from pathlib import Path
import sys

OUT=Path(__file__).resolve().parent
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
commands=[]
for result in sorted(OUT.glob('*/result.json')):
    row=json.loads(result.read_text());output=result.parent/row['output']
    commands.append(dict(result=result.relative_to(OUT).as_posix(),result_sha256=sha(result),
        command=row['command'],started=row['started'],exit_code=row['exit_code'],
        output=output.relative_to(OUT).as_posix(),output_sha256=sha(output),duration_seconds=row['duration_seconds']))
for result in sorted(OUT.glob('*/results.json')):
    for row in json.loads(result.read_text()):
        output=result.parent/row['log']
        commands.append(dict(result=result.relative_to(OUT).as_posix(),result_sha256=sha(result),
            command=row['command'],started=row['started'],exit_code=row['exitCode'],
            output=output.relative_to(OUT).as_posix(),output_sha256=sha(output),duration_seconds=row['durationMs']/1000))
commands.sort(key=lambda r:(r['started'],r['output']))
matrix=json.loads((OUT/'full-matrix-1/results.json').read_text())
by_log={r['log']:r['exitCode'] for r in matrix}
for name in ['rust-tests','generated','kerml-readiness','sysml-readiness','rustdoc','standards',
    'frontend-check','frontend-build','node-tests','browser-tests','grammar-inventory','grammar-tables',
    'grammar-tests','historical-lexical-corpus','independent-runtime']:
    assert by_log[name+'.txt']==0,name
for name in ['clippy-final-2','format-final-2','reference-result-matrices-final','seven-profile-lineage',
    'kernel-extensions-1','reference-authority-check','cross-authority-check','measurements-check','preservation-1']:
    assert json.loads((OUT/name/'result.json').read_text())['exit_code']==0,name
preserved=json.loads((OUT/'preservation.json').read_text())
assert preserved['unchanged'] and not preserved['historical_manifests_and_evidence_modified']
status=json.loads((OUT/'strict-publication-status.json').read_text())
assert not status['semantic_publication_accepted']
summary=dict(format='agentique-v9-closing-verification/1',
    successful_final_repository_checks=True,structural_runtime_gates=dict(kerml=0,sysml=0),
    strict_rustdoc_exit=0,profile='agentique-kerml-1.0-operational/6',
    focused_producer_matrices_passed=True,authority_stop='KLCV9-F-001 / KERML11-1',
    preserved_starting_files=preserved['original_files'],verified_acquisitions=preserved['verified_new_acquisitions'],
    development_failures_retained=True,old_raw_conformance_failures_separate=True,
    semantic_publication_accepted=False,full_corpus_v6_expansion_complete=False,
    command_count=len(commands),completion=status['completion'])
for name,data in [('command-index.json',dict(format='agentique-v9-command-index/1',commands=commands)),
    ('closing-verification.json',summary)]:
    encoded=json.dumps(data,indent=2,ensure_ascii=False)+'\n';path=OUT/name
    if '--check' in sys.argv:
        assert path.read_text(encoding='utf8')==encoded
    else:
        path.write_text(encoded,encoding='utf8',newline='\n')
print(json.dumps(summary,ensure_ascii=False))
