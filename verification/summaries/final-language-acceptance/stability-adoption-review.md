# Conditional ADR 0026 adoption review

**Status: prepared; not a readiness decision.** Reviewed against source
`694722d1569664dc0206cbb1ba4b5590cb9725f6`. ADR 0026 and ADR 0027 remain
proposed. This documentation change runs no publication, changes no acceptance
catalogue or coverage result, and does not authorize production workspace
integration. Apply the adoption edits below only after the measured H-K gates.

## Architectural review

The monolithic implementation is compatible with
[ADR 0026](../../../docs/adr/0026-language-foundation-stability-contract.md).
Its promise is stable observable semantics, not a frozen scheduler, allocation
layout or complete language conformance. The additions to the ADR make the
actual immediate authority and its transport boundaries explicit.

| Stable contract | Implementation boundary and remaining evidence |
| --- | --- |
| Semantic identity and immutable revisions | Kernel identities, immutable snapshots/construction views and exact semantic contexts retain their separate meanings. Held declared-history and shared-storage work must preserve them; allocation tokens used by sharing tests are not semantic IDs. |
| Declared/derived separation, relationships and provenance | Kernel derivations retain declared inputs, relationship identities, semantic order and canonical proof/search support. A construction view or restored frontier is never an accepted strict publication. |
| Complete/Incomplete/Invalid and negative evidence | KerML/SysML queries compose evidence and bounded searches. A negative answer still requires the relevant potential writers and provider population to be closed; a matching value is insufficient. |
| Producer closure certificate | `producer_closure_incremental.rs` reuses unaffected graph topology and producer-scope calculations. It still recomputes the global causal/requirement fixed points. The full rebuild remains the exact reference, including obsolete-row removal, negative/provider reads and the semantic digest. This is not a claim of fully incremental global certificate propagation. |
| Frontier continuation | `publication_frontier.rs` and the worklist restore the same monolithic computation from an externally pinned journal and authenticated graph/state. Structural, StableProperties and ContextualBindings remain existing scheduler strata. Exact uninterrupted/resumed medium equivalence is a measured prerequisite, not implied by the presence of serialization code. |
| Accepted standards and trusted restoration | Only the accepted language facade conveys publication authority. Lossless dependent evidence transport preserves selected ordered-reference positions and proof/search support. It is separate from legacy structural archives and frontier state; an archive alone is not a receipt. Actual accepted Systems restoration remains gated. |
| Effective query semantics | Permanent authored fixtures must run with accepted Systems and KerML, retaining completeness, evidence and original inherited identities. Synthetic or canonical-construction preflight tests cannot substitute. |
| Inward dependencies and platform boundary | The kernel remains the substrate, KerML builds on it, and SysML composes KerML; the platform depends inward. No Gen1 semantic storage, application migration or executable language semantics follows from this contract. |

The [lossless cache review](systems-cache-evidence.md) describes the implemented
Systems transport format and its focused checks. The
[measured tail profile](medium-tail-profile.md) is partial performance evidence,
not acceptance. The current [milestone policy](README.md) retains exactly one
new full release attempt after the medium equivalence gate. Neither historical
wall-time exhaustion nor a near-complete frontier is a semantic rejection or a
publication success.

[ADR 0027](../../../docs/adr/0027-compositional-semantic-publication.md) is
unchanged: proposed research, not adopted, and not current publication authority.
Current read-side footprints are insufficient to prove useful Systems strata;
they do not establish semantic indivisibility. Keep its component planner,
exact comparator, boundary audits and invalid-partition tests. Do not add a
source-partition or component-sealing prerequisite to this readiness decision.

## Required readiness record

Create a new `readiness.md` in this directory after the actual gates. Do not
overwrite the historical language-stability-bridge readiness result. Each row
must link the actual report/command and pin the tested source plus relevant
content identities. The following are requirements, **not observed results**:

