# Invocation producer scope

An actual `KerML.Invocation` descriptor left a nested argument expression's
result typing open solely because the enclosing invocation was pending. Its
planner instead writes the invocation and, for non-Function targets, its own
direct result. The test-first reproduction fails on `83ca429`; Function and
non-Function planner controls pass.

`SubjectAndOwnedResults` is an appended generic scope selecting the subject and
direct ReturnParameterMembership-owned Features. Masks, causal readers and effect
audits share the selector. Unknown endpoints and potential ownership/reference
writes remain conservative; reconstruction recomputes the selection. Existing
scope discriminants and accepted KerML publication bytes remain unchanged.

The narrowed scope also exposed an existing missing FeatureChainExpression
Membership effect: creation of its source-target member under an existing input
had accidentally borrowed Invocation's former broad permission. The descriptor
now declares that actual effect. The corresponding audit control rejects its
removal; producer output is unchanged.

Five scope tests, the actual feature-chain effect audit, and package library/test
Clippy pass. Exact commands, earlier failing controls and output hashes are in
`invocation-scope-commands.json`. The combined language expression gate and corpus
acceptance remain separate checks.
