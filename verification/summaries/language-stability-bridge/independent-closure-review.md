# Independent closure integration review

Reviewed `2aa9fb894f49442ce64c39a7f97cf60cac2a60b1` in an isolated worktree before
the rebuilt Actions gate. Review scope is initial/checkpoint closure, source
provider completeness, immutable dependency exclusions, changed applicability
and future writer activation. No corpus run was started by this review.

Final disposition: **go for the bounded Actions audit** after independently
testing production fix `9ad3d3aef59ef0bee0c86999566f8804d256e281`.
All 52 local `producer_closure_tests` passed (exit 0, 11.79 s). The closure owner's
separate output-disappearance regression is outside this worktree's population.

Initial disposition was **no-go pending source-provider correction**. Exact graph
identity and a complete producer registry are insufficient when source linking
still has a required endpoint whose subject is unknown.

The minimal counterexample has Feature 1, Classifier 2 and FeatureTyping 3.
Relationship 3 has `type = 2`, but `typedFeature` is an outstanding construction
obligation. With no applicable producer writers, the initial certificate returns
Complete for Feature 1's EffectiveTyping, and its current `feature_types` query
returns Complete empty. Source refinement can subsequently link relationship 3
to Feature 1. The unknown provider must keep the population open independently
of whether producers have work.

Permanent regressions cover:

- unresolved FeatureTyping source (`FEATURE_TYPING_TYPED_FEATURE`);
- unresolved Redefinition/Subsetting source, owned FeatureChaining target, and
  owned Membership member endpoint;
- a composite Step with an absent owner while a Behavior's explicitly pending
  namespace population contains an unfinished FeatureMembership. That endpoint
  is derived, so source pending-scope evidence is required even when no stored-slot
  construction obligation exists.

All three `initial_closure_` tests failed against the reviewed commit (exit 101).
An earlier exploratory ownership fixture failed its setup assumption: omitted
FeatureMembership owned-related values create no required stored-slot obligation.
The final ownership regression uses the real pending namespace contract instead.
A read-only carrier probe separately confirmed the four relationship cases; its
temporary diagnostic test was replaced by permanent assertions.

The remainder of the review found no separate blocker: rebinding authenticates
the unchanged registry and interpretation, fingerprints old/new records and
incoming carriers, reopens missing metadata and changed reads, and recomputes
causal/future-writer masks. Detached derived outputs contribute fingerprints to
their canonical support. Accepted dependency exemptions preserve model-scoped
and arbitrary incoming-source writer blockers; record immutability alone is not
treated as proof of a closed inverse population.

The initial provider fix `4390c42225ca0b87fcb5a6cb7ac9b4b70f035e92` corrected
those direct cases. Independent testing found two related transport gaps:

- An existing completed producer on Classifier 3 reads Feature 1's closed
  EffectiveTyping. Adding a FeatureTyping with an unknown source reopens Feature
  1 but must also reopen the retained Classifier 3 producer and its effects.
- A pending namespace can adopt an existing FeatureMembership that already owns
  a composite Step. The Step's immediate membership is fixed, but its owning
  Type is not; the owner-negative formal predicate must remain Incomplete.

Both are permanent regressions. The final fix seeds causal producer dependencies
from pending source-provider masks and propagates pending ownership through
existing canonical ownership carriers. The independent closure subset passed
after these changes. This closes the bounded A1/A3/A5 review; actual Actions and
publication gates remain the next discriminator. This review does not establish
Systems publication or foundation readiness.

`independent-review.json` records exact commands, tool versions, source/patch
identities, actual exits and output digests. Raw output is ignored under
`verification/generated/language-stability-bridge/`, following ADR 0021. Cargo
uses debug symbols disabled, incremental compilation disabled, two build jobs
and isolated `target/bridge-architecture`; no cache cleanup was needed.
