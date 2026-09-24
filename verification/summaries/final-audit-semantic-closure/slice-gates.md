# Fresh scoped semantic gates

`verification/scripts/systems_semantic_slice_gate.py` validates the two reviewed
scopes in `slices.json`: `H1-H3-combined` (16 documents) and `H4-medium` (the exact
prior 13 documents). It calls neither producers nor the finalizer and cannot
issue publication authority. A successful report remains unpublished evidence.

Run it against each fresh `sysml_systems_publication --audit-only
--profile=operational-v3` report:

```text
python verification/scripts/systems_semantic_slice_gate.py --slice=H1-H3-combined --report=<combined16-report.json> --output=<combined16-gate.json>
python verification/scripts/systems_semantic_slice_gate.py --slice=H4-medium --report=<medium13-report.json> --output=<medium13-gate.json>
```

The gate checks exact reviewed document paths, pinned archive bytes and stable
document identities; complete parse/construction/byte retention; zero kernel
obligations; complete mandatory references; Complete converged producer closure
with no diagnostics; all applicable pairs and requirements closed; and present,
clean effective publication-query audit evidence with bounded workers. It also
requires explicit v3, the accepted immutable KerML v9 identity without replay,
and false publication attempt/acceptance/authority flags.

The fresh report must also contain `corpus_witnesses` emitted by the real-query
example helper. Combined16 requires all seven EnumerationDefinitions and 21
literals; medium13 contains neither enumeration document and requires zero of
each. Literal IDs, owning EnumerationDefinition IDs, original VariantMemberships
and explicit names are checked against the independently pinned ownership plan.
Each literal must retain exactly one canonical owner FeatureTyping, Complete
effective typing/names and its DataValue subsetting. Variation must be canonically
true on every expected definition.

Both scopes require two BinaryInterface subjects with exactly the original two
ordered PortUsage ends, and all three FlowUsage subjects with exactly the original
Message parameters and their own ends, including original canonical owners. The
fixture IDs appear only in verification code. The gate compares actual answers
with those independent constants even if the report's expected fields are changed
to agree with an incorrect answer.

Both exact HappensDuring ConnectionUsages must retain their original canonical
FeatureTyping edges and plain Association targets. Broad effective Usage typing
retains HappensDuring; the four typed projections must be Complete, conforming,
exclude that plain Association, and preserve the formal subset relationships.
Each exact source case has a nonempty PartDefinition population and corresponding
inherited occurrence/item/connection populations. Aggregate flags alone cannot
replace these checked identities and answer populations. Query witnesses must
report no canonical element-count change and no publication authority.

The prior classification and slice plan have independent reviewed content pins.
The classifier is rerun over all 238 distinct rows and must reproduce the exact
674 weighted findings, 50 subjects and zero `Other` findings. The combined plan
covers all 47 local subjects plus three targets in the accepted KerML graph.
That mapping establishes scope coverage; it is not a substitute for the fresh
effective audit or a claim that dependency targets are separately local subjects.

The medium reference population remains exactly 695. No retained, independently
authenticated combined16 reference total was found: the rejected direct-finalizer
report omitted per-document source-reference counts. Combined validation therefore
requires every fresh document/global reference population to agree and be Complete;
the output labels this evidence limit rather than inventing a historical count.

Each result records the fresh report hash, reviewed fixture hashes, original
source-content identity, all certificate digests, query-result digest, actual
counts, construction-report duration and effective-audit duration separately.
The report's `elapsed_seconds` is captured before the effective audit and is not
presented as end-to-end duration. No wall-clock acceptance threshold is imposed.

Verification: `python -m unittest discover -s verification/scripts -p
test_systems_semantic_slice_gate.py` completed with exit 0, seven tests passed in
0.124 seconds (final run). The tests include pinned original source verification, both exact
scopes, same-count source/scope substitutions, missing/failed effective audits,
incomplete pairs/requirements, producer diagnostics, altered reference counts,
publication/profile/dependency mismatches, malformed digests and modified reviewed
classification/mapping fixtures. Synthetic passing reports in these tests do not
represent executed semantic gates. Fresh combined and medium outcomes are pending.

Witness-extension verification: the same command completed with exit 0 and 11
tests passed in 0.457 seconds. Added negatives cover missing/failed/self-reduced
witness sets, canonical owner/VariantMembership/typing changes, pending explicit
names, lost DataValue subsetting, copied or reordered structural identities,
changed expected and actual fields together, wrong original ConnectionUsage
carriers, missing typed operations and violated projection subsets. Output retains
a digest of the complete witness object and its scope-specific population counts.
