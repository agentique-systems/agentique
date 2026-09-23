# In-memory Gen2 modeling workspace

`ProjectWorkspace` owns one Working head and immutable revision history over
authenticated accepted KerML and Systems publications. Integration remains gated
by the language foundation acceptance contract; implementation and test
compilation do not establish Systems publication or platform acceptance.

Document operations check an expected head, prepare exact source/syntax inputs,
compile the current document set, then publish one new revision atomically.
Stale heads, invalid edit spans, missing documents and construction invariant
failures publish nothing. Recovery, unresolved references and explicitly
unsupported source become Working revisions with native diagnostics. Their
queries borrow the current construction/overlay and preserve incomplete scope
evidence. They never fall back to an earlier revision's graph.

`WorkingProjectRevision::validate` returns a `ValidatedProjectRevision` for the
same immutable handle only after the Phase1V1 contract passes: complete parsed
inputs, a strict declared graph, no blocking diagnostics, Complete converged
producer closure, a fully closed certificate attached to the exact context and
complete mandatory references. Validation does not claim full language
conformance or execution support.

Both handles expose the existing KerML and SysML query contracts. Element/source
lookup and diagnostics are revision-bound; there is no second semantic DTO
model. Accepted standards remain shared, immutable dependencies. Authored
construction is rebuilt for each edit; future provider-footprint invalidation
can reduce recomputation without changing revision or acceptance contracts.

The `verification` feature records actual producer evaluations and inspects
physical kernel table ownership. It supplies no acceptance authority. See
[the integration test contract](tests/README.md) for accepted-cache tests,
including self-model edits and 100 mixed documents across five revisions with
parallel readers. No persistence, server adapter or Gen1 storage is included.
