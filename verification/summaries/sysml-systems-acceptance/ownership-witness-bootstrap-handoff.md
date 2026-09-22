# Ownership witness bootstrap: read-only handoff

Inspected integrated source `a8359d8` and the existing Actions slice log while
that run was active. No new candidate was run and no production code was changed.
An isolated parse-only locator scan over the exact verified KerML/SysML sources
mapped these subjects; its temporary diagnostic-tool change was restored.

| Diagnostic subject | Exact pinned source |
| --- | --- |
| `128675031989172710339588986668643513292` / `60cde8e3-6ddc-504c-8152-e09f771bf3cc` | `Kernel Semantic Library/Transfers.kerml`, bytes 10594..10815: package-level `abstract step acceptPerformances: AcceptPerformance[0..*] nonunique subsets performances` |
| `136825237176413910705835738885986810070` / `66ef9507-4c0b-5a78-93db-e907945d78d6` | `Kernel Semantic Library/Performances.kerml`, bytes 5663..5837: package-level `abstract step performances: Performance[0..*] nonunique subsets occurrences` |

These are accepted KerML dependency Steps. Their package ownership does not
provide a FeatureMembership owning Type. The formal
`StepEnclosedPerformanceSpecialization` antecedent has no scalar flag guard;
`formal_targets.rs` now correctly requests an `EffectiveOwnership` witness
before concluding that the absent owning Type makes this antecedent false.

The existing log associates both diagnostics with reference
`274370872562044140583764357053143282556`
(`ce69e86e-5444-5daa-9b9b-9e139a67737c`):
`Systems Library/Actions.sysml`, bytes 9355..9363, the `receiver` chaining
relationship in `bind receiver = accepter.receiver;`. The recorded provisional
candidate changed from `Transfers::AcceptPerformance::receiver`
(`04b776b6-a02e-5e0c-8240-0f013b5999e2`) to the redefined receiver inside
`TransitionPerformances::TransitionPerformance::accept`
(`5547e6fa-9148-5520-baf4-40c856495620`, membership
`bd1ea854-58ff-5d47-8bc7-875eec48f0ce`). This observation identifies a query path,
not an acceptance failure or proof that ownership is its only remaining blocker.

`lookup_relationship_target` resolves a later feature-chain segment from its
previous endpoint. Inherited lookup and implied-base redundancy traversal call
`direct_specializations`, `library_specializations`, `required_library_bases`,
and the formal predicates on these dependency Steps. Their closure diagnostics
therefore propagate into the chained receiver lookup.

The lifecycle is explicit in `sysml.rs`: the initial reference-only refinement
constructs fresh declared drafts with no certificate. Once that reaches a fixed
point, each producer-assisted refinement round reconstructs declared records,
runs combined producer closure, attaches that exact overlay's certificate, and
then resolves references. A changed endpoint population causes another declared
reconstruction and another producer closure. `LibraryDraft::set_semantic_candidate`
clears the prior certificate before the new one is attached. This is safe, but
can amplify the cost of one-endpoint-at-a-time progress.

Possible follow-up, not implemented: a scheduler-issued initial partial witness
bound to the exact new graph, context, and independently assembled combined
registry could close requirements for which no registered producer can write.
Every genuinely applicable writer must remain Pending until evaluated. This must
not reuse an old graph certificate, treat quiescence as evidence, or weaken the
negative formal predicate. The closure agent was reviewing this opportunity.

A focused regression should retain package-owned Step generals and a local
`accepter.receiver` chain, checking uncertified incompleteness, exact fresh
no-writer ownership certification, rejection after a graph or registry change,
and blocking when a synthetic Ownership writer is added. The existing delayed
local ownership test alone does not exercise this accepted-dependency lookup.

Work stopped at the user's wrap-up request. Systems publication remains
unaccepted; this handoff establishes no effective language or platform readiness.
