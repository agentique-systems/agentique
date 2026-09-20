# KLCV9-F-001: end values become owned cross features

The exact pinned Semantic Library exercises a new independent structural
authority conflict associated with open **KERML11-1**. It is outside v1–v6 and
prevents semantic acceptance. The complete machine-readable proof is
[cross-feature-authority-conflict.json](../verification/kerml-semantic-closure-v9/cross-feature-authority-conflict.json).

The two witnesses are `Occurrences::Occurrence::incomingTransfersToSelf::target`
and `Occurrences::Occurrence::outgoingTransfersFromSelf::source`. Their unchanged
source contains `end feature redefines target = that;` and
`end feature redefines source = that;`, respectively. The record retains exact
archive/entry identities, byte ranges, current reference XMI identities, membership
order, and complete local typing/redefinition closures.

Let F be the containing feature, U its end with a nondefault value, V the opposite
end, E the value Expression and B the mandatory FeatureValue BindingConnector.
U and V are typed by `Occurrences::Occurrence`; F is typed by `Transfers::Transfer`.
U is featured by F. `checkExpressionTypeFeaturing` requires E's featuring types
to equal U's. `checkFeatureValueBindingConnector` independently requires B's
featuring types to equal U's. Both therefore require the exact set `{F}`.

Published `ownedCrossFeature()` selects the first Feature owned through a
non-FeatureMembership after its exclusions. Its OCL tests whether the member is
a FeatureValue, although FeatureValue is the membership and E is the member.
The literal selector therefore selects E. Even if that misplaced exclusion is
read as a membership exclusion, it selects B instead. Neither reading repairs
the domain contradiction:

| Reading | Selected member | Required value domain | Required binary cross domain |
| --- | --- | --- | --- |
| Literal published selector | E | `{F}` | `{Occurrences::Occurrence}` |
| Exclude FeatureValue memberships | B | `{F}` | `{Occurrences::Occurrence}` |

The literal cross-featuring OCL also says `excluding(self)` instead of excluding
the owning end. The proof evaluates that reading separately: both ends remain,
so it requires a Cartesian product whose component types equal the end types.
`asCartesianProduct(F)` necessarily includes F's type, `Transfers::Transfer`,
which is absent from the end-type set. That reading fails too. The proof does not
silently repair either formal defect to manufacture its conclusion.

The current pilot explicitly excludes both BindingConnectors and FeatureValue
memberships in `FeatureUtil.getOwnedCrossFeatureOf`. This changes canonical
cross-feature selection. It corroborates the issue but does not authorize an
Agentique correction. KERML11-1 is still open, last updated 2026-04-21 in the
retained official issue page. Preliminary revision text is retained separately.

This is not caused by the four unresolved references or incomplete corpus-wide
queries. The independent proof resolves every local reference, computes inherited
end typing and domain typing, and uses no Agentique producer/query helpers.
An arbitrary-name kernel fixture reproduces the incompatible domains in all
seven profiles with complete local query answers and an ordinarily valid
contextual FeatureValue binding.

KERML11-8 changes only the reviewed reference-result connector endpoint check;
it cannot change E/B cross-feature identity. KERML11-145 supplies contextual result
chains; it cannot change the exact featuring equality or published selector.
Adding an exclusion, replacing or reordering members to change selection, or
changing their ownership/domain requires another canonical semantic decision.
No such correction is adopted here. The ordinary implementation obligations remain
open; accepted publication, bindings and facade are not issued.
