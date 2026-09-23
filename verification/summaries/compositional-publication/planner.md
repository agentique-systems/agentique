# Provisional semantic dependency planner

`PublicationDependencyPlan` extracts canonical reference carriers, supplied
source/import/mandatory-reference/standard-target edges, observed query reads,
existing producer write scopes and potential future-subject writers. It reuses
the existing `ProducerDescriptor`, scope selectors, future ownership bound and
pending-provider masks. It does not introduce a producer registry or issue a
closure certificate.

The graph retains the existing global non-typing requirement guards and treats
unknown producer reads, live inverse searches and unbounded writes conservatively.
Generic source-reference reads cannot authenticate complete producer-family read
coverage. Therefore `MissingProviderEvidence` remains on applicable subjects even
when the caller supplies such query reads. A collapsed SCC in this provisional
analysis does not establish that the final graph intrinsically has one semantic
component. Narrower sealing still requires scheduler-issued evidence and the
equivalence gates in ADR 0027.

Global dependencies use virtual hubs, avoiding a subjects-by-writers adjacency
matrix. Iterative SCC traversal and a stable provider-first topological order
preserve deterministic partitions without recursion. The plan digest streams a
versioned tagged encoding of graph/context/registry identities, components,
dependencies, potential writers and findings; it never hashes pointer identity
or Debug output. Proposed orders that split an SCC or put a provider later are
rejected, without claiming that an otherwise consistent order is certified.

Focused verification: nine tests pass, covering linear/diamond/cyclic graphs,
32 input permutations, canonical typing/subsetting carriers, a forbidden later
writer split, negative/unknown reads, future writer activation, missing subjects
and a 12,000-subject chain. An independent review added an authenticated-provider
regression: a physically immutable overlay and caller-supplied digest retain
missing/unknown provider findings; an actual `ProducerClosedDependency` mount
passes. The planner now reuses certificate issuance's exact
`dependency_closure_source` boundary. The first run compiled successfully but failed one
malformed test fixture missing FeatureTyping::typedFeature; the required endpoint
was supplied and all eight then passed. This was a fixture correction, not an
expected negative acceptance result.

Package library/test Clippy with warnings denied and workspace formatting pass.
Actual commands, exit codes, source identities and log locations are in
`planner-commands.json` and `planner-format-commands.json`; raw output is in the
ignored generated directory. Checks used one build job, disabled debug symbols
and incremental artifacts, and the isolated `target/comp-plan` target.

No corpus producer execution, compositional fixed-point equivalence, component
sealing, accepted Systems publication or platform readiness is claimed here.
