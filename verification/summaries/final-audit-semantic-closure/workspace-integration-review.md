# Held workspace review for the post-stability gate

This is a read-only source review, not workspace acceptance. No workspace was
opened, no accepted Systems publication was fabricated, and no revision was
reported as Validated. Integration remains after the Systems publication,
self-model and effective core gates required by ADR 0026.

## Exact reviewed source and integration order

The reviewed branch is `platform/workspace-integration`, ending at
`1ee7be295f43266c7f480b78865f3fd60a0874e9`.

1. `427d29b2633d5ac1183e0338df96d4ef4300552e` adds the crate, shared kernel
   storage and declared history, SourceInputs/SourceCompilation, and acceptance
   fixtures. It is the complete held implementation patch, not just a facade.
2. `1ee7be295f43266c7f480b78865f3fd60a0874e9` adds explicit project identity,
   checks the SysML query context during validation, strengthens self-model
   revision assertions, and replaces verbose historical preparation records.

Do not replace current source files with the entire old branch tree: that branch
predates the strict direct finalizer and subsequent semantic closure corrections.
Apply these two reviewed commits in order and reconcile their local changes.

Read-only `git apply --check --ignore-space-change -` for the first commit
against the lead worktree at `cc0f99381469d3ab6e1b9926ca7cd57b69d3a958` returned
exit 0. The second checked alone returned exit 1 because files introduced by the
first commit are absent; this does not establish a sequential application
conflict. Neither patch was applied by this review. Later lead changes can alter
the application result.

## Required reconciliation before activation

- `AcceptedSourceDependency::syntax_profile` introduced in
  `crates/kerml-text/src/sysml/source.rs` matches only Published/v1/v2. Add v3
  according to the accepted publication's exact grammar/profile mapping; do not
  silently downgrade its semantic context to v2.
- `crates/modeling-workspace/tests/support/mod.rs` explicitly looks up
  `sysml-systems-operational-v2`. Select the actually accepted v3 receipt after
  publication. Keep both caches mandatory and retain trusted restoration.
- Preserve current finalizer/cache/receipt changes during source integration.
  The shared kernel archive refactor must restore the already issued artifact
  with identical semantic identity; it is not permission to replay standards.
- The held authored capability loop in `SourceInputs::compile` checks only
  `effective_usages` on local Definition/Usage records. That operation does not
  exercise all typed projections, interface ends or return/end cycles. This is
  a static coverage gap relative to this milestone's final-audit lessons: before
  broadening the Validated contract, reuse the strict applicable effective-query
  suite for local authored subjects and add a malformed typed-target rejection
  fixture. A fully closed producer certificate alone cannot establish that
  every promised effective operation is Complete.
- Replace historical preparation status with actual new command results only
  after accepted-cache execution. Preserve historical failed or held outcomes.

The follow-up read-only review identifies a direct reuse boundary: expose an
internal `agq-kerml-text` wrapper over `sysml/publication.rs::audit_sysml_population`
and call it from `SourceInputs::compile` on the revision's own queries and all
local canonical subjects, including derived local records. Accepted dependency
records stay excluded. Retain the report and its exact `SysmlSemanticContextId`
on immutable `SourceCompilation`, map findings to blocking source diagnostics,
and require a current, finding-free report during workspace validation. This is
authored-model validation; it must not invoke Systems publication, exact corpus
counts, binding issuance or artifact transactions.

A targeted negative fixture can declare `part def NotADataType;` followed by
`attribute broken : NotADataType;`. Require preserved canonical FeatureTyping
and resolved references, an Invalid effective attribute-definition query with
the incompatible target retained in its diagnostics, and failed validation.
If the real frontend closes its producer certificate, assert that explicitly
to establish that closure alone is insufficient. If an earlier gate rejects it,
record that fact rather than claiming it isolates the new audit. Repairing the
definition to `attribute def` should validate a new immutable revision without
changing the rejected revision or shared standard storage. This fixture is
prepared guidance, not an executed gate.

## Architectural assessment

| Principle | Reviewed implementation |
| --- | --- |
| Parser separate from canonical model | SourceInputs holds source/syntax; SourceCompilation exposes a canonical strict or construction frontier. No syntax DTO becomes the model. |
| Generic kernel, relationships first-class | SharedMap and DeclaredConstructionHistory use kernel identities and provenance; they introduce no SysML metaclasses into the kernel. Associations retain canonical occurrence identity. |
| Declared versus derived | The declared history is reconciled before semantic construction. Derivation remains pinned to the exact declared frontier. |
| Immutable revisions | Workspace apply completes input preparation and compilation before updating head/history. Retained Arc revisions borrow their own graphs. Failed operational edits leave the head unchanged. |
| Source versus semantic identity | Project, workspace revision, kernel revision, document, syntax and element identities remain distinct. Temporary omission differs from explicit deletion; retired IDs cannot be resurrected. |
| Explainability and negative closure | Query factories retain their revision's context, certificate and pending provider information. Recovery cannot fall back to an earlier complete graph. |
| Inheritance without copies | Effective queries return existing canonical IDs. Fixtures assert inherited IDs and preserved original records after redefinition. |
| Shared immutable publications | Local tables borrow accepted base tables. Tests observe actual canonical/index/proof/search storage and evaluated producer subjects, beyond facade Arc equality. |
| SysML composes KerML | The workspace depends inward on frontend, semantic and kernel crates. Both query factories borrow the same model. |
| Separate execution | Workspace validation is structural platform acceptance. No execution evaluator, simulation verdict or standard conformance claim is introduced. |

No architectural inversion is evident in this bounded review. The effective
validation coverage gap above must be resolved or explicitly scoped before a
stronger Validated promise is made. The large shared-storage refactor still needs
its kernel invariants, archive identity and accepted-cache tests on the new tree.

## Exact dogfooding and scale gates to activate after stability

With accepted `AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`:

```text
cargo test -p agq-kerml-text --lib accepted_agentique_self_model_closes_queries_edits_and_matches_programmatic_semantics -- --ignored --nocapture
cargo test -p agq-modeling-workspace --features verification --test self_model -- --ignored --nocapture
cargo test -p agq-modeling-workspace --features verification --test phase1 -- --ignored --nocapture
cargo test -p agq-modeling-workspace --features verification --test working_states -- --ignored --nocapture
```

The self-model fixture opens all five real documents, validates r1, adds a part
to ModelingPlatform, validates r2, and checks old query populations, unchanged
authored IDs, architectural dependencies and shared standard storage. It models
Agentique's conceptual contracts without forcing Rust types to mirror metaclasses.

The held scale fixture creates 100 mixed documents and five retained revisions
with four parallel readers. Its fourth revision deliberately removes a provider,
has 99 documents and remains Working; the fifth repairs to 100 documents and
Validated. Report that exact shape rather than claiming five valid 100-document
revisions. Add a five-valid-revision variant if that stronger scale condition is
adopted. Strict Rustdoc, workspace tests and generation-dependency checks remain
required after integration.