| Gate | Required result |
| --- | --- |
| F: medium prerequisite | Both uninterrupted and authenticated-resume paths: 695/695 Complete references, zero kernel obligations, converged Complete producer closure and fully closed certificates. Exact graph, selected proofs/searches, certificate, effective queries and semantic digest agree with the reference medium semantics. Record invocation resets and strata separately from global log-row numbers. |
| G: full monolithic publication | 21/21 parse and 21/21 construction; zero kernel obligations; converged Complete scheduler; fully closed certificate; 1,327/1,327 mandatory references Complete; unresolved/incomplete/ambiguous/invalid/mismatch all zero; authority conflicts and capability findings zero; identity/provenance clean. |
| H: accepted publication and restoration | Issue CanonicalSysmlSystemsLibrary, StandardSysmlBindings, the trusted receipt and cache from that exact accepted result. Trusted restoration authenticates Systems KPAR/source identities, grammar/semantic profiles, descriptors, producer registry/rules, accepted KerML, bindings, graph and closure certificate without producer replay. Record exact identities and actual cache/tamper checks. |
| I: ADR 0026 review | Apply the conditional status edit only with this measured readiness evidence. ADR 0027 remains proposed. Full conformance and Gen1 release obligations remain separately incomplete/unchanged. |
| J: five-file Agentique model | Real SysML frontend plus accepted KerML/Systems: parse, references, combined producer closure and architecture assertions Complete. Check SemanticKernel, KerMLEngine, SysMLEngine, ModelingPlatform and ExecutionSubsystem and their intended dependencies. Independently validate the implementation mapping; generate no Rust from the model. |
| K: effective SysML core | Permanent authored accepted-dependency fixtures cover Definition/Usage; Attribute/Item/Part; Port/Connection/Interface; Occurrence/Action/State; Requirement/Constraint/Case; View/Metadata. Preserve exact authored/programmatic comparisons and query evidence. |

The recorded medium equality must include selected contribution evidence: equal
aggregate graph/certificate hashes can conceal a lossy archive's omitted local
proof/search payloads. Timing, pointer layout and diagnostic allocation are
observations rather than semantic acceptance fields. A checkpoint state mismatch
or semantic defect must reject/invalidate resume rather than silently restarting
with a different witness.

At this reviewed source the trusted Systems catalogue is empty. The SysML profile
defines explicit `OPERATIONAL_V1` and `OPERATIONAL_V2`; it has no `OPERATIONAL`
alias to advance. H should add `OPERATIONAL = OPERATIONAL_V2` once accepted,
preserve explicit historical profiles, and review intended call sites. Do not
silently change the enum's Published default as a substitute for that named alias.

The ignored accepted-cache test is
`accepted_systems_cache_roundtrip_and_tampering`. The ignored accepted authored
fixture is
`accepted_agentique_self_model_closes_queries_edits_and_matches_programmatic_semantics`.
Both require the compiled `sysml-systems-operational-v2` receipt plus
`AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`. Building them or passing
their small syntax/construction preflights does not execute their accepted gates.
The separate traceability check is
`agentique_implementation_traceability_is_separate_and_resolves_existing_paths`.

The held acceptance-fixture commit `639736a` adds explicit root Agentique and
LanguageEngine/ExecutionSubsystem composition, ProjectWorkspace dependency and
ExecutionCompiler -> ValidationService assertions, plus effective Occurrence
coverage. Source review confirms these address the gaps in the reviewed main
fixture. Integrate and execute that fixture against accepted dependencies before
claiming J/K. Held workspace dogfooding checks can reuse these cases but cannot
substitute for J while workspace integration remains gated.

## Conditional activation edits

Only after H-K succeed, replace ADR 0026's status paragraph with:

```text
Status: adopted for the generation-2 language foundation. The measured readiness
decision is recorded in verification/summaries/final-language-acceptance/readiness.md.
Full language conformance remains separately incomplete. ADR 0027 remains
proposed research and is not current publication authority.
```

