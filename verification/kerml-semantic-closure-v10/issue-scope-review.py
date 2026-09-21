"""Retained-issue scope review; never a substitute for conditional corpus proofs.

The categories below describe the issue's question, not the implementation or
execution disposition of any linked structural constraint.
"""
import collections
import json
import sys
from corpus import *

p = Pinned()
source = OUT / 'issue-inventory.json'
issues = json.loads(source.read_text(encoding='utf8'))['issues']
pending = {i['key']: i for i in issues if i['disposition'] == 'RetainedForStructuralOrExecutionScopeReview' or i['key'] == 'KERML11-75'}
reviews = {}

def review(category, number, rationale):
    key = number if isinstance(number, str) else f'KERML11-{number}'
    assert key not in reviews
    reviews[key] = dict(category=category, rationale=rationale)

for number, rationale in {
    210: 'Lexical name resolution and contextual connector chains must be distinguished. A resolved name alone does not establish the instance-specific binding graph; complete connector/domain analysis remains required.',
    35: 'The issue proposes an additional static type-conformance constraint. Published subsetting currently accumulates types. Existing typing and disjoining constraints still require independent evaluation; no new presumed-disjointness rule is adopted.',
    78: 'All five pinned Flows are longhand. The independently retained transfers/flowTransfers cycle is exercised. A cycle alone is not a structural contradiction; the disjoint MessageTransfer/FlowTransfer types also require analysis before any acceptance claim.',
    69: 'All 23 reference guards agree, but that is not a proof for the independently expanded pinned graph. The Array guard is unchanged.',
    3: 'Six diagnosed cross-multiplicity bound expressions require a domain analysis separate from both KERML11-4 and the owned-cross selector. No alternative domain is adopted.',
    197: 'Longhand FlowEnd and PayloadFeature projections can be empty under the published derivations. The at-most-one payload constraint permits this; the remaining full Flow derivation/validation obligations are not waived.',
    199: 'The pinned superoccurrence Feature exists. The issue questions documentation or library intent; source identity and ordinary structural validity must both be preserved until an authorized correction exists.',
    79: 'accept explicitly redefines the second inherited parameter while its local direction-bearing Feature is first. Exact positional and explicit redefinition closure is still required; the reference source change is not authorized.',
    23: 'Succession cross-multiplicity structure must be checked independently of the temporal-order interpretation. This issue does not authorize tightening the pinned bounds.',
    90: 'Both constructors inherit default FeatureValues. Missing direct defaults is not an absence proof. Complete default-binding graph analysis remains required.',
    75: 'Five direct VectorFunctions imported/owned membership collisions now independently prove the prose/OCL disagreement (KLCV10-F-004). Broader cyclic/inherited/visibility closure remains mandatory; no correction is adopted.',
    72: 'Complete inherited membership and redefinition suppression must be compared with optional redundant-specialization removal; two Triggers findings cannot be waived on the issue title alone.',
    51: 'The question distinguishes nested-feature domains from role-type interpretation. Domain and connector structure cannot be deferred; any additional interpretation conflict requires a pinned local proof.',
    48: 'TransferBefore has a static disjoint-supertype path. An empty extent alone is not forbidden structural syntax. Complete disjoining and multiplicity obligations must establish whether any structural contradiction follows without requiring an instance.',
    34: 'Multiplicity ownership and inherited closure remain structural obligations. The disputed cardinality interpretation must not justify omitting inherited graph facts.',
    29: 'Package-level features and their bounds are present. Domain structure is mandatory even though the consequences for every runtime instance concern extent interpretation.',
    25: 'The sufficient portions Feature is present. The relationship graph and sufficient flag are preserved; proving unintended extent equivalence is distinct from deciding a source correction.',
    22: 'The decision/merge modeling-pattern discrepancy concerns relationships as well as runtime traversals. Exact local union/subsetting/binding populations still need analysis.',
    24: 'The requested sufficiency of spaceTimeCoincidentOccurrences would change library semantics. Its existing graph must be checked without adding the flag.',
}.items(): review('StructuralCorpusProofRequired', number, rationale)

