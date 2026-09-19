# KLCV5-F-001 — Feature-chain end-conformance authority conflict

Strict semantic publication remains blocked by a conflict distinct from the
authorized KERML11-76 library corrections. The directly corresponding issue is
[KERML11-68](https://issues.omg.org/issues/KERML11-68), currently open.

The untouched pinned `StatePerformances.kerml` gives
`StateTransitionPerformance::transitionLinkTarget` the feature value
`transitionLink.laterOccurrence`. Its target is the end feature
`Occurrences::HappensBefore::laterOccurrence`.

KerML 1.0 section 8.4.4.9.6, printed page 270 (PDF page 296), supplies the semantic
equivalent of a feature-chain expression. Its nested target feature is declared
without `end`, and must redefine both the operator's nested target and the
selected feature. The pinned metamodel defaults `Feature::isEnd` to false.
The pinned `validateRedefinitionEndConformance` constraint, printed page 175
(PDF page 201), requires an end target to be redefined by an end feature.

These facts independently yield:

| Fact | Value |
| --- | --- |
| Selected target's `isEnd` | true |
| Prescribed nested redefining feature's `isEnd` | false |
| Required implication | false |

The current reference implementation's implied XMI reproduces the same failed
implication. This is not inferred from Agentique's incomplete feature-chain
lowering. The independent verifier follows actual XMI Redefinition endpoints and
literal flags. A separate arbitrary-name canonical witness checks all four flag
combinations using the newly implemented ordinary structural validator.

Reproduction:

```text
python verification/kerml-library-content-errata-publication-v5/verify-authority-stop.py
cargo test --locked --offline -p agq-kerml-semantics --test structural_validation
```

The [machine-readable witness](../verification/kerml-library-content-errata-publication-v5/authority-stop.json)
records pinned source bytes/range/hash, exact formal constraint/default, reference
XMI IDs and the failed endpoint pair. The captured issue page, current release
commit, acquisition URLs/hashes, PDF extraction and actual command results are in
the same evidence directory.

KERML11-81 changes participant descriptors. KERML11-140 changes redefinition
lookup. KERML11-76 repairs inherited-member collisions in four other documents.
None authorizes changing feature-chain equivalence or end conformance. Repairing
this conflict would require choosing a new semantic correction: changing the
nested feature's end status, relaxing the constraint, or changing the pinned
library expression. No such choice has been smuggled into v3.

This meets the separate-authority stop policy: it is independently reproduced,
isolated from Agentique construction defects, outside the existing correction
authorities, and requires a materially different semantic rule or library fact.
Ordinary implementation gaps remain work after that authority decision. Neither
this witness nor successful kernel storage constitutes accepted publication.
