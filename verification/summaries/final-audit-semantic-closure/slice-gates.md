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
