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
| G4 AcceptAction accepter, payload and accepted message | `transition_features(Trigger)`, `accept_action_payload_parameter`, `effective_usages` + `kerml().lookup_path` | Closed real-scheduler micro proves accepter/payload identities, but its synthetic parameter name does not prove actual library message semantics. The new canonical distinction test keeps the first payload parameter separate from the undirected `acceptedMessage` feature. Held `accepted_trigger_and_message` checks the real `Actions::AcceptAction::aState::aTransition`, its accepter's standard base, and the distinct complete payload/message identities. No runtime value or argument-expression semantics is claimed. |
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

## Follow-up review at the authored acceptance boundary

Source `3241748f2b8fc711547120a2860ee4c4279394d1`, based on root `aef4cd0`.
All requested G1–G5 structural families have concrete query methods or explicit
composition through the borrowed KerML facade. No additional public method or
producer behavior was needed. Owned state/transition roles remain the documented
plural structural projection; this does not adopt unresolved singular formulas
or add execution semantics.

The accepted harness now directly checks effective Usage typing, PortDefinition
typing, subsetting, composite subactions and constraint populations in both text
and independent programmatic models. Connection and interface end queries must
return the same two canonical PortUsage end IDs, each with its own ordered
ReferenceSubsetting target. Case/verification subjects, actors, objectives and
return parameters retain their existing positive assertions, supplemented by
Requirement population selection. The edit fixture now proves that r3 suppresses
the original inherited port and r2's complete port population remains unchanged.
The dependency assertion borrows the accepted overlay directly instead of
materializing a throwaway project snapshot just to inspect its Arc identity.

The G4 correction is substantive terminology/coverage, not a new API:
`payloadParameter` is not the library's `acceptedMessage` feature. The pinned
Actions source and metamodel expose these distinct concepts; the new canonical
regression proves the distinction with honest Incomplete effective answers
without a certificate. The held actual-library assertion separately requires
Complete trigger, payload, effective member and qualified member-lookup results.
The old synthetic Actions micro's parameter named `acceptedMessage` remains
useful for generic lookup/payload tests, but is not evidence of actual accepted
message-role semantics.

Checks used `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2` and the isolated
`../agentique/target/bridge-self-model` target. No cache load or corpus run occurred.
Combined output is ignored under
`verification/generated/agentique-self-model/effective-api-review/` where retained.

| Exact command | Exit / result | Output SHA-256 |
| --- | --- | --- |
| `cargo test -p agq-sysml-semantics --lib structural_query_tests` | 0; 8 passed | Unavailable; tool output only |
| `cargo test -p agq-kerml-text --lib` | 0; 43 passed, 1 accepted-cache gate ignored | `59cd23c4a1993f30d48d34eb67d5b8c4427a618e93b4fbaf946aa8daba1f3819` |
| `cargo clippy -p agq-sysml-semantics -p agq-kerml-text --all-targets -- -D warnings` | 0 | `9d07f1e062512d6e9446a5b3ec0c08de29524740345a0cc7bfcec3aed2e91290` |
| `cargo fmt --all -- --check` | 0; empty output | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `git diff --check` | 0; empty output | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

The added accepted assertions are compiled, not passed. Systems acceptance and
the resulting closed authored/programmatic and edit runs remain required; this
review does not establish language readiness. Graph-sharing observations also
remain subject to the separate
[storage review](../../../docs/modeling-workspace-frontend-boundary.md#shared-dependency-storage-review):
record/Arc identity checks alone do not establish I8's no-standard-graph-copy gate.

The follow-up assertion review, based on `baae387`, now requires
`certificate.is_fully_closed(model)` for authored self-model revisions, the
independent programmatic graph, and the case-role project. Effective nested
usages, definition projections, composite children, interface ends, parameters,
entry/do/exit actions and requirement subjects must contain the existing
canonical authored IDs; two equally empty text/programmatic answers cannot pass
those checks. Each graph resolves its own IDs, preserving independent identity.
No additional semantic API or unsupported language behavior was introduced.

`cargo test --locked --offline -p agq-kerml-text --lib case_acceptance_fixture_uses_real_frontend_and_explicit_standard_redefinitions`
exited 0: one frontend test passed in 0.45 s (13.60 s including compilation),
output SHA-256 `fb3b5d32c1df5244595d969f4099746c2a77a53593c48ccf3513f15d01942bc3`.
The tested acceptance file SHA-256 was
`a7d11ee13604d8bdc1aefcf2fa3ecfd071c2b571df29a0eb4b0941c6e774e8af`.
Formatting and diff checks exited 0. The ignored accepted-library test was
compiled only; no cache was loaded and none of its new assertions has passed
against an accepted Systems publication yet.
