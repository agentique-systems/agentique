# Agentique architectural acceptance model

These five SysML 2.0 documents model Agentique as a system. They are real inputs
to the Operational v2 frontend and canonical construction tests. They describe
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
an executable proof or a passing verification result. The execution, repository,
client and view components remain architectural plans.

`implementation-map.json` provides separately tested implementation traceability.
An empty path list denotes planned work. The language engine does not inspect
Cargo manifests, and this manifest does not generate implementation code.

The structural tests live in `crates/kerml-text/src/agentique_self_model_tests.rs`.
They distinguish current-graph acceptance from accepted Systems publication and
producer closure. The latter remains an explicit integration gate until the
accepted Systems dependency can be consumed by authored projects.
