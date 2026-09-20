"""Index real command evidence and keep every acceptance boundary explicit."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()
base=json.loads((OUT/'preflight.json').read_text())['base_commit']
for name in ['docs/operational-kerml-profile.md','docs/kerml-standard-library-foundation-review.md']:
    original=subprocess.check_output(['git','cat-file','blob',base+':'+name],cwd=ROOT)
    assert (ROOT/name).read_bytes().startswith(original),name
v2=json.loads((ROOT/'standards/kerml-1.0-operational-errata-v2.json').read_text())
frozen=(ROOT/'verification/kerml-library-content-errata-publication-v5/operational-profile-v2-frozen.md').read_bytes()
assert sha(frozen)==v2['entries'][0]['algorithm_sha256']
assert (ROOT/v2['entries'][0]['algorithm']).read_bytes().startswith(frozen)
commands=[]
for directory in sorted(p for p in OUT.iterdir() if p.is_dir() and not p.name.startswith('closing-')):
    if (directory/'result.json').exists():
        row=json.loads((directory/'result.json').read_text())
        commands.append(dict(evidence=(directory/'result.json').relative_to(OUT).as_posix(),**row))
    if (directory/'results.json').exists():
        for row in json.loads((directory/'results.json').read_text()):
            commands.append(dict(evidence=(directory/'results.json').relative_to(OUT).as_posix(),**row))
full=json.loads((OUT/'full-matrix-1/results.json').read_text())
runtime={r['log']:r['exitCode'] for r in full if r['log'] in ['kerml-readiness.txt','sysml-readiness.txt']}
assert runtime=={'kerml-readiness.txt':0,'sysml-readiness.txt':0}
assert all(r['exitCode']==0 for r in json.loads((OUT/'final-rust-3/results.json').read_text()))
for name in ['format-final','clippy-final','historical-profile-matrix-final','authority-matrix-final',
             'result-binding-authority-final','end-membership-independent','six-profile-review']:
    assert json.loads((OUT/name/'result.json').read_text())['exit_code']==0,name
coverage=json.loads((OUT/'validation-coverage.json').read_text())
quality=json.loads((OUT/'quality-comparison.json').read_text())
memory=json.loads((OUT/'memory-final.json').read_text(encoding='utf-8-sig'))
if isinstance(memory,dict): memory=[memory]
assert not coverage['complete'] and coverage['named_constraints']==258
assert json.loads((OUT/'v5-quality-final-2/result.json').read_text())['exit_code']==1
sources=['crates/kerml/src/profiles.rs','crates/kerml-semantics/src/context.rs',
         'crates/kerml-semantics/src/contract.rs','crates/kerml-semantics/src/formal_targets.rs',
         'crates/kerml-semantics/src/implicit.rs','crates/kerml-text/src/library/construction.rs']
captured=json.loads((OUT/'v5-quality-final-2/input-hashes.json').read_text())
for record in captured:
    assert sha((ROOT/record['path']).read_bytes())==record['sha256'],record['path']
dependency=dict(format='agentique-v7-dependency-review/1',complete_publication_dependency_audit=False,
    review=[
        dict(dependency='profile and formal errata',implementation='SemanticContext profile ID, frozen manifest digest, typed FormalConstraintTarget search, exact bound IDs, facts and model digest.'),
        dict(dependency='library corrections',implementation='Original v3 reviewed origins/IDs accepted only along explicit v3/v4/v5 lineage.'),
        dict(dependency='namespace population',implementation='Existing ownership, membership, incoming/property and root population search dependencies remain required.'),
        dict(dependency='positional ordering',implementation='Owned relationship/membership ordering retained. Full independent target comparison remains unfinished.'),
        dict(dependency='feature-chain expansion',implementation='Existing terminal-chain query dependencies retained. Full structural expansion and dependency closure remain unfinished.'),
        dict(dependency='standard bindings',implementation='Candidate anchor identity/version and graph digest remain explicit. Accepted bindings/facade are not issued.')],
    sources=[dict(path=p,sha256=sha((ROOT/p).read_bytes())) for p in sources],
    stress=dict(existing_workspace_suite='final-rust-3/results.json',full_library_audit='v5-quality-final-2/result.json',
        memory='memory-final.json',batch_size=128,identical_context_asserted_at_each_batch=True,
        accepted_publication_concurrent_reader_and_authored_integration=False))
closing=dict(format='agentique-v7-closing-verification/1',
    branch=subprocess.check_output(['git','branch','--show-current'],cwd=ROOT,text=True).strip(),
    result='KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED',
    authority_stop='KLCV7-F-001 / KERML11-145',operational_v5=True,
    authorized_formal_corrections=['KERML11-205','KERML11-206','KERML11-207'],
    pinned_starting_bytes_unchanged=json.loads((OUT/'preservation.json').read_text())['unchanged'],
    new_unauthorized_correction_applied=False,structural_runtime_gates=runtime,
    audited_crate_and_standard_inputs_unchanged_since_capture=True,
    historical_review_and_profile_doc_prefixes_unchanged=True,v2_frozen_algorithm_bytes_unchanged=True,
    quality=quality['v5']['totals'],formal_coverage=coverage['category_counts'],
    observed_os_peak_working_set_bytes=max(r['peak_working_set_bytes'] for r in memory),
    strict_semantic_publication=False,accepted_snapshot=False,accepted_bindings=False,
    loaded_kerml_standard_libraries=False,accepted_authored_project_consumption=False,
    execution_deferred=True,sysml_work_started=False,
    known_nonzero_reports=['Full semantic quality and formal coverage: acceptance withheld.',
        'Historical candidate binding manifest stale; no accepted regeneration.',
        'Strict raw metamodel conformance: separate published authoring anomalies.',
        'Existing external Controller fixture RES001.',
        'Historical v5 acquisition-index discrepancies already present on clean main.'],
    command_count=len(commands),
    command_index_scope='All captured command directories except closing index/self-check runs, which retain their own separate logs.')
for name,value in [('command-index.json',commands),('dependency-review.json',dependency),('closing-verification.json',closing)]:
    encoded=json.dumps(value,indent=2)+'\n'
    path=OUT/name
    if '--check' in sys.argv: assert path.read_text(encoding='utf8')==encoded,name
    else:path.write_text(encoded,encoding='utf8',newline='\n')
print(json.dumps(dict(commands=len(commands),runtime=runtime,authority_stop=closing['authority_stop'],publication=False)))
