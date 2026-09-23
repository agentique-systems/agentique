# Instantiation target population

The pinned KerML 1.0 `InstantiationExpression::instantiatedType` operation
(`standards/normative/kerml-1.0/KerML.xmi`, lines 496–515) rejects
FeatureMemberships before selecting the first remaining owned Membership.
The query previously selected that same target but retained the entire
Membership population as a producer dependency. A pending writer of only
FeatureMemberships therefore reopened an otherwise complete Invocation row.

The query now uses the existing `owned_relationships_excluding` contract.
It preserves canonical ownership order, accepts eligible declared and derived
memberships, and retains the selected member endpoint evidence. It does not
certify absence from an incomplete source or change operator target lookup.

The targeted red regression used the actual Invocation descriptor and a pending
subject-scoped FeatureMembership writer. It returned the correct Function but
left Invocation Pending in both query evaluator modes. The precise query makes
that row Complete; unknown or qualifying Membership writers and explicit broad
reads merged in either order still reopen it. Additional controls exercise
selected target reconstruction and derived eligible memberships.

Exact commands, exits, source identities and output hashes are in
`instantiation-population-commands.json`. The initial additional-control build
failed because the test used nonexistent convenience methods; the corrected
fixture uses the ordinary context and materialization APIs. No standard
publication/cache was loaded and no historical accepted KerML identity changed.
