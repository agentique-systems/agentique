# ADR 0026: language foundation stability contract

Status: proposed; adoption requires the recorded language-foundation readiness
gate below. This document does not accept the Systems Library or authorize
production workspace integration by itself.

## Context and scope

Agentique's canonical semantic graph is the foundation for modeling workspaces,
queries, validation and later execution. Platform consumers need stable meanings
for identities, revision boundaries and evidence. They do not need a frozen
index representation or complete KerML/SysML conformance.

The normative targets remain pinned KerML 1.0 and SysML 2.0 with explicitly
versioned operational interpretations. The accepted KerML Operational v9
publication remains accepted. This decision neither reopens it nor transfers
generation-2 progress to generation-1 release coverage. Generation 1 continues
under `standards/coverage.json` and `verification/traceability.json`.

This contract complements [ADR 0022](0022-kerml-publication-vs-conformance.md),
[ADR 0025](0025-producer-closure-evidence.md), and the
[runtime contract](../language-core-runtime-contract.md). Its stability promise
concerns observable semantics. Concrete API additions and implementation choices
remain subject to ordinary Rust API review.

## Stable external architectural contracts

| Contract | Meaning consumers may rely on |
| --- | --- |
| `ElementId` | Identifies a canonical semantic element independently of path, display name, syntax location or storage index. The same element retains identity across revisions where the frontend's explicit reconciliation permits. Replacing unreconciled source does not promise guessed identity. Retired IDs cannot be reused within a declared history. |
| `RevisionId` | Identifies immutable declared kernel input, not a derived result set, repository commit, source revision, timestamp or content hash. Transactions bind to the exact base snapshot; equal-looking reconstructed inputs are not interchangeable transaction bases. |
| Snapshot and revision boundary | Publication creates immutable coherent inputs. Failed construction or validation cannot modify the prior revision. Existing handles remain readable after later edits; a workspace's head is a separate reference. |
| Declared and derived separation | Declared origin includes authored/imported/generated declarations. Derived facts have rule identities, canonical dependencies and explanations in an overlay pinned to declared inputs. An overlay cannot silently replace a declared fact or be rebased onto changed declarations. Construction views and their obligations are explicitly unpublished. |
| Relationship identity | Relationships are canonical records with their own identities and participants; applicable association occurrences use their canonical occurrence identity and provenance. Navigation and inverse indexes project those facts. They must not create additional semantic relationships or alternate writable carriers. Ordered roles retain semantic order. |
| Source and provenance | Source bytes, source revisions, syntax identities, canonical element identities and artifact identities remain distinct. Half-open source ranges and declared/derived origins remain inspectable. Original authority/library bytes are preserved. Reformatting or cache transport cannot replace semantic authority. |
| `SemanticContext` identity | Identifies the exact graph/overlay and all interpretation inputs needed by its queries, including profiles, registry, rules, bindings, standard dependencies and closure contract when present. A declared revision alone is insufficient to identify effective results. Query caches cannot mix contexts. |
| `Complete / Incomplete / Invalid` | Complete establishes the named query's promised scope and applicable producer closure; it does not mean whole-language conformance or verification success. Incomplete retains unmet premises/obligations even when a provisional value exists. Invalid reports contradictory or invalid premises. Empty, absent, uncomputed and unsupported are not interchangeable. |
| Negative and search evidence | Absence requires a closed relevant population. Evidence names positive facts and bounded negative/provider searches, including missing identities and applicable producer opportunities. A currently empty search cannot prove absence while a relevant writer or provider remains open. |
| Producer closure certificate | Immutable sidecar evidence is issued through the scheduler's checked boundary. It binds exact graph, registry and interpretation context, requirement coverage and causal producer quiescence. Convergence alone is insufficient. Graph reconstruction requires checked revalidation of affected facts, searches and potential effects; neither blindly reusing a certificate nor globally discarding independent closure is the contract. |
| Semantic closure identity | Graph identity and the validity of a particular closure witness are distinct. An unrelated additive change may preserve a witness only after proving its subject, relevant producer opportunities and closed searches unchanged. Equal observed values cannot preserve a witness when a relevant potential writer changed. A rebound certificate must authenticate the new exact graph. |
| Accepted standard publications | Only an accepted language facade conveys acceptance. Strict snapshots, structural archives, parsed libraries, construction bindings and scoped closure results do not. Accepted identities bind source/content, profile/grammar, descriptor graph, rules, bindings, dependency publications, semantic graph and closure evidence under the relevant versioned receipt contract. Exact trusted restoration verifies these identities without replaying accepted producers. |
| Immutable accepted dependencies | Projects share accepted standard records and identities. Local declarations cannot rewrite accepted records or their composite ownership; the accepted publication's own semantic view remains fixed. Combined project queries can observe local noncomposite relationships that reference standard elements. Such source, inverse and project-wide searches require their own closure evidence. The immutable boundary discharges only requirements no local producer or pending local provider can change. |
| Effective feature/member semantics | Effective queries retain original element and Membership identities, visibility, specialization and redefinition suppression, order where defined, and evidence. Inheritance is lookup/projection over the canonical graph; inherited copies are forbidden. Current-graph projections cannot be relabeled as effective closure. |
| KerML query composition | KerML queries operate over the shared language-agnostic kernel view. Query composition retains completeness, invalidity, evidence and positive/negative search dependencies. Status-only queries and invalidation read sets are explicitly evidence-free and cannot substitute for proof or publication. |
| SysML query composition | SysML reuses KerML contracts over the same canonical graph and composes its authenticated semantic context. Definition/Usage and structural Attribute/Item/Part, Port/Connection/Interface, Occurrence/Action/State, Requirement/Constraint/Case and View/Metadata APIs retain those completeness and evidence contracts. Each accepted family requires permanent authored fixtures over the accepted Systems dependency. Structural support does not imply executable semantics. |

