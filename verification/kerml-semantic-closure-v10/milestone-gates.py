"""Review every requested v10 gate without equating traversal with closure."""
import hashlib
import json
from pathlib import Path
import sys

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
def read(path): return json.loads(path.read_text(encoding='utf8'))
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()

review = read(OUT / 'publication-review.json')
audit = read(OUT / 'full-v7-complete.json')
selector = read(OUT / 'expanded-selector-verification.json')
assert not review['semantic_publication_accepted']
assert selector['all_comparisons_passed']
rows = []
def gate(number, name, status, detail):
    rows.append(dict(gate=number, name=name, status=status, detail=detail))

gate(0, 'Freeze KERML11-1 authority', 'Complete', 'Exact commit/parent/blob/diff, regression, current pilot, issue, XMI and pinned witness packet verified independently.')
gate(1, 'Operational v7 profile', 'Complete', 'V7 extends frozen v6; correction identity participates in semantic context and authored metadata; default remains v2.')
gate(2, 'ownedCrossFeature selector', 'Complete', 'Reviewed eligibility and exclusions use normative metaclass conformance.')
gate(3, 'Semantic ownership ordering', 'Complete', 'First eligible owned membership follows canonical ordered ownership; incomplete projection does not select an ID.')
gate(4, 'Cross-feature derivation', 'Incomplete', 'Separate selector/subsetting/cross queries exist; all typing, featuring and specialization consequences are not closed.')
gate(5, 'Exact Occurrences witnesses', 'Complete', 'Both exact expanded witnesses independently select none; source ownership order is preserved.')
gate(6, 'Synthetic correction matrix', 'Complete', 'Eight-profile 120-row matrix and additional incomplete/produced-infrastructure regressions pass.')
gate(7, 'Independent selector verifier', 'Complete', 'Independent Python/XMI path checks all source Features, synthetic rows and both expanded witness sequences.')
gate(8, 'Full authority applicability sweep', 'Incomplete', 'All 258 constraints, 218 derived/operation entries and 410 issues traversed; per-antecedent proof closure remains incomplete.')
gate(9, 'Aggregate authority blocker register', 'Incomplete', 'All four independently proved conflicts are registered. Incomplete applicability proof prevents certifying an exhaustive final blocker total.')
gate(10, 'Independent ordinary structural implementation', 'Incomplete', 'Reference scope, typing, value/result structure and resource handling improved after authority conflicts; ordinary backlog remains.')
gate(11, 'Unresolved required references', 'Complete' if audit['expanded_references']['unresolved'] == 0 else 'Incomplete', f"Expanded required references: {audit['expanded_references']['count']}; unresolved: {audit['expanded_references']['unresolved']}.")
gate(12, 'Incomplete required references', 'Incomplete', f"Expanded incomplete answers: {audit['expanded_references']['incomplete']}; no unique-candidate waiver is applied.")
gate(13, 'Namespace distinguishability', 'Incomplete', f"Expanded distinct findings: {audit['distinguishability']}; Triggers positional/chain closure remains unfinished.")
gate(14, 'FeatureValue structural semantics', 'Incomplete', 'Source population and implemented producer graph audited; domain/default-binding and dependent validation closure remain incomplete.')
gate(15, 'Feature-chain structural semantics', 'Incomplete', 'Ordered stored/contextual chains and terminal query supported; complete implied graph obligations remain.')
gate(16, 'Positional semantics', 'Incomplete', 'Complete inherited parameter and chain interaction remains unfinished; no ID-order substitution is used.')
gate(17, 'Required derived operations', 'Incomplete', 'The complete inventory is retained, with partial query mappings and missing publication proofs explicitly marked.')
gate(18, 'Structural validation inventory', 'Incomplete', f"Actual coverage: {review['structural_coverage_status_counts']}.")
gate(19, 'Strict DeferredExecution boundary', 'Complete', 'Zero unfinished structural rules are classified DeferredExecution; actual evaluation remains separate.')
gate(20, 'Full v7 corpus expansion', 'Incomplete', f"All implemented producers and audit queries ran over all three libraries: {audit['derived_overlay_count']} derived records, {audit['query_count']} queries. Required absent producer families prevent full semantic closure.")
gate(21, 'Strict semantic publication operation', 'Incomplete', 'The evidence gate fails closed. The production language-semantic acceptance API is not finalized.')
gate(22, 'Authority-only blocked completion', 'NotReached', 'Ordinary structural implementation is still incomplete; the authority-only completion phrase is forbidden.')
gate(23, 'Accepted immutable publication', 'Withheld', 'Semantic acceptance fails; no accepted Snapshot IDs exist.')
gate(24, 'Accepted bindings', 'Withheld', 'Historical stale-binding check fails; accepted regeneration/version advancement requires semantic acceptance.')
gate(25, 'LoadedKermlStandardLibraries', 'Withheld', 'No facade exposes the unaccepted corpus as an accepted publication.')
gate(26, 'Authored integration', 'Incomplete', 'V7 authored metadata is tested and integration is designed; the full deliberately unaccepted-fixture integration matrix is unfinished.')
gate(27, 'Eight-profile historical matrix', 'Complete', 'Published/v1-v7 correction boundaries and preservation checks pass; default v2 and prior manifests remain unchanged.')
gate(28, 'Resource and batch invariance', 'Incomplete', 'Bounded full-corpus audit and focused complete-answer concurrency/partition regressions pass. Exhaustive all-producer-family partition invariance remains unproved.')
gate(29, 'Documentation', 'Complete', 'Profile documentation, ADR 0020, appended foundation review and evidence index retain all 26 explicit answers and the incomplete status.')
assert [row['gate'] for row in rows] == list(range(30))
report = dict(format='agentique-v10-milestone-gates/1', gates=rows,
              ordinary_structural_implementation_complete=False,
              semantic_publication_accepted=False, authority_only_completion_allowed=False,
              inputs={p.name: sha(p) for p in [OUT/'publication-review.json', OUT/'full-v7-complete.json', OUT/'expanded-selector-verification.json']})
encoded = json.dumps(report, indent=2, ensure_ascii=False) + '\n'
path = OUT / 'milestone-gates.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8') == encoded
else: path.write_text(encoded, encoding='utf8', newline='\n')
print('All 30 requested gates accounted for; ordinary structural gaps and publication prerequisites remain explicit.')
