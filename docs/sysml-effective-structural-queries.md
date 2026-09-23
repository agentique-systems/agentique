# Effective SysML structural queries

`agq-sysml-semantics` query contract `agq-sysml-query/5` composes KerML queries
over the same canonical graph. Every result carries its context, completeness,
canonical fact observations and supporting query evidence. Returned `ElementId`s
are the original declarations or derived elements; inheritance never copies them.

| Platform need | SysmlQueries API |
| --- | --- |
| Definition/Usage members | `owned_usages`, `effective_usages`, `nested_usages`, `effective_nested_usages` |
| Attribute, Item, Part, Port, Connection, Interface, Occurrence, Action, State, Requirement, Constraint, Case, VerificationCase populations | `owned_usages_of_kind`, `effective_usages_of_kind`, `UsageKind` |
| Composite children | `effective_composite_usages`, `effective_subitems`, `effective_subparts`, `effective_subactions` |
| Typing | `effective_usage_types`, `effective_attribute_definitions`, `effective_item_definitions`, `effective_part_definitions`, `effective_port_definitions` |
| Relationships | `effective_supertypes`, `effective_subsetted_features`, `effective_redefined_features` |
| Ports and connections | `effective_ports`, `effective_connection_ends`, `effective_interface_ends`, `effective_connection_related_features` |
| Actions and states | `effective_parameters`, `effective_return_parameters`, `state_actions`, `transition_features`, `accept_action_payload_parameter` |
| Requirement/case roles | `requirement_case_features` with Subject, Actor or Objective |
| Names | `effective_names`, `effective_qualified_name` |

The accepter, payload and accepted message are distinct structural identities.
`transition_features(Trigger)` returns the AcceptActionUsage serving as a
transition's accepter; `accept_action_payload_parameter` projects its first
effective parameter, matching `AcceptActionUsage::payloadParameter`. The pinned
[Actions library](<../standards/libraries/Systems-Library/Systems Library/Actions.sysml>)
separately declares the undirected `AcceptMessageAction::acceptedMessage` feature
and binds `TransitionAction::acceptedMessage` to `accepter.acceptedMessage`.
Find that feature through the closed `effective_usages` population and the shared
KerML member lookup (`queries.kerml().lookup_path`). It is not the payload parameter
or a new AcceptActionUsage metamodel property. The permanent regression keeps
these IDs separate; the accepted-library harness must prove the actual library
path after publication. Payload/receiver argument expression derivations and
runtime message values are not claimed by these structural projections.

Effective answers require scheduler-issued closure evidence for the exact
semantic context and relevant subjects, including ancestors and returned usages.
An empty result also retains its closure search. The attached certificate must
match the combined KerML/SysML producer registry. A Boolean label or quiescent
scheduler alone is not evidence. Missing closure produces `Incomplete` while
retaining any known canonical identities for diagnostics.

`current_*` queries explicitly answer over the current graph. Direct/owned queries
also retain this existing meaning. Contract /4 adds `current_names` and
`current_qualified_name`, and requires closure on the effective naming variants;
this is a versioned semantic distinction, not an optimization.

These APIs select structural populations. They do not execute actions, verify
requirements, or claim all formal validators are implemented. Variation,
individual, portion and unresolved published-authority semantics remain explicit
pending capabilities even when implemented producers close. State and transition
queries return plural, ordered membership-role projections. The final SysML 2.0
inventory retains anomalies in several singular state/transition/objective
derivations; these APIs do not silently adopt corrections to those formulas.

The source authority is the pinned final SysML 2.0 descriptor graph and the
retained bodies in [the semantic inventory](../standards/sysml-semantic-coverage.json).
Focused tests cover all 13 usage families, ordered connector/interface ends,
typing domains, membership roles, inherited identity, negative closure evidence
and immutable revision changes. Actual combined-scheduler Actions fixtures prove
that supported effective answers become Complete with a compatible certificate,
while an unsupported variation remains Incomplete. Accepted Systems and authored
self-model integration remain separate readiness gates.