for number, rationale in {
    178: 'Whether a body expression is model-level evaluable can depend on structural operand/referent facts. Do not classify the operation itself as runtime execution merely because actual evaluation is deferred.',
    87: 'The proposed forAll/exists evaluability rule requires a structural predicate/body classification before execution. No unreviewed evaluability extension is implemented.',
    86: 'Exposure of Type::specializes through model libraries is a library/API extension. The existing semantic specialization query remains a mandatory structural operation.',
    5: 'The prose has not followed a constructor evaluability constraint change. Structural modelLevelEvaluable derivation and actual constructor execution are separate obligations.',
}.items(): review('StructuralEvaluabilityReviewRequired', number, rationale)

for number, rationale in {
    31: 'The complaint asks when invariant expressions are evaluated and true. It does not replace the mandatory structural specializations of Expression and Invariant.',
    30: 'Evaluation time for a FeatureValue requires runtime state. Ownership, expression domains, result structure and binding production remain structural and mandatory.',
    73: 'The requested immediate start of doAction changes temporal behavior. Existing entry/do-action successions and all their structural constraints still require checking.',
    52: 'Interruptibility and run-to-completion concern temporal execution. The exact pinned succession and its references still require structural validation; no source edit is authorized.',
    18: 'Interpretation of isAbstract refers to which instances exist. The flag and specialization graph remain structurally checkable without choosing an instance interpretation.',
    71: 'Capturing currentTime versus continuously binding it changes values over time. Both default/non-default and initial structural graph cases remain mandatory; the pinned source is unchanged.',
    42: 'The continuity invariant compares evaluated timestamps under alternative time models. Its references and expression/result structure remain structural requirements.',
    40: 'The issue explicitly concerns execution-algorithm coverage in Annex A. It cannot justify deferring any structural constraint from the normative inventory.',
    44: 'Whether every derived Feature value is computable at all times is an evaluation question. Ownership and FeatureValue binding structure must still be checked.',
    41: 'The issue questions temporal interpretations of Core flags. It does not remove structural flag, direction, specialization, domain or ownership obligations.',
    37: 'Determination and use of inout values concern instance semantics. Explicit direction derivation and parameter/redefinition direction conformance remain structural.',
    21: 'Whether a Step happens during its owning occurrence is a temporal interpretation question. Every applicable published Step specialization, with the already authorized target-name corrections, remains mandatory.',
    19: 'Sufficient and necessary extent conditions concern instance interpretation. Structural Type flags and all existing derivations remain required.',
    20: 'Who may change a directed value requires instance/time semantics. Structural direction, conjugation and redefinition conformance are not deferred.',
    16: 'Constancy over the entire lifetime of a domain instance requires values and time. The canonical flag and any existing structural restrictions remain required.',
    15: 'Dynamic multiplicity compares evaluated cardinalities over time. This does not defer multiplicity ownership, typing, bound structure or featuring domains.',
    14: 'Classifier multiplicity interpretation asks about instance cardinality. The separate validateClassifierMultiplicityDomain structural rule remains mandatory.',
    17: 'Temporal/whole-instance interpretation of isPortion is separate from structural flag conformance and required portion specialization. Neither is silently changed.',
}.items(): review('InstanceOrExecutionInterpretation', number, rationale)

for number, rationale in {
    201: 'Displaying inherited members with a caret is proposed new notation. Inherited semantic memberships are still required and must not be copied into canonical ownership.',
    202: 'Additional numerical and simulation functions are an extension request. Existing required references and pinned function declarations must remain complete without adding unpinned functions.',
    50: 'Eliminating membership metaclasses would change the metamodel. Published membership classes and their constraints remain authoritative in v7.',
    88: 'Dynamic metadata retrieval is a proposed new library/notation facility. Existing metadata ownership, typing and filtering structure must still be validated.',
    47: 'A textual way to specialize anonymous features is an extension request. Canonical identity-based specialization and all references actually present in the pinned corpus still require closure.',
    46: 'Quantified-space triggers are a requested library extension. No additional trigger is added to the frozen corpus.',
    38: 'Automatic inheritance of additional metaproperties is proposed. Existing explicit direction/end/conformance rules and v2 redefinition semantics are retained.',
    36: 'Merging namespaces across files is proposed new authoring semantics. Duplicate declarations must not be silently merged; current import/namespace rules remain mandatory.',
}.items(): review('UnadoptedExtensionProposal', number, rationale)