A negative owner predicate distinguishes an absent closed owner population, an
unresolved owner, and an owner/type that producers may still introduce. Explain
must expose why the relevant ownership and effective typing populations are
closed, which producer requirements discharge that conclusion, and any accepted
immutable dependency boundary used. Missing evidence remains Incomplete.

Language crates depend inward. `agq-kernel` is language-agnostic; KerML builds on
the kernel and SysML composes KerML. No `agq-kernel`, `agq-kerml*` or `agq-sysml*`
crate may depend on the Gen1 model/workspace/application stack. Platform views
borrow canonical records; syntax, database rows, DTOs and diagrams are not a
second canonical model. Transport and storage remain outside language contracts.

## Monolithic publication and resumable frontiers

The immediate Systems authority is the existing monolithic dependency-driven
scheduler, through its Structural, StableProperties and ContextualBindings strata.
Acceptance still requires convergence, Complete producer closure, a fully closed
certificate, the complete mandatory-reference audit, and all facade acceptance
checks. Construction success, a scoped audit, matching counts or a graph digest
alone cannot substitute for this boundary.

Incremental certificate maintenance may reuse unaffected topology and producer
scope calculations. The complete causal and requirement fixed points remain the
authority; the full certificate rebuild is the reference implementation. Exact
equivalence includes closure rows, requirements, negative/provider/search reads
and the semantic certificate digest. Removing an opportunity or changing an
applicable scope must remove obsolete rows.

A durable frontier checkpoint is an **unaccepted continuation of that same
computation**. Its externally retained journal pin authenticates the archive and
scheduler/certificate state. Resume also authenticates source/publication inputs,
the accepted KerML dependency, profiles, descriptors, producer registry, rules,
bindings, graph and certificate context. A mismatch rejects resume. Checkpoint
transport preserves ordered-reference positions and selected proof/search support;
observational timings and allocation layout do not convey authority. Trusted
accepted-publication cache restoration is a separate facade boundary and performs
no accepted producer replay. Legacy structural archives that omit optional
selected support cannot be treated as lossless accepted Systems caches.

[ADR 0027](0027-compositional-semantic-publication.md) remains proposed research.
Current provider/search footprints do not prove useful Systems strata; they do
not prove semantic indivisibility. Keep its planner, exact comparator, boundary
audits and invalid-partition tests. Neither this contract's adoption nor the
initial workspace requires compositional sealing or another Systems partition run.

## Implementation freedom and compatibility

Ordered maps may become arenas or persistent maps. Indexes, caches, proof/search
interning layout, producer queue representation, incremental validation,
parallelism, memoization and future on-disk encodings may evolve. Such a change
is implementation-only when it preserves identity, observable ordering,
provenance, results, completeness and evidence obligations under the same
contract. Pointer identity is not semantic identity; it may be observed in
sharing tests to detect accidental copying, never used as a publication digest.

A semantic breaking change changes an answer or a guarantee in the table under
the same advertised inputs. Examples include changing element reconciliation or
derived-ID rules, treating derived data as declared, changing relationship
identity, weakening Complete, copying inherited features, removing negative
search dependencies, accepting a stale certificate, or changing an accepted
standard dependency's identity in place.

Such changes require an ADR explaining affected consumers and migration, plus an
explicit profile/rule/contract version change for the changed interpretation.
Published historical profiles and accepted receipts remain reproducible. Update
context/receipt identities and permanent regression fixtures; do not reinterpret
old revisions or issue an old Complete marker with a weaker meaning. A cache
format change may use a cache-format version without a semantic profile change
when the reconstructed semantics and acceptance identities are unchanged.

Compatible additions may expose new queries or independently versioned support.
They must not make an unsupported operation silently succeed, expand an existing
Complete promise without proving its additional scope, or hide prior invalidity.
Fixes to erroneous semantic behavior require the same explicit impact review;
calling a change a bug fix does not exempt it from identity/version obligations.

## Adoption gate and bounded platform scope

The integration owner adopts this ADR only with a concise readiness record under
`verification/summaries/final-language-acceptance/` establishing all of:

- accepted immutable KerML and Systems canonical publications and accepted
  bindings, including complete mandatory reference audits;
- working authored KerML and SysML integration using those publications;
- sound producer closure transport, initial closure and negative conclusions;
- stable effective structural API families listed above, exercised by authored
  and programmatic equivalent fixtures;
- the permanent Agentique self-model parsing, lowering, closing and answering
  semantic architecture tests using accepted publications;
- stable identity/provenance contracts and the recorded Gen1/Gen2 dependency
  audit.

The [conditional adoption review](../../verification/summaries/final-language-acceptance/stability-adoption-review.md)
maps these contracts to the implemented monolithic path and names the remaining
measured gates. It is not the readiness decision. Earlier language-stability
bridge summaries remain historical evidence and are not rewritten as successes.

The record must distinguish measured acceptance from proposed behavior. Failed or
unrun gates keep this ADR proposed and [ADR 0024](0024-gen2-modeling-workspace.md)
production integration gated. Full conformance and unrelated validator coverage
are not additional gates. When these conditions hold, the recorded decision is
`AGENTIQUE LANGUAGE FOUNDATION STABLE — MODELING PLATFORM MAY PROCEED`.

The resulting authorization is the in-memory revisioned modeling workspace only.
Persistence, server/API migration, diagrams, transformations, execution IR,
simulation and assistant features remain later milestones. No new open-ended
language-conformance phase is introduced by this ADR.
