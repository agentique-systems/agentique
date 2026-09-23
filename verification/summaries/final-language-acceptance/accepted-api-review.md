# Authored Systems API and self-model acceptance review

This is a source coverage review, not accepted Systems publication evidence.
Reviewed integration source: `bf02cf8`, including the View/Metadata fixture added
on the integration branch. The additional assertions in this commit are held
until the accepted Systems gate can run. No accepted catalogue, publication
default, receipt, or ADR status is changed here.

## Authored API coverage

All semantic rows below belong to the explicitly requested ignored test
`accepted_agentique_self_model_closes_queries_edits_and_matches_programmatic_semantics`
in `crates/kerml-text/src/accepted_self_model_tests.rs`. It requires both accepted
standard caches, checks producer convergence and Complete references, and checks
a fully closed certificate before querying authored semantics.

| Required area | Permanent authored fixture | Effective API and assertions |
| --- | --- | --- |
| Definition / Usage | Five files in `models/agentique/` | `effective_supertypes`, `effective_usages`, `effective_usage_types`; accepted standard role ancestry; inherited canonical member identity and redefinition suppression |
| Attribute / Item / Part | `Contracts.sysml`, `ModelingPlatform.sysml` | `effective_attribute_definitions`, `effective_item_definitions`, `effective_part_definitions`, `effective_subitems`, `effective_subparts`; exact authored target identities |
| Port / Connection / Interface | `Contracts.sysml`, `ModelingPlatform.sysml` | `effective_ports`, `effective_port_definitions`, `effective_connection_ends`, `effective_connection_related_features`, `effective_interface_ends`; ordered authored endpoints and canonical reference-subsetting relationships |
| Occurrence / Action / State | `ModelingPlatform.sysml` | This commit explicitly adds `effective_usages_of_kind(Occurrence)` with authored workspace/action/state IDs and independent programmatic comparison; existing `effective_subactions`, `effective_parameters`, and `state_actions` verify action parameters plus entry/do/exit roles |
| Requirement / Constraint | `ModelingPlatform.sysml`, `tests/fixtures/agentique-cases.sysml` | `requirement_case_features`, `effective_usages_of_kind(Requirement)` and `effective_usages_of_kind(Constraint)`; canonical subject/objective/constraint identities |
| Case | `tests/fixtures/agentique-cases.sysml` | `effective_usages_of_kind(Case/VerificationCase)`, `requirement_case_features`, `effective_return_parameters`; subject/actor/objective/result exact identities and accepted Case/VerificationCase ancestry |
| View / Metadata | `tests/fixtures/agentique-view-metadata.sysml` on integration branch | `effective_supertypes`, `effective_usage_types`, `effective_usages`; authored View/Metadata definitions reach accepted standard roles, real prefix MetadataUsage remains canonical, inherited ViewUsage retains its original ID |

The View/Metadata APIs use generic Definition/Usage query contracts. The test
does not invent a second metadata DTO or claim an unimplemented specialized
view execution API. An ordinary frontend fixture test verifies actual View and
Metadata canonical metaclasses separately from accepted semantic closure.

The independent programmatic comparison covers the rich platform populations,
including the new Occurrence population. Case and View/Metadata use independent
permanent authored fixtures and direct accepted API assertions.

## Architecture and traceability

Existing accepted assertions check SemanticKernel isolation, KerMLEngine to
SemanticKernel, SysMLEngine to KerMLEngine, ViewService to QueryService,
validated compiler input, SimulationRuntime to ExecutionIR, and the workspace's
standard/query members. This commit adds explicit ProjectWorkspace dependency
targets, ExecutionCompiler to ValidationService, and canonical part composition
plus effective typing for Agentique, LanguageEngine, and ExecutionSubsystem.
The accepted gate still checks the same original five source files.

The independent ordinary test
`agentique_implementation_traceability_is_separate_and_resolves_existing_paths`
in `agentique_self_model_tests.rs` parses/lowers the model separately, resolves
each qualified manifest element, checks existing implementation paths, rejects
parent traversal, and requires planned Gen2 entries to have no implementation
paths. It does not generate Rust. Once the workspace implementation is accepted,
its mapping should move from planned Gen2 to its actual crate path.

## Activation items owned by integration

1. Keep the trusted Systems catalogue empty until the full acceptance gate passes.
   After adding the accepted Operational v2 catalogue entry, change the existing
   unit test which currently expects that exact identifier to be unavailable;
   retain rejection of unknown identifiers.
2. `SysmlBaselineProfile::OPERATIONAL` is absent at the reviewed source. Add its
   Operational v2 alias only after accepted publication; preserve explicit
   historical profiles.
3. Run `accepted_systems_cache_roundtrip_and_tampering` against the newly accepted
   receipt/cache. The existing test checks zero producer replay, shared accepted
   KerML storage, identities/bindings/context/certificate, rewritten cache exactness,
   changed manifest fields, changed ZIP entries, duplicates/extras, and truncation.
4. Run the accepted authored gate above before adopting ADR 0026 or reporting
   effective SysML core/self-model acceptance. The new assertion code has not
   been executed against an accepted Systems publication by this review worker.

## Verification for this review change

Command in the checkpoint worktree: `cargo fmt --all -- --check`.
Result: exit 0, no output. No Rust build or publication was run for this test-only
review change, to avoid competing with the integration release/medium/full gates.
