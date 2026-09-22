# Independent declared-population evidence review

Scope: `3942e68614862a65300c8e15f8b2626176ddbf0a`, limited to original
declaration populations, mixed current/source evidence, checkpoint reconstruction
and dependent archive restoration. No publication command was run.

Final disposition: **go for the corrected bounded Actions gate** after independently
testing correction `7120fb7` (local cherry-pick `eadad43`). All 59 closure tests,
the opt-in/idempotent producer identity test and the exact SysML anchor provenance
test passed. This is a scoped soundness review, not Systems publication acceptance.

The correction adds original slot values/origins to the opt-in producer model
digest and checkpoint subject fingerprints. Historical contexts without a
producer registry retain their existing digest. Current property roots now
survive evidence merging and persist as current `Property` searches when they
overlap a declared-population search. Both ordinary and producer evaluators are
covered. The three independent counterexamples now pass, including archive
restoration; no archive format change was needed.

Initial disposition: **no-go until two reproduced boundary defects are fixed**.

1. `producer_closure_rebind.rs::subject_signatures` and the existing aggregate
   model digest do not distinguish original slot `[a]` plus derived `[b,c]` from
   original `[a,b]` plus derived `[c]`, when aggregate values and explanations
   match. `declared_owned_relationships` correctly returns different values, but
   direct attachment accepts the old certificate and rebinding retains the
   completed reader with no affected subjects. The permanent regression checks
   both authentication and reopening.
2. `producer_closure.rs::producer_reads` narrows a property read when it sees
   `DeclaredProperty`. A merged direct current `fact(Property)` read on a still
   wholly declared slot is not marked as current, so the first derived append
   can escape its dependency. The existing mixed-read test covered a slot that
   was already derived. The new regression checks the missing case in both
   merge orders and requires the current read to survive persisted evidence.

The dependent archive regression confirms that `write_dependent_overlay` and
`read_dependent_overlay` already preserve the original slot separately from its
aggregate, retain the supplied dependency allocation, and reproduce the same
bytes. Original and restored producer contexts must match individually; the two
different original populations must have different producer graph identities.
The initial archive failure is solely that last identity assertion.

The remaining inspected path is consistent with the intended boundary:
`Evaluator::path` selects memberships from original namespace ownership, checks
standard-library element/name/membership provenance, public visibility and exact
metaclass, and retains current member-query evidence. An adopted carrier is
excluded; declaring it in the original population exposes ambiguity. Pending
namespace populations and unresolved member endpoints remain incomplete.

`declared-evidence-review.json` records exact commands, source/patch identities,
exit codes, tool version and output digests under ADR 0021. The first recorded
command failed to compile because the test used an ambiguous `Explanation`
import; the qualified import corrected that fixture error before semantic
counterexamples were recorded. Raw logs remain ignored generated artifacts.