In the same evidence-backed integration, update the architecture's current
"No Systems publication is accepted" and "ADR 0026 remains proposed" paragraphs
to name the actual accepted Systems profile, receipt and readiness record. Retain
historical bridge/convergence narratives with explicit historical labels. Record
the required foundation decision exactly:

```text
AGENTIQUE LANGUAGE FOUNDATION STABLE — MODELING PLATFORM MAY PROCEED
```

If any gate is failed, incomplete or unrun, retain the current proposed status
and record `AGENTIQUE LANGUAGE FOUNDATION NOT YET STABLE`. This conditional text
does not establish either a passing result or another general foundation phase.

Review these JSON locations in `standards/v2-coverage.json` during activation:

| Location | Conditional update and constraint |
| --- | --- |
| `/claim` | Describe measured canonical Systems acceptance and language readiness, keeping full conformance/application migration/execution separate. Its current medium-Incomplete sentence conflicts with the nested Complete medium result; reconcile it to actual evidence rather than copying old prose. |
| `/modeling_platform_gate` | Replace gated/reason/evidence using the new readiness record and actual completed stages. Foundation readiness permits M to start; it does not prove M's workspace implementation, 100 documents or five revisions passed. |
| `/sysml_systems_library_publication/status` and `/evidence` | Name the actual full accepted publication and concise report, preserving previous failures as historical records. |
| `/sysml_systems_library_publication/latest_completed_scope` | Keep medium success explicitly scoped; record full success separately with exact mandatory-reference and closure metrics. |
| `/sysml_systems_library_publication/full_publication_this_bridge` | Preserve the prior bridge's exhausted two-attempt policy as historical. It must not describe this milestone's separately authorized single 60-minute release attempt. |
| `/sysml_systems_library_publication/systems_publication`, `/accepted_bindings`, `/trusted_receipt_catalogue` | Mark accepted/available only after actual issuance; include identities or evidence links instead of invented digests. |
| `/sysml_systems_library_publication/effective_authored_acceptance` | Replace held only after J/K actually run against the accepted dependency. |
| `/compositional_semantic_publication` | Preserve incomplete research status, absent sealed strata/composite certificate/component checkpoints and the outstanding proof obligation. Qualify its embedded Systems/workspace fields so they do not incorrectly make monolithic readiness depend on adopting ADR 0027. |
| `/kerml_canonical_publication`, `/legacy_release_register` | Keep accepted KerML Operational v9 and generation-1 obligations unchanged. Do not reopen or regenerate KerML. |

After M is separately integrated and checked, add current implementation notes
to the phase-1 design and frontend-boundary storage audit. The older audit's
map/index-copy findings are historical; retain their source baseline and add the
new observed sharing result rather than erasing them. Link actual immutable
revision, edit/query, accepted-standard storage, self-model and scale evidence.
Update ADR 0024's implementation status only to the scope those results support.
The semantic-kernel cost-model paragraph and the generation-boundary audit also
need a dated current-implementation note after shared storage is integrated.

The [workspace recomputation roadmap](../../../docs/modeling-workspace-phase1-design.md#future-semantic-recomputation-after-an-authored-edit)
now records document edit -> query/provider invalidation -> affected producer
populations -> closure revalidation -> immutable revision. It reuses ProducerEffect
and the retained compositional research, with conservative fallback for unproved
footprints. It does not depend on component sealing. Persistence, server/API
migration, Systems Modeling API, diagrams, transformations, execution IR,
simulation and assistant migration remain excluded.

## Verification of this documentation change

Only documentation and evidence-record consistency checks are appropriate here.
The accompanying command summary records the actual diff/relative-link/JSON and
conditional-status checks. No Rust build, corpus execution, accepted-cache gate
or workspace test was run for this documentation change. The lead retains the
milestone's required final checks and must attach their actual results separately.
