# Authored sources over accepted publications

`SourceProject::with_accepted_sysml_standard_libraries` consumes an
`Arc<CanonicalSysmlSystemsLibrary>`. The facade retains its accepted KerML
publication. This additive language-consumer entry point selects Operational v2
explicitly; existing source constructors retain their earlier behavior.

Each successful source operation records immutable document syntax revisions,
the canonical declared snapshot, a separate derived semantic overlay, mandatory
reference outcomes, the combined producer status, and its exact closure
certificate. `ProjectRevision::queries()` reads the composed KerML model;
`sysml_queries()` exposes the effective SysML API over the same canonical IDs.
`snapshot()` remains declared, while `semantic_model()` includes derived facts.
Neither facade copies inherited members into authored declarations.
Unchanged document objects and their lossless syntax arenas are shared by `Arc`
across source revisions; editing one document does not clone all other arenas.

Both publications remain protected shared dependencies. The independently
validated mounted certificate preserves the precise accepted KerML ancestor and
the Systems graph's closed producer evidence. Library roots cannot see newly
authored roots. Accepted bindings remain bound to their original immutable graph.

Construction refinement and final strict reconstruction revalidate previous
producer reads through closure checkpoints. The status exposes retained and
reopened evaluation counts. The scheduler still considers local authored
subjects after edits; this is not an incremental-work performance guarantee.
Standard subjects are excluded from local producer scheduling. Query-context
fingerprinting and local reconstruction remain candidates for future incremental
work.

This frontend constructor still requires syntactically complete, structurally
constructible source. It does not introduce a Working/Validated workspace or a
source-recovery contract. Producer incompleteness and diagnostics remain visible;
effective query results retain their own pending capability findings. A complete
source slice does not mean full SysML conformance or executable semantics.

The constructor alone establishes no acceptance of the Agentique model. The
separate exact-cache acceptance fixture must parse, close, and query that model
against an actually accepted Systems publication before the readiness gate passes.
