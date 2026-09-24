# Authored audit evaluator batches

The authored effective audit keeps its 32-subject batches and exact publication
dispatcher. It authenticates the final immutable graph once, then uses fresh
`SysmlQueries::fork()` evaluators for the batches. The existing exact context
equality assertion remains in place. The bound evaluator only enumerates subjects;
it does not retain the batches' query caches.

The new additive query method borrows the same `ModelView`, clones the same SysML
context and bindings, and delegates to the existing `KerMlQueries::fork()`. That
method forks the authenticated `SemanticContext` and creates empty evaluator
caches. It retains producer certificates, accepted dependencies, pending scopes,
profile identity, and all graph fingerprints. No producer fact, rule version,
profile interpretation, finalizer operation, or audit subject changes.

## Static cost finding

Previously every batch called `SourceCompilation::sysml_queries()` anew. The
strict path calls `EffectiveSourceModel::context`, then
`ProducerClosedDependency::project_overlay_context`. `SemanticContext::bind`
checks descriptors and origins and hashes the combined accepted/local graph;
attaching the registry additionally computes `producer_model_digest`. For local
population L this repeated whole-graph work approximately `ceil(L / 32) + 1`
times. Forking removes that repetition, without caching an answer across graph
changes. This is code inspection, not a measured speedup.

Remaining fixed work includes accepted dependency mounting, authored lowering and
reference refinement, producer closure for genuinely changing frontiers, final
reference queries, and one final authored audit context. Public revision query
factory calls still authenticate their graph independently. The scale fixture
currently calls that factory inside its reader/iteration/revision loops; this
pre-existing cost is outside this change. Standard canonical storage remains
shared, and the audit only evaluates local IDs (including derived locals).

## Verification

The focused fixture warms an original evaluator, then checks forked effective
typing and naming for equal values, completeness, pending state, diagnostics,
positive/search dependencies, explanations, canonical dependencies and origins.
It verifies the same borrowed graph and exact SysML/KerML contexts, preserved
standard bindings, independent evaluator counters, and continued querying after
the original evaluator is dropped. An uncertified query remains Incomplete with
the same pending evidence. The synthetic accepted dependency helper is pinned
to Operational v2; the test preserves that identity.

Commands run with one build job, no incremental compilation or debug data, using
the isolated existing `target/authored-effective-audit` target. Raw command logs
are retained in ignored `verification/generated/authored-effective-audit/`.

- Initial focused test: exit 101. The fixture attempted to attach Operational v3
  to the v2 synthetic dependency and correctly received
  `SemanticExtensionIdentityMismatch`. Only the fixture profile was corrected.
- `cargo test --locked --offline -p agq-sysml-semantics query_fork_preserves_closed_and_pending_answers_with_independent_evaluators -- --nocapture`:
  exit 0, 1 passed, 101 filtered out; runtime 3.03 seconds after 42.72 seconds
  compilation.
- `cargo clippy --locked --offline -p agq-sysml-semantics -p agq-kerml-text -p agq-modeling-workspace --all-targets --features verification -- -D warnings`:
  exit 0, all three packages checked; 12.19 seconds.
- `cargo fmt --all -- --check`: exit 0.
- `git diff --check`: exit 0.

No full accepted-cache or platform scale runtime is claimed by this record.
