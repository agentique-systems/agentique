"""Close evidence from actual completed commands; semantic acceptance stays false."""
import hashlib
import json
from pathlib import Path
import subprocess
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[1]
def read(p):return json.loads(p.read_text())
def write(name,data):(OUT/name).write_text(json.dumps(data,indent=2)+'\n')
commands=[]
for path in OUT.glob('*/result.json'):
    row=read(path)
    commands.append(dict(**row,directory=path.parent.name,
                         log=(path.parent/row['output']).relative_to(OUT).as_posix()))
for path in OUT.glob('*/results.json'):
    for row in read(path):
        commands.append(dict(command=row['command'],started=row['started'],
                             exit_code=row['exitCode'],duration_seconds=row['durationMs']/1000,
                             directory=path.parent.name,
                             log=(path.parent/row['log']).relative_to(OUT).as_posix()))
commands.sort(key=lambda r:(r['started'],str(r['command'])))
write('command-index.json',commands)
required=[
    ('format-final','cargo fmt'),('clippy-final-2','cargo clippy'),
    ('rust-regression-final','cargo test --workspace'),
    ('complete-matrix-1','cargo run --locked --offline -p agq-metamodel-gen -- --check'),
    ('complete-matrix-1','--baseline kerml-1.0 --require-runtime'),
    ('complete-matrix-1','--baseline sysml-2.0 --require-runtime'),
    ('complete-matrix-1','cargo doc'),('complete-matrix-1','npm run standards:check'),
    ('complete-matrix-1','npm run check'),('complete-matrix-1','npm run build'),
    ('complete-matrix-1','npm test'),('complete-matrix-1','npm run test:e2e'),
    ('authority-final-2','verify-authority.py'),('subobject-authority-final-2','verify-subobject-authority.py'),
    ('end-matrix-final','redefinition_end_v4'),('authored-profile-matrix-final','authored_redefinition_profile_matrix'),
    ('ordinary-validation-matrix','ordinary_validation_v6'),('preservation-final','preservation.py'),
    ('workspace-build-final','cargo build')]
passed=[]
for directory,needle in required:
    matches=[r for r in commands if r['directory']==directory and needle in
             (r['command'] if isinstance(r['command'],str) else ' '.join(r['command']))]
    assert len(matches)==1,(directory,needle)
    assert matches[0]['exit_code']==0,matches[0]
    passed.append(matches[0])
quality=read(OUT/'quality-comparison.json')
coverage=read(OUT/'validation-coverage.json')
conflict=read(OUT/'subobject-authority-conflict.json')
assert conflict['id']=='KLCV6-F-001' and not conflict['changes_applied']
assert not quality['accepted_semantic_publication'] and not coverage['complete']
states=[
 (0,'complete','Independent KERML11-68 authority packet and actual Git objects'),
 (1,'partial','V4 identity/context/authored metadata implemented; no library publication identity issued'),
 (2,'complete','Exact owner-restricted rule; prior profiles retain the published implication'),
 (3,'complete','220 flag/owner/profile rows plus unresolved endpoint check'),
 (4,'partial','Pinned StatePerformances/reference pair evaluated; complete canonical feature-chain expansion remains unfinished'),
 (5,'partial','Complete fresh corpus recounted; every current diagnostic classified A/B with individual facts. Full semantic cause closure remains unfinished'),
 (6,'partial','Stored ownership ordering, inherited result correspondence and typed name ambiguity implemented; full closure unestablished'),
 (7,'not_complete','Full implied positional target comparison not completed'),
 (8,'partial','Canonical inherited result identities and local positional redefinition implemented'),
 (9,'partial','Terminal feature-chain end population implemented; full structural expansion remains required'),
 (10,'not_accepted','Fresh distinguishability counts retained; no accepted closure claim'),
 (11,'not_complete','Structural expression findings retained separately from executable evaluation'),
 (12,'authority_blocked','KLCV6-F-001 independently reproduced; other mandatory coverage remains unfinished'),
 (13,'partial','K68 validator/test diff and matching canonical matrix; broad differential suite unfinished'),
 (14,'not_issued','No strict semantic publication acceptance'),
 (15,'not_issued','No accepted LoadedKermlStandardLibraries facade'),
 (16,'not_issued','No accepted binding regeneration before accepted publication'),
 (17,'not_complete','V4 authored profile metadata tested; accepted library consumption unavailable'),
 (18,'partial','Implemented rules retain fact/search/context dependencies; full publication dependency audit unfinished'),
 (19,'not_run','Accepted publication stress suite cannot run without accepted publication'),
 (20,'complete','Appended profile governance and ADR 0016'),
 (21,'complete','All nineteen review questions answered without claiming completion')]
write('gate-status.json',dict(completion='KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED',
    stop='KLCV6-F-001',stop_classification='E',ordinary_gaps_are_not_stop_reasons=True,
    gates=[dict(gate=n,status=s,disposition=d) for n,s,d in states]))
write('closing-verification.json',dict(
    branch=subprocess.check_output(['git','branch','--show-current'],cwd=ROOT,text=True).strip(),
    base_commit=read(OUT/'preflight.json')['base_commit'],
    required_completed_checks=passed,
    complete_structural_runtime_gates={'kerml':'passed, exit 0','sysml':'passed, exit 0'},
    strict_published_metamodel_conformance='Separate failing authoring audit; see published-conformance',
    semantic_quality=quality['current'],formal_coverage_complete=False,
    named_formal_constraints=coverage['named_constraints'],explicit_checks=coverage['explicit_checks'],
    original_bytes=read(OUT/'preservation.json'),
    authority_stop='KLCV6-F-001',accepted_publication=False,
    accepted_bindings=False,accepted_facade=False,authored_accepted_library_consumption=False,
    executable_evaluation='Deferred separately; never a waiver for structural incompleteness',
    preserved_failed_and_superseded_commands=[r for r in commands if r['exit_code']!=0]))
print(json.dumps(dict(required_checks_passed=len(passed),authority_stop='KLCV6-F-001',
    publication_accepted=False,current=quality['current']['totals'])))
