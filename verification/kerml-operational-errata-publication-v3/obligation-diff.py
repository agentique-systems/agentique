"""Compare the preserved published construction with the operational reproduction."""
import json
from pathlib import Path
import sys

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
before_path = OUT.parent/'kerml-standard-library-publication-v2/gate-0-obligations/obligations.json'
after_path = OUT/'gate-6-operational-2/obligations.json'
before = json.loads(before_path.read_bytes())
after = json.loads(after_path.read_bytes())
manifest = json.loads((ROOT/'standards/kerml-1.0-operational-errata.json').read_bytes())
removed_properties = {d['descriptor_id'] for d in manifest['entries'][0]['descriptors'] if d['kind'] == 'Property'}
raw = before['structural_obligations']
operational = after['structural_obligations']
participant = [o for o in raw if o['property_id'] in removed_properties]
remaining = [o for o in raw if o['property_id'] not in removed_properties]
assert not any(o['property_id'] in removed_properties for o in operational)
assert remaining == operational, 'Unrelated obligation changes'
assert before['library_set'] == after['library_set']
assert before['rule_version'] == after['rule_version']
assert after['baseline_profile'] == manifest['profile_id']
assert after['strict_publication_attempt']['accepted'] is False
report = dict(format='agentique-operational-obligation-diff/1',
              published_evidence=str(before_path.relative_to(ROOT)),
              operational_evidence=str(after_path.relative_to(ROOT)),
              published_profile=manifest['published_profile_id'], operational_profile=manifest['profile_id'],
              comparison_rule_set=before['rule_version'],
              note='Gate 6 compares the same /6 rules across profiles; /7 subsequently versions profile-aware context identity without changing resolution rules.',
              library_set=before['library_set'], published_obligations=len(raw), operational_obligations=len(operational),
              published_participant_obligations=len(participant), operational_participant_obligations=0,
              unchanged_non_errata_obligations=len(remaining), removed_obligations=participant,
              remaining_obligations=remaining, strict_publication=after['strict_publication_attempt'])
print(json.dumps(report, indent=2))
if '--output' in sys.argv:
    with Path(sys.argv[sys.argv.index('--output')+1]).open('x') as stream:
        json.dump(report,stream,indent=2)
