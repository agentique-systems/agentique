"""Assemble actual gate outcomes; an audit process succeeding is not acceptance."""
import hashlib
import json
from pathlib import Path
from evidence_io import read_json

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
quality = read_json(OUT / 'quality-comparison.json')
causes = read_json(OUT / 'obligation-causes.json')
patch = read_json(OUT / 'operational-patch-diff.json')
coverage = read_json(OUT / 'validation-coverage.json')
manifest = read_json(ROOT / 'standards/kerml-1.0-operational-library-errata-v3.json')
authority = read_json(OUT / 'authority-matrix.json')
runtime = {}
for baseline in ['kerml-1.0', 'sysml-2.0']:
    path = ROOT / 'standards/generated' / baseline / 'full-audit.json'
    errors = read_json(path)['runtime_errors']
    assert errors == []
    runtime[baseline] = dict(errors=errors, audit=str(path.relative_to(ROOT)),
                             sha256=hashlib.sha256(path.read_bytes()).hexdigest())

gates = [
    (0, 'complete', 'Four named libraries and all 21 fresh baseline diagnostics individually inventoried; ten root patterns, including latent collisions.'),
    (1, 'complete', 'Typed canonical transform; no pinned text, syntax or archive replacement.'),
    (2, 'complete', 'Generic ReviewedCorrection provenance on introduced/replaced facts; original parsed model and source assertions preserved.'),
    (3, 'complete', 'Private content/profile/entry/path-qualified deterministic UUID domain; independently regenerated IDs agree.'),
    (4, 'complete', 'Explicit operational v3; published, v1 and v2 identities preserved. Default remains v2.'),
    (5, 'complete', 'Minimal reviewed Objects combined types/common redefinitions/typings; all three corpus conflicts eliminated.'),
    (6, 'partial', 'All four correction sets implemented and independently verified as source/model facts. Inference remains incomplete in FeatureReferencingPerformances and Observation.'),
    (7, 'complete', 'Independent published-to-v3 diff: 53 new records, zero removed IDs, zero unreviewed changes, all seven required dimensions.'),
    (8, 'failed', 'Six category-B distinguishability findings remain unwaived.'),
    (9, 'incomplete', 'Implied naming ordering remains incomplete; no invented precedence.'),
    (10, 'incomplete', 'All 453 incomplete reference answers classified individually: KQ_IMPLIED_NAMING_ORDER.'),
    (11, 'incomplete', 'All 3496 expression-result findings have one owned result; inherited-result inference/validation remains required.'),
    (12, 'incomplete', 'All 258 constraints inventoried conservatively; 28 explicit checks, required coverage not established.'),
    (13, 'failed', 'Strict semantic quality gate rejects the candidate. Ordinary kernel storage acceptance is separately demonstrated.'),
    (14, 'not_issued', 'Acceptance prerequisite unmet; historical candidate bindings remain stale and are not relabeled accepted.'),
    (15, 'not_issued', 'Acceptance prerequisite unmet; no LoadedKermlStandardLibraries accepted facade.'),
    (16, 'partial', 'Authored profile selection tested across all four profiles; accepted library dependency integration cannot be issued.'),
    (17, 'partial', 'Correction facts carry profile/content/entry/output authority through existing proof contracts; profile mismatch is rejected. Complete publication-wide conclusions remain unavailable.'),
    (18, 'failed', 'Structural-semantic quality remains incomplete; no accepted bindings or publication.'),
    (19, 'complete', 'Historical descriptor/redefinition/collision witnesses pass, with explicit v3 cases and frozen v2 authority bytes.'),
    (20, 'complete', 'V5 review, living profile extension, ADR 0015 and separate authority-conflict document published locally.'),
]
commands = []
for path in sorted(OUT.glob('*/result.json')):
    row = read_json(path)
    commands.append(dict(evidence=str(path.relative_to(OUT)), **row))
for path in sorted(OUT.glob('*/results.json')):
    for row in read_json(path):
        commands.append(dict(evidence=str(path.relative_to(OUT)), **row))
required = []
for directory in ['final-rust-3', 'docs-final', 'final-1', 'product-1', 'strict-1', 'authority-final']:
    required.extend(dict(evidence=directory + '/results.json', **row)
                    for row in read_json(OUT / directory / 'results.json'))
report = dict(format='agentique-kerml-v5-gate-status/1',
    completion='INCOMPLETE', accepted_semantic_publication=False,
    stop=dict(id='KLCV5-F-001', issue='KERML11-68',
        evidence='authority-stop.json', independently_reproduced=True,
        isolated_from_agentique_construction_defects=True,
        outside_existing_81_140_76_authorities=True,
        materially_different_semantic_or_source_correction_required=True,
        ordinary_implementation_findings_waived=False),
    git=read_json(OUT / 'preflight.json'),
    correction=dict(profile=manifest['profile_id'], entries=len(manifest['entries']),
        selectors=len(manifest['selectors']), operations=sum(len(e['operations']) for e in manifest['entries']),
        manifest_sha256=patch['review_sha256'], introduced_records=patch['new_records'],
        removed_records=patch['removed_records'], unreviewed_changes=patch['unreviewed_changes'],
        all_four_libraries_audited=authority['all_four_named_models_audited'],
        independent_root_witnesses=len(read_json(OUT / 'four-model-witnesses.json')['cases']),
        issue_open=True, adopted_omg_correction=False),
    profiles=quality['profiles'], source_assertions=read_json(OUT / 'source-assertion-diff.json'),
    incomplete_references=dict(count=causes['incomplete_reference_count'],
        by_cause=causes['by_diagnostic_family'], evidence='obligation-causes.json'),
    validation=dict(named_constraints=coverage['named_constraints'],
        explicit_checks=coverage['explicit_checks'], required_scope_complete=coverage['complete']),
    ordinary_kernel_storage=causes['ordinary_kernel_storage_attempt'],
    runtime=runtime, raw_metamodel_conformance_separate=True,
    accepted_bindings=False, accepted_facade=False, accepted_authored_library_integration=False,
    gates=[dict(gate=number, status=status, reason=reason) for number, status, reason in gates],
    required_command_results=required,
    nonzero_results='README.md distinguishes actual failed quality gates, known negative checks, unrelated reference drift, and superseded attempts.')
assert report['correction']['unreviewed_changes'] == 0
assert report['correction']['all_four_libraries_audited']
assert not report['accepted_semantic_publication']
for name, data in [('gate-status.json', report), ('command-index.json', commands)]:
    with (OUT / name).open('x', encoding='utf8') as stream:
        json.dump(data, stream, indent=2)
        stream.write('\n')
print(json.dumps(dict(completion=report['completion'], correction=report['correction'],
                     runtime_errors={key: len(row['errors']) for key, row in runtime.items()},
                     recorded_commands=len(commands))))
