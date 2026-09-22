# ADR 0025: producer closure evidence for negative semantic conclusions

Status: implemented; corpus acceptance remains a separate gate.

## Decision

An unchanged graph is insufficient evidence that an effective semantic answer is
exhaustive. The scheduler owns a deterministic registry of stable producer-family
identities, applicability and declared potential effects. Registry identity is
part of the composed semantic query contract. Effects are declarations of what
a family can change, including branches that emitted nothing in one evaluation.

The scheduler records each subject/family evaluation as inapplicable, pending,
complete or incomplete. A complete result becomes closed only at a quiescent
frontier: its dependencies remain unchanged, dirty work has been discharged, and
its graph and interpretation contract still match. An incomplete evaluation
cannot certify the effects that family may change. Producers outside a scoped
population remain obligations; accepted immutable dependencies are not replayed.

Producer dependencies are transitive. For example, an unfinished scalar writer
can invalidate a currently complete typing producer that read that scalar. The
typing result remains open until that upstream writer closes. The certificate
uses per-family semantic reads and potential effects to retain this obligation;
checking only the typing producer's latest return status is insufficient.

`ProducerClosureCertificate` is an immutable sidecar with a private issuing
boundary. It binds model, registry and context-contract digests and stores compact
subject/family states and closed requirement masks. It is not a canonical model
Element and does not modify library source bytes or metamodel authority. Shared
certificate storage avoids expanding a producer proof tree on every query.

`SemanticClosureRequirement` explicitly maps effective typing, featuring,
membership, naming and value context to relevant effects. Typing includes typing,
specialization, subsetting, redefinition and feature-chain effects. Ownership and
semantic dependency propagation account for writes originating on other subjects.
Requirements without precise local dependency traversal conservatively retain
global blockers. Under-certification is preferable to an unsupported absence.

Negative queries record `SearchDependency::ProducerClosure`, including subject,
requirement and certificate digest. A missing witness is also an explicit search
dependency and produces Incomplete. Explain output exposes the proof boundary;
no synthetic FactKey represents it. A positive type or specialization witness can
establish a positive antecedent without proving that additional types are absent.

Canonical proof transport retains only the subject and versioned closure
requirement as a kernel structural search. This is a dependency to reopen, not a
witness. Query evaluation obtains the exact certificate from its current context.
Historical certificate identities stay outside canonical records and their graph
digest, so equivalent semantic graphs do not acquire worklist-history identities.

Certificate attachment checks the exact graph, registry and context contract.
Forks share valid evidence; changing interpretation inputs or the graph discards
it. New frontiers issue new evidence after dependency invalidation. Certificates
do not make unresolved source references or unsupported producer semantics valid.

## Compatibility and publication

The optional closure contract has its own versioned identity. The accepted KerML
Operational v9 receipt and historical graph digest remain pinned; no KerML v10
profile is created. Historical trusted restoration remains distinct from issuing
new closure evidence. Published/v1/v2 SysML authority remains explicitly selected.

Systems construction retains its certificate beside its unpublished overlay.
Strict publication independently checks final producer closure, mandatory
references, capabilities, provenance and bindings. The accepted Systems identity
must include registry and certificate digests. A scoped Actions preflight cannot
be promoted into the full 21-document publication.

## Verification

The regression matrix includes delayed A-to-B activation, incomplete and
inapplicable families, changed registries, changed graph frontiers, public
certificate attachment, negative-query explanations, queue-order/batch
determinism and a 60,000-subject scale fixture. Actual command outcomes and
remaining publication gates belong in
`verification/summaries/producer-closure-certificate/` under ADR 0021.
