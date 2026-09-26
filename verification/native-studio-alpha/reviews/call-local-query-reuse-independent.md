# Independent source review: call-local query reuse

Reviewed exact commit `20840905a0a1554b1106357b2cb6230f77ac279e` without compiling,
opening a runtime, or running another standards consumer. The reviewer did not
author this optimization. This review permits integration for qualification; it
does not establish measured speedup or accepted-model equivalence.

## Findings ranked by impact

No source-level production correctness blocker was found.

1. **P2, execution qualification outstanding.** Run the ignored accepted-runtime
   oracle before claiming equivalence or performance improvement. The committed
   baseline timings describe the old path, not the optimized result.
2. **P2, explicit coverage limit.** Actual Working-revision SysML-factory failure
   parity is not exercised by the accepted Validated baseline. The generic
   dispatch test proves fallback/error selection; the incomplete raw-query test
   proves a query-evidence boundary. Neither establishes that complete production
   fallback path. The handoff document correctly discloses this, and source
   inspection found no changed error priority or authority inference.

## Production path

Projection construction stays lazy. A call which needs connector endpoints
creates the ordinary checked KerML evaluator at the same point as before. A
focused interface query reuses that evaluator, or creates the same ordinary
evaluator if no connector query needed one. Semantic query order and warning
collection remain unchanged. The evaluator cannot escape the call or cross a
revision.

Inspector now first obtains the ordinary checked SysML evaluator and borrows its
contained KerML evaluator. This is not a weaker context factory. I independently
traced `ProjectRevision` through `CompiledSource` factories, the strict effective
source and construction paths, `ProducerClosedDependency::attach`, and
`SysmlSemanticContext::for_closed_dependency`. The mounted context already retains
its exact producer registry, semantic naming extension, dependency witness and
closure attachment. Reattaching the same registry and naming identities is
explicitly idempotent; mismatches remain errors. The SysML wrapper validates its
dependency contract and bindings.

If SysML creation fails, the new dispatch calls the ordinary KerML factory and
keeps the baseline KerML profile label. If that factory also fails, its exact
existing error remains the returned error. Early missing-element/model errors
still happen before factory construction. No query values, completeness,
diagnostics or provenance fields are replaced with guessed values.

## Baseline and oracle fidelity

An independent read-only Python comparison through `git show` checked the
preserved `project_observed` and `inspect_observed` bodies against
`d404a8759891acc5d621dcad655fa4a981e17d63`. Both match exactly after CRLF/LF
normalization: 7,524 and 5,398 characters respectively. The projection helper
suffix also remains exactly unchanged. The source command exited 0. No generated
test result or semantic acceptance was inferred from that comparison.

The ignored oracle compares 17 projection cases and eight Inspector cases on one
ordinary restored Validated revision. It checks full DTO equality, serialized
bytes, error variant/debug payload and Display text. Raw parity checks include
context, values, completeness, diagnostics, positive/search/canonical
dependencies, origins and explanations; complete Debug comparison covers private
producer/shared-search evidence. It uses no identity or context normalization.

Projection cases cover focused architecture with and without connector work,
standard visibility, hidden elements, both graph scopes at depths 0/1/2,
Requirements, missing focus and unsupported version. Inspector cases cover
system, platform, repository, inherited port, connected port, connector,
requirement and missing element. Raw projection checks compare a fresh focused
evaluator with a connector-primed shared evaluator. All remain conditional on
the actual oracle completing successfully.

The fixture-only incomplete query checks explicitly retain pending-inheritance
diagnostics and Invalid results; they are not accepted runtime evidence.
Alternating full-call timing pairs and rotated fresh/fork/shared trials separate
construction from query work and disable profiling for both old and new calls.
Their three observations are not p95, disk-cold, UI queue, GPU or photon latency.

The oracle copies an explicitly closed SQLite backup and rejects a nonempty WAL
or rollback journal. It authenticates normal cache facades and resolves the exact
requested project/revision through the service, requires Validated restoration,
and checks its semantic fingerprint before/after. It neither seeds nor commits
a model. The preserved backup identity and actual process evidence should remain
with the eventual qualification result.
