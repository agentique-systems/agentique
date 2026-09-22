# G/H structural readiness review

Scope: exposed `agq-sysml-query/5` structural APIs and the permanent authored
fixtures. This review does not assert Systems publication or language readiness.
Source baseline: root semantic correction `3942e68`, layered scheduler fixture
`857b12e`, and the previously prepared authored integration/harness.

Evidence terms: **canonical** means actual kernel queries with known values and
honest `Incomplete` without a certificate; **closed** means a real combined
scheduler issued the compatible certificate; **held** means the accepted-cache
harness compiles but has not run against an accepted Systems receipt.

| Requirement | Concrete API | Evidence and remaining acceptance |
| --- | --- | --- |
| G1 owned/effective Usage; nested Usage | `owned_usages`, `effective_usages`, `nested_usages`, `effective_nested_usages` | Canonical `all_usage_families_*` and new `nested_usage_populations_*` retain inherited IDs and suppress redefined originals. Closed Actions micro tests establish effective Usage completeness. Held rich workspace asserts complete inherited populations. |
| G1 definition/type, specialization, subsetting, redefinition | `direct_usage_types`, `effective_usage_types`, `direct_specializations`, `effective_supertypes`, `effective_subsetted_features`, `effective_redefined_features` | Canonical type-domain, inherited typing and nested redefinition tests. Held authored/programmatic fixture compares standard bases, effective types and redefinition. |
| G1 qualified name | `current_qualified_name`, `effective_qualified_name` | Canonical naming/revision tests retain negative searches and old answers. Closed Actions fixture checks message naming. Held authored fixture checks `PlatformArchitecture::IncrementalWorkspace::acceptedRevision`. |
| G2 Attribute/Item/Part definitions and children | `effective_attribute_definitions`, `effective_item_definitions`, `effective_part_definitions`; `owned_usages_of_kind`, `effective_usages_of_kind` | Canonical projection tests distinguish DataType/Structure/PartDefinition domains and invalid targets. Held fixture compares these projections after combined closure. |
| G2 subitems/subparts/composite nesting | `effective_subitems`, `effective_subparts`, `effective_composite_usages` | New nested test distinguishes composite Item/Part descendants from reference parts through typed Usage inheritance; existing mutation test preserves old revisions. Accepted-positive nesting remains part of held authored validation. |
| G3 ports, definitions, ends, interfaces, ConnectorAsUsage | `effective_ports`, `effective_port_definitions`, `effective_connection_ends`, `effective_interface_ends`, `effective_connection_related_features` | Canonical ordered ends and typed-domain tests; new BindingConnectorAsUsage/SuccessionAsUsage test guards against narrowing to ConnectionUsage. Held authored fixture checks real interface PortUsage ends, endpoint order and effective ports. |
| G4 parameters/subactions | `effective_parameters`, `effective_return_parameters`, `effective_subactions` | Canonical parameter/result separation and nested composite subactions; closed Actions micro exercises parameter-dependent payload structure. Held authored fixture has typed in/out action parameters. |
| G4 entry/do/exit and transition actions | `state_actions`, `transition_features` | Canonical membership-kind evidence and domain checks; closed Actions micro checks transition trigger. State/transition APIs deliberately return plural **owned** membership-role projections; they do not choose an arbitrary singular result or execute it. |
| G4 AcceptAction accepter/message | `transition_features(Trigger)` composed with `accept_action_payload_parameter` | Closed real-scheduler Actions micro retains the accepter and payload's original IDs. The API exposes structural parameter identity, not a runtime message value. |
| G5 requirements/constraints/cases/verification cases | `effective_usages_of_kind` with `UsageKind::{Requirement, Constraint, Case, VerificationCase}` | Canonical all-family selector; new verification-case inheritance test checks subtype selection. Held rich authored model includes a requirement and constraint. Separate held `accepted_case_roles` checks actual Case/VerificationCase standard bases and populations. |
| G5 subjects/actors/objectives/results | `requirement_case_features(Subject/Actor/Objective)`, `effective_return_parameters` | Canonical role tests and new inherited verification-case test preserve original actor/objective/result identities while replacing the subject by redefinition. Held `accepted_case_roles` requires complete roles/results on both authored definitions and their typed usages, with explicit redefinition of the accepted library defaults. |

Canonical tests are in [structural_query_tests.rs](../../../crates/sysml-semantics/src/structural_query_tests.rs),
[projection_tests.rs](../../../crates/sysml-semantics/src/projection_tests.rs),
[qualified_names_tests.rs](../../../crates/sysml-semantics/src/qualified_names_tests.rs),
and [transition_tests.rs](../../../crates/sysml-semantics/src/transition_tests.rs).
Closed evidence is in [producer_tests.rs](../../../crates/sysml-semantics/src/producer_tests.rs).

| H requirement | Permanent evidence | Acceptance state |
| --- | --- | --- |
| Real parts/items/attributes/ports/connections/actions/states/requirements/constraints | Five [Agentique SysML documents](../../../models/agentique/README.md), real Operational v2 frontend and canonical lowering | Current-graph tests pass; combined accepted gate held. The empty constraint is a modeled obligation, not verified behavior. |
| Standard bases and effective typing | `assert_revision` in [accepted_self_model_tests.rs](../../../crates/kerml-text/src/accepted_self_model_tests.rs) checks accepted standard IDs and producer closure | Held; cannot infer acceptance from canonical fixture results. |
| Inheritance/redefinition, ports/endpoints, qualified names, requirements | Held harness `rich_summary`; ordinary self-model semantic tests already check source reference endpoints and zero kernel construction obligations | Current graph passes; accepted Complete assertions prepared. |
| Stable IDs/no inherited copies | Ordinary deterministic construction and source-edit tests; held three-revision fixture checks original inherited member identity and retained authored IDs | Ordinary tests pass; accepted edits held. |
| Independent programmatic equivalence | [Direct ChangeSet fixture](../../../crates/kerml-text/src/sysml_programmatic_vertical_tests.rs), independent IDs; held `programmatic_equivalence` closes that graph over the same accepted standards | Current-graph equivalence passes; effective equivalence held. |
| Shared standards/revisions/parallel reads | Authenticated dependency witness; held graph and syntax pointer checks and parallel queries | Witness tests pass; accepted rich history held. |

No additional producer or language-conformance phase is proposed. Remaining
readiness evidence is concrete: accept Systems, restore its exact receipt, run the
held rich/self-model gate and the small G5 positive fixture, and fix any actual
failure. Unsupported variation/individual/portion capabilities remain visible;
the real closed variation regression does not become Complete through quiescence.

Focused validation of this review's three additional tests (same low-disk profile
as adjacent summaries): `cargo test -p agq-sysml-semantics structural_query_tests`
exited 0 with 7 passing tests. They modify no producer machinery or public APIs.

The separately held [G5 source fixture](../../../crates/kerml-text/tests/fixtures/agentique-cases.sysml)
also passes `cargo test -p agq-kerml-text --lib case_acceptance_fixture` (exit 0,
one test). This is actual Operational v2 parse/construction evidence for case,
verification, actor, objective and result memberships, deliberately retaining
unresolved standard paths when no standards are supplied. It does not discharge
the accepted positive gate. The held rich equivalence summary also now covers
effective nested usages and composite subitem/subpart populations.