for number, rationale in {
    209: 'Missing human-readable function definitions do not supply an alternative structural graph. Evaluation semantics remain separate from structural parameter/result validation.',
    39: 'Textual UUID interchange is a representation limitation. The kernel must preserve canonical identities; it does not authorize changing any library source.',
    144: 'This asks to align documentation text with the specification. Original library bytes and comments are preserved; no structural correction is adopted.',
    181: 'The grammar optionality complaint must be distinguished from structural expression semantics. The full pinned corpus parses; no grammar correction is introduced by v7.',
    179: 'The listed undefined grammar references and capitalization require syntax authority review. The owned-cross correction changes no grammar production.',
    105: 'The multiline-note closing example disagrees with the grammar. Pinned lexical corpus verification remains separate from structural semantic acceptance.',
    106: 'The issue asks how lexical notes differ from semantic comments. It does not authorize dropping canonical annotations or changing the grammar.',
    49: 'The declared disjoining from Occurrence is explicit; the prose names its narrower LinkObject subtype. The graph is preserved, and the separate TransferBefore issue is still analyzed.',
    28: 'Missing interchange files for language-description examples are an artifact request, not a correction to the three pinned libraries.',
    26: 'Additional normative XMI/JSON library archives are an artifact request. Pilot reference XMI is corroboration, not replacement authority for pinned library bytes.',
}.items(): review('DocumentationSyntaxOrInterchange', number, rationale)

review('AbsentCorpusAntecedent', 27, 'No pinned Type instance lies outside Classifier or Feature. This absence is computed below from normative metaclass conformance, not names or reference behavior.')
review('OutsideKerMLCorpus', 'SYSML21-274', 'ShapeItems/SpatialItems belong to the Systems Library, excluded from this publication scope. Their geometry is not imported into KerML acceptance.')
review('OutsideKerMLCorpus', 'SYSML21-245', 'The disputed FlowConnectionUsage message notation belongs to SysML. No SysML semantic rules or frontend are introduced.')
review('OutsideKerMLCorpus', 'SYSML21-7', 'The specialized PerformActionUsage/RequirementConstraintMembership naming rules belong to SysML. Generic KerML effective naming remains mandatory.')

assert set(reviews) == set(pending), (sorted(set(pending)-set(reviews)), sorted(set(reviews)-set(pending)))
naked_types = [p.brief(e) for e in p.records if p.is_kind(e, 'Type') and not (p.is_kind(e, 'Classifier') or p.is_kind(e, 'Feature'))]
assert not naked_types
assert all(r['metaclass'] in names for r in p.records.values())
flows = [e for e in p.records if p.is_kind(e, 'Flow')]
assert len(flows) == 5
assert not [e for e in p.records if p.is_kind(e, 'FlowEnd') or p.is_kind(e, 'PayloadFeature')]
rows = [dict(issue=key, issue_text_sha256=__import__('hashlib').sha256(pending[key]['text'].encode('utf8')).hexdigest(),
             title=pending[key]['title'], linked_constraints=pending[key]['linked_constraints'],
             **value, structural_constraints_deferred=False, authorizes_correction=False,
             exhaustive_conditional_corpus_proof_complete=False)
        for key, value in sorted(reviews.items())]
report = dict(format='agentique-v10-additional-issue-scope-review/1', issue_inventory_sha256=digest(source),
              pinned_archive_sha256=digest(p.archive), issues_reviewed=len(rows),
              category_counts=dict(collections.Counter(r['category'] for r in rows)), reviews=rows,
              independently_checked_populations=dict(naked_types=naked_types, kerml_metaclasses_only=True,
                  longhand_flows=[p.brief(e) for e in flows], flow_ends=0, payload_features=0),
              authority_applicability_closed=False, structural_coverage_closed=False,
              scope='All 63 previously scope-unreviewed issues were read. Scope categorization is not a waiver, a completed per-rule applicability proof, or a claim that only registered authority blockers remain.')
encoded = json.dumps(report, indent=2, ensure_ascii=False)+'\n'
path = OUT / 'additional-issue-scope-review.json'
if '--check' in sys.argv: assert path.read_text(encoding='utf8') == encoded
else: path.write_text(encoded, encoding='utf8', newline='\n')
print(f'Reviewed all {len(rows)} additional issue scopes; zero structural constraints deferred; conditional corpus proofs remain open.')
