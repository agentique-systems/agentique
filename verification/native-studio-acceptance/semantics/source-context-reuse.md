# Exact base-context reuse draft

`source-context-reuse.patch` is unapplied. It changes one production file and
adds a regression over authenticated runtime inputs. Static checks, source and
file identities are recorded in `source-context-reuse-patch.json`.

The current compilation independently initializes retained KerML and SysML
readers. Both initializations bind the same immutable graph, repeating full
provenance/profile checks, descriptor checks and aggregate/original-declaration
digests. The strict audit initializes SysML first; first later KerML access can
repeat that work during validation, semantic fingerprinting or repository receipt
authentication. This duplicate is separate from the repeated context builds
inside the producer scheduler.

The draft authenticates the original KerML input through its existing retained
cell, then composes SysML from a borrow of that exact input. Ordinary
`SysmlSemanticContext::for_closed_dependency` still checks the dependency
contract, bindings, producer registry and naming interpretation. Generic SysML
composition can enrich a base KerML context, so the draft deliberately does not
substitute SysML's inner context back into the original base identity. No graph,
certificate, query result or proof is rebound to a different revision.

Expected savings: one duplicate final-graph authentication per compilation that
uses both read flavors. The audit still runs all required queries. The 37.723 s
final closure and 47.949 s audit are not claimed as savings, and a new measured
run is required to quantify the actual reduction.

The new ignored regression uses the real authenticated KerML/Systems caches and
compares retained queries against the old independently constructed contexts.
It checks both initialization orders, exact context identities, all six closure
requirements, names, inherited features and five composed query families over
every local subject plus an absent subject. Entire composed answer Debug
representations include values, status and proof/search evidence. It then creates
an unresolved Working child and verifies the same equality and parent-reader
stability. No revision-ID normalization is used within a revision.

After coordinated application, compile ordinary text tests and run the focused
runtime regression with `AGENTIQUE_SOURCE_ROOT`, `AGENTIQUE_KERML_CACHE` and
`AGENTIQUE_SYSTEMS_CACHE` pointing at the exact accepted inputs:

```text
cargo test --locked --offline -p agq-kerml-text --lib retained_composed_reads_match_independent_binding_for_strict_and_working_revisions -- --ignored --nocapture --test-threads=1
```

The full real command/cold oracle remains required after integration. So far,
isolated `rustfmt --check` and `git apply --check` passed; no compile or runtime
test has run for this draft.
