# KLCV8-F-001: FeatureReferenceExpression binding domain

The exact pinned `ControlFunctions::'.'` declaration independently exercises the
open [OMG issue KERML11-8](https://issues.omg.org/issues/KERML11-8). This affects
`checkFeatureReferenceExpressionBindingConnector`, which is outside the five
rule families authorized by KERML11-145. The failure remains after applying the
authorized contextual-result direction to the separate outer Function binding.

## Pinned authority and source

The [machine-readable proof](../verification/kerml-semantic-closure-v8/feature-reference-authority-conflict.json)
retains literal formal constraints, relevant operations, exact source bytes and
ranges, input hashes, reference identities, inferred relationships and evaluated
predicates. It checks the original Function-Library KPAR and ControlFunctions
entry against the existing library-set manifest. The source is unchanged:

```kerml
abstract function '.' {
    in feature source : Anything[0..*] nonunique {
        abstract feature target : Anything[0..*] nonunique;
    }
    private feature chain chains source.target;
    chain
}
```

The tail `chain` is a FeatureReferenceExpression `E`, with referent `C` (the
authored source.target chain), owned by Function `F` through
ResultExpressionMembership. `E` owns its raw result `R` through
ReturnParameterMembership. These are distinct canonical identities.

| Reference-XMI role | Identity |
| --- | --- |
| Function F | `0faf5d9e-a55e-54b4-9e94-c39e9ecf560a` |
| FeatureReferenceExpression E | `bae21159-312d-5633-85d4-4d1a2507bdd5` |
| Authored chain referent C | `96af035d-1383-5477-8e9a-61f586d7eddc` |
| Raw result R | `d3f706b6-c10e-5a2f-ace7-a6ee92f57563` |
| Inner reference binding B | `1437b712-3673-507e-a849-18dd2445987b` |
| Separate outer Function binding | `0aaa4bf5-a8ea-57d6-b5c2-f4bab40a47b2` |

These reference IDs are not asserted to equal Agentique canonical IDs. The
reference release head was freshly reacquired and matches the retained XMI commit
`fb97b754f29588b8e9c7a35f370880cd15eb29e7`; all 36 XMI input hashes are recorded.
The current pilot commit is `5cca16d846016e62bb1e54e0e50e675254a022ef`.

## Required graph and contradiction

The published prose requires an owned-member BindingConnector between the
referent and the **raw result**. The literal OCL uses the erroneous identifiers
`targetFeature` and `relatedFeatures`; the proof also fails under the clear
prose interpretation `referent` and `relatedFeature`. An editorial spelling
repair therefore cannot solve the domain contradiction.

The local structural inference is present before the check:

- R subsets C and positionally redefines `Performances::Evaluation::result`.
- C has the ordered source.target chain; its terminal contributes to its
  supertype closure, and each chaining Feature conforms to the previous domain.
- B has two ends with ReferenceSubsetting endpoints C and R, positional end
  redefinitions, and the `Links::selfLinks` base specialization.
- R is nonvariable and is not a chain. Its mandatory featuring domain is E.
  C's featuring domain is F. E is featured by F; this does not make E specialize F.
- Complete reference supertype closures include inherited typing and implied
  base relationships. F does not specialize E, and E does not specialize F.

`checkConnectorTypeFeaturing` requires every endpoint to be featured within
every connector featuring type. The relevant truth table is:

| Candidate connector domain | Referent C | Raw result R |
| --- | --- | --- |
| F | Pass | Fail |
| E | Fail | Pass |
| No featuring type | Fail | Fail |

F is a Function classifier, so its compatibility uses specialization. The
Feature common-redefinition alternative cannot rescue E: E owns R, and F is not
a Feature. Variable-feature exceptions do not apply. The prescribed direct and
indirect featuring population contains no common candidate, so
`defaultFeaturingType` is null. No identity ordering decision is involved.

The current reference **already** gives B plain OwningMembership and a
TypeFeaturing to F. This avoids imposing E through FeatureMembership ownership,
but R still fails the F-domain check. Reference behavior is explicitly recorded
as a mismatch, not accepted as a corrected authority.

## Why KERML11-145 does not cover this result

Independently construct a distinct Feature Q with chaining Features `[E, R]`.
Q has F's domain from E. The outer Function connector can relate the Function
result to Q, own that connector via FeatureMembership, and satisfy the Function
domain through its existing specialization. This is the authorized direction for
`checkFunctionResultBindingConnector`.

The inner connector B is still required by a **different constraint** to relate
C and R. Replacing its R endpoint with Q changes its identity-based requirement.
Adding a Function-to-Expression specialization, inventing a common subtype,
changing R's ownership/variable flag, or weakening connector conformance also
changes canonical semantics without authority. Completing ordinary inference
does none of these things.

The proof is local to a complete structural witness; it does not claim that all
Agentique corpus inference is complete. Remaining corpus incompleteness is
reported separately and is not used to infer this conflict.

## Reproduction and disposition

The [independent verifier](../verification/kerml-semantic-closure-v8/feature-reference-authority.py)
uses original KPAR bytes and reference XML, without calling Agentique lowering,
semantic queries or KERML11-145 helpers. It checks the corrected outer graph as a
proof object, not as an implemented v6 publication. The
[arbitrary-name canonical test](../crates/kerml-semantics/tests/feature_reference_authority_v8.rs)
uses complete kernel Snapshots and checks complete query support, including
base specialization and result/end redefinitions, under Published and v1–v5.

This is the independently reproduced Gate 1 stop. No KERML11-8 correction is
applied. KERML11-145 remains authorized, but no v6 implementation or accepted
publication is claimed. See the [authority map](../standards/kerml-1.0-constraint-authority-map.json),
[ADR 0018](adr/0018-operational-result-domain-corrections.md), and
[verification record](../verification/kerml-semantic-closure-v8/README.md).
