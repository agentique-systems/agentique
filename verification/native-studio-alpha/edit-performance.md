# Candidate reconstruction diagnosis

No end-to-end semantic latency improvement is claimed by this workstream. The
accepted runtime is being rematerialized independently; the new real-model gate
has not yet run. No semantic producer, kernel index, publication receipt,
authority decision or frozen publication input was changed.

## What the existing measurements establish

The prior accepted-cache oracle at commit
`068fc4412452f5faf9f8fee6fdba0dfb1328d951` measured add-part at **167,012 ms
incremental versus 95,220 ms full authored reconstruction**. Both paths evaluated
141 final-closure producer subjects and audited 116 effective subjects. The
incremental path lowered one document (41 records); the full path traversed six
document passes (98 records). The full seven-edit process peaked at 5,078,302,720
bytes of Windows peak working set. That is whole-process memory, including
accepted-publication restoration and all edits/oracles, not per-edit allocation.

See [the original oracle](../summaries/modeling-platform-phase2/frontend-acceptance.json).
These are historical observations, not rerun alpha results. They demonstrate
that reduced lowering work did not establish reduced wall time.

The later 401-to-9 result used a **100-document rename construction fixture**,
not CreatePartUsage with accepted semantic closure. Its 24.721 ms versus
161.748 ms comparison excluded producer closure, effective audit and persistence.
See [its explicit measurement boundary](../summaries/agentique-studio-phase3/performance.md).

There is no retained current accepted-cache phase profile from which to name the
dominant phase honestly. Current `CompilationTimings` already separates reference
refinement, declared construction, preparatory producers, strict kernel validation,
final closure/certification, final references and effective audit. Nested timings
overlap source preparation and must not be added indiscriminately.

## Why speculative reuse was rejected

- Lowering already reuses unchanged documents; declared reconstruction already
  submits changed/new/removed records against the strict predecessor.
- Kernel validation and local index reconstruction still process the combined
  candidate. Skipping these needs invariant-specific dependency boundaries.
- Producer checkpoint rebinding authenticates actual reads, including negative
  searches. The historical equal final evaluation counts make its overhead worth
  measuring, but do not prove that rebinding is the dominant current phase.
- Effective audit stores results and subjects, not complete reusable successful
  query footprints or future-writer exclusion. An unchanged subject record cannot
  authorize reusing its effective answer after another element changes.
- `source_inputs.rs`, `sysml/source.rs`, construction/refinement and the semantic
  engine are recorded in `standards/sysml-publication-inputs.json`. Changing their
  authority/freshness input bytes during accepted-cache recovery would require
  separate contract handling; this workstream does not alter those files.

## Focused real command gate

`agq-modeling-agent/tests/create_part_performance.rs` exercises the actual
`ModelCommand::CreatePartUsage` source mapping, ModelingService, durable repository
and current `models/agentique/*.sysml`. It requires a Validated baseline, constructs
a Working candidate under ModelingPlatform, confirms the durable head and old
semantic fingerprint remain unchanged, and compares against `full_rebuild` over
the exact same parsed source identities. Both results must validate.

The comparison checks canonical records, IDs, provenance, relationship order,
semantic fingerprint, closure semantic digest, reference values/completeness and
positive/negative/proof evidence, diagnostics, and the created part's effective
query evidence. It does not substitute an aggregate count for equivalence.

After runtime authentication, supply the exact installed cache paths through
`AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`, then record:

```text
cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-agent --features verification --test create_part_performance create_part_command_matches_full_self_model_reconstruction -- --exact --ignored --nocapture --test-threads=1
```

The `CREATE_PART_PERFORMANCE` JSON line separates total command preparation,
incremental/full compile phases and validation. Work retained/eliminated remains
alongside wall time. Runtime restoration is separate. Peak memory is explicitly
null unless an external process monitor measures it. Compare compile totals with
compile totals: full reconstruction excludes parsing and command/service overhead.
The gate makes no commit, restart or complete operator-journey claim.

Focused compile/test execution is delegated to the integration lead to avoid
competing builds against the shared target. Local `rustfmt --edition 2024
crates/modeling-agent/tests/create_part_performance.rs` completed successfully;
integrated check results must be recorded before this gate is called verified.

## RenameElement feasibility

The parser's `production::Document::edit` deliberately assigns fresh identities
to nodes overlapping an edit; `authored_id` derives the canonical ElementId from
SyntaxNodeId and an output role. Replacing a declaration's name therefore retires
its identity and may also replace overlapping ancestor declarations. An arbitrary
text replacement is not a stable-identity RenameElement command.

A sound command needs explicit declaration identity reconciliation bound to the
old exact source revision, collision and scope checks, reference-use updates
covering qualified names/imports/aliases and shadowing, and exact full-rebuild
comparison under the authorized identity mapping. Current frontend behavior
provides neither a reviewed rename mapping nor that proof. Rename remains
unsupported; no command is enabled by this analysis.
