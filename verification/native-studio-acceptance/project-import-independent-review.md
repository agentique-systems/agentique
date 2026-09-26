# Independent project/import/recovery review

Reviewed the implementation from integration commit `a88c5d00` (locally cherry-picked as `179fac6b`). Production review focused on the durable boundary, not on whether a dialog opens.

## Durable behavior

`create_project_at` checks operator authority, name/location and outstanding candidate phase before touching the requested location. It opens the requested repository and calls the normal service project transaction before replacing the active platform. A rejected open or a failed durable project transaction leaves the previous platform available. `AddSourceDocument` uses the existing expected-head preparation path and does not request implicit validation. Validation and compare-and-swap commit retain the normal candidate lifecycle.

The added accepted-runtime integration test exercises invalid and unauthorized creation, a fresh empty Working project, retention of the old repository, exact CRLF import bytes, refusal to commit an unvalidated import, cancellation without a revision/head change, validated durable commit, duplicate-path refusal, stale-head refusal, and failed repository replacement. It uses the installed accepted publication and isolated temporary databases. It is explicitly ignored in the ordinary suite, because executing it performs real semantic work; it has not been run by this agent.

The generic-restart sample-injection bug found during this review is fixed separately in `5a472116`. A remaining UI gap was reported to the integration lead: using `projects.is_empty()` as the setup/authentication test also disables New Project in an authenticated empty explicit repository.

An ambiguous acknowledgement of a new-project transaction is not currently tested. `create_project_at` uses the service convenience transaction rather than retaining a prepared project operation for a native retry. This review did not reproduce an acknowledgement failure and does not claim one occurred.

## Source-only recovery scope

The `source_recovery` example checks two validated commits in a fresh process against their original semantic fingerprint, checkpoint identities, Graph/System/Requirements projections and Inspector DTOs. It explicitly requires `DurableSource` and `semantic_cache_used == false`. Its external acceptance driver must remove semantic cache blobs between the create and restore processes; merely running the two modes is not sufficient.

The authored example is deliberately small (a SysML part and a KerML class). It does not establish the real self-model's source-only recovery, requirement/verification behavior, or equivalence of every internal proof record. Those claims need their separate real acceptance evidence.

## Requirements/Diff honesty

Requirements grouping uses projected metaclasses and relationship kinds, never text labels as authority. The subject feature and its typed architecture remain different canonical identities joined by their original edges. Incoming package ownership is excluded from the obligation neighborhood. Architecture focus traverses that same bounded modeled neighborhood. No relationship is synthesized to shortcut requirement-to-architecture, and the surface states that modeled links do not establish satisfaction or verification success.

The four engineering roles are not themselves evidence that every role is usable on the real self-model. Satisfaction and verification still need representative real-product screenshots and interactions. The existing self-model's sparse requirement neighborhood cannot qualify a dense or varied requirements workflow by itself.

Diff groups and counts describe the two supplied, identically scoped projections. Exact before/after revisions and canonical targets are retained, including removed-object inspection in its original revision. The toolbar now calls these "projected changes"; it must not be reported as a count of all changed canonical semantic facts. Fields not represented in the projection can still change without appearing as a graphical object/relationship difference. Structure is the initial display mode; Relationships, Requirements and All preserve progressive access to the full projected pair.

Commands in the isolated worktree: `cargo fmt --all` and `git diff --check` exited 0 with no output. No compilation or accepted-runtime test was run here because the integration lead coordinates the shared targets and memory-heavy gates.
