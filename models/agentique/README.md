# Agentique architectural acceptance model

These five SysML 2.0 documents model Agentique as a system. They are real inputs
to the accepted Operational v3 frontend and canonical construction tests. They describe
conceptual contracts, not a one-to-one mapping of Rust types.

`Agentique.sysml` composes the language engine, modeling platform and planned
execution subsystem. `Contracts.sysml` describes query, revision, diagnostic,
publication and execution interfaces. The remaining documents describe those
subsystems. Non-composite `ref part` usages represent directed architectural
dependencies; composite parts represent contained components. The kernel has no
reference dependency on either language engine. Views reference the query service,
and the execution compiler consumes a validated semantic-state abstraction.

The platform document is also the rich authored regression: items, attributes,
ports, interface ends and connections, inherited workspace structure,
redefinition, action parameters, state entry/do/exit, a requirement subject and
an unimplemented constraint. `preservesPriorState` is a modeled obligation, not
an executable proof or a passing verification result. ModelRepository,
ModelingService and SystemsModelingApiAdapter model the Phase 2 implementation
boundaries. Execution, client and view components remain architectural plans.

The durable self-model gate in `crates/modeling-service/tests/durable_platform.rs`
commits these exact five documents, drops the service/workspace state, restores
their canonical identities, and edits the architecture-experiment branch while
main remains unchanged. Its actual outcome is recorded in the Phase 2 summary.

`implementation-map.json` provides separately tested implementation traceability.
An empty path list denotes planned work. The language engine does not inspect
Cargo manifests, and this manifest does not generate implementation code.

The structural tests live in `crates/kerml-text/src/agentique_self_model_tests.rs`.
They distinguish current-graph acceptance from accepted Systems publication and
producer closure. The latter remains an explicit integration gate until the
accepted Systems dependency can be consumed by authored projects.

The prepared accepted-publication gate is
`crates/kerml-text/src/accepted_self_model_tests.rs`. It uses the additive accepted
`SourceProject` constructor and requires exact trusted caches supplied explicitly:

```powershell
$env:AGENTIQUE_KERML_CACHE = '<accepted KerML cache path>'
$env:AGENTIQUE_SYSTEMS_CACHE = '<accepted Systems cache path>'
cargo test -p agq-kerml-text --lib accepted_agentique_self_model_closes_queries_edits_and_matches_programmatic_semantics -- --ignored --nocapture
```

Requesting this gate with absent or unaccepted cache files fails. It never
rebuilds either standard publication. A passing run must establish combined
producer closure, all mandatory references, effective architecture queries,
independent programmatic equivalence, shared standard and syntax objects, and
three immutable edit revisions with parallel readers. The test remains explicitly
ignored in routine unit tests until those accepted artifacts are available.

The same gate loads the small `crates/kerml-text/tests/fixtures/agentique-cases.sysml`
fixture to require Complete case/verification-case subjects, actors, objectives
and result parameters. Explicit redefinitions preserve the accepted standard
defaults' identities. This remains structural inspection; no case is executed and
no modeled requirement is declared verified.
